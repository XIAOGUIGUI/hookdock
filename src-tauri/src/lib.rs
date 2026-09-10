mod file_replace;
mod hook_installer;
mod hook_server;
mod storage;
mod terminal_focus;

use chrono::Utc;
use hook_installer::{HookInstallState, HookInstaller};
#[cfg(target_os = "windows")]
use hookdock_protocol::TerminalTarget;
use hookdock_protocol::{
    normalize_hook, BridgeRequest, BridgeResponse, Decision, HookEvent, PROTOCOL_VERSION,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU16, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::path::BaseDirectory;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, State, WebviewWindow, WindowEvent};
use tauri_plugin_autostart::ManagerExt as AutostartManagerExt;
use tauri_plugin_clipboard_manager::ClipboardExt;
#[cfg(not(target_os = "windows"))]
use tauri_plugin_notification::NotificationExt;
use tauri_plugin_opener::OpenerExt;

use storage::{
    AppSettings, AppSettingsPatch, DockEdge, SavedPlacement, SourceProfile, SourceProfilePatch,
    Storage,
};

const APP_DATA_DIRECTORY: &str = "HookDock";
const EXPANDED_WIDTH: f64 = 520.0;
const EXPANDED_HEIGHT: f64 = 440.0;
const COMPACT_WIDTH: f64 = 196.0;
const COMPACT_HEIGHT: f64 = 52.0;
const EDGE_MARGIN: i32 = 12;

pub struct RuntimeState {
    storage: Mutex<Storage>,
    installer: HookInstaller,
    pending: Mutex<HashMap<String, Sender<BridgeResponse>>>,
    selected_event_id: Mutex<Option<String>>,
    pub(crate) runtime_path: PathBuf,
    pub(crate) listening: AtomicBool,
    pub(crate) port: AtomicU16,
    pub(crate) shutting_down: AtomicBool,
    compact: AtomicBool,
}

impl RuntimeState {
    fn new(data_directory: PathBuf, installer: HookInstaller, compact: bool) -> Self {
        Self {
            runtime_path: data_directory.join("runtime.json"),
            storage: Mutex::new(Storage::load(data_directory)),
            installer,
            pending: Mutex::new(HashMap::new()),
            selected_event_id: Mutex::new(None),
            listening: AtomicBool::new(false),
            port: AtomicU16::new(0),
            shutting_down: AtomicBool::new(false),
            compact: AtomicBool::new(compact),
        }
    }

    pub(crate) fn register_pending(&self, id: String) -> Receiver<BridgeResponse> {
        let (sender, receiver) = mpsc::channel();
        self.pending
            .lock()
            .expect("pending lock")
            .insert(id, sender);
        receiver
    }

    pub(crate) fn remove_pending(&self, id: &str) {
        self.pending.lock().expect("pending lock").remove(id);
    }

    fn pending_ids(&self) -> Vec<String> {
        self.pending
            .lock()
            .expect("pending lock")
            .keys()
            .cloned()
            .collect()
    }

    pub(crate) fn ingest(&self, app: &AppHandle, event: HookEvent) {
        let (event, settings, profile) = {
            let mut storage = self.storage.lock().expect("storage lock");
            let stored_event = match storage.add_event(event.clone()) {
                Ok(event) => event,
                Err(error) => {
                    eprintln!("Failed to persist HookDock event: {error}");
                    event
                }
            };
            let settings = storage.settings();
            let profile = storage.source_profile(&stored_event.source);
            (stored_event, settings, profile)
        };
        *self.selected_event_id.lock().expect("selected event lock") = Some(event.id.clone());

        if profile.enabled
            && profile.show_toast
            && settings.notifications_enabled
            && event.should_notify
        {
            show_hook_notification(app, &event, !settings.play_sound || !profile.play_sound);
        }
        if profile.enabled
            && profile.auto_expand
            && settings.show_panel_on_attention
            && event.expects_response
        {
            self.compact.store(false, Ordering::Relaxed);
            if let Some(window) = app.get_webview_window("main") {
                apply_window_mode(&window, false);
                position_window(&window, self);
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        self.emit_snapshot(app);
    }

    fn snapshot(&self) -> AppSnapshot {
        let storage = self.storage.lock().expect("storage lock");
        AppSnapshot {
            listening: self.listening.load(Ordering::Relaxed),
            port: match self.port.load(Ordering::Relaxed) {
                0 => None,
                port => Some(port),
            },
            events: storage.events(),
            pending_event_ids: self.pending_ids(),
            selected_event_id: self
                .selected_event_id
                .lock()
                .expect("selected event lock")
                .clone(),
            settings: storage.settings(),
            source_profiles: storage.source_profiles(),
            hook_status: self.installer.status(),
            generic_command: self.installer.generic_command(),
            platform: std::env::consts::OS.to_owned(),
            version: env!("CARGO_PKG_VERSION").to_owned(),
            compact: self.compact.load(Ordering::Relaxed),
        }
    }

    pub(crate) fn emit_snapshot(&self, app: &AppHandle) {
        let _ = app.emit("app-state", self.snapshot());
    }
}

#[cfg(target_os = "windows")]
fn activate_notification(
    app: &AppHandle,
    event_id: String,
    terminal_target: Option<TerminalTarget>,
) {
    let Some(state) = app.try_state::<Arc<RuntimeState>>() else {
        return;
    };
    *state.selected_event_id.lock().expect("selected event lock") = Some(event_id);

    if terminal_target
        .as_ref()
        .is_some_and(|target| terminal_focus::focus_terminal(target).is_ok())
    {
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.hide();
        }
        state.emit_snapshot(app);
    } else {
        reveal_window(app, &state, true);
    }
}

#[cfg(target_os = "windows")]
fn show_hook_notification(app: &AppHandle, event: &HookEvent, silent: bool) {
    use notify_rust::{Notification, NotificationResponse};

    let mut notification = Notification::new();
    notification.summary(&event.title).body(&event.body);
    if silent {
        notification.sound_name("Silent");
    }

    if std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_owned))
        .is_some_and(|directory| {
            let directory = directory.to_string_lossy().to_ascii_lowercase();
            !directory.ends_with("\\target\\debug") && !directory.ends_with("\\target\\release")
        })
    {
        notification.app_id(&app.config().identifier);
    }

    let Ok(handle) = notification.show() else {
        return;
    };
    let app = app.clone();
    let event_id = event.id.clone();
    let terminal_target = event.terminal_target.clone();
    std::thread::spawn(move || {
        let _ = handle.wait_for_response(move |response: &NotificationResponse| {
            if matches!(
                response,
                NotificationResponse::Default | NotificationResponse::Action(_)
            ) {
                activate_notification(&app, event_id.clone(), terminal_target.clone());
            }
        });
    });
}

#[cfg(not(target_os = "windows"))]
fn show_hook_notification(app: &AppHandle, event: &HookEvent, silent: bool) {
    let mut notification = app
        .notification()
        .builder()
        .title(&event.title)
        .body(&event.body);
    if silent {
        notification = notification.sound("Silent");
    }
    let _ = notification.show();
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppSnapshot {
    listening: bool,
    port: Option<u16>,
    events: Vec<HookEvent>,
    pending_event_ids: Vec<String>,
    selected_event_id: Option<String>,
    settings: AppSettings,
    source_profiles: Vec<SourceProfile>,
    hook_status: BTreeMap<String, HookInstallState>,
    generic_command: String,
    platform: String,
    version: String,
    compact: bool,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HookReply {
    decision: Decision,
    #[serde(default)]
    reason: Option<String>,
    #[serde(default)]
    answers: Option<HashMap<String, String>>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct OperationResult {
    ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

impl OperationResult {
    fn ok() -> Self {
        Self {
            ok: true,
            error: None,
        }
    }

    fn error(error: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(error.into()),
        }
    }
}

#[tauri::command]
fn get_state(state: State<'_, Arc<RuntimeState>>) -> AppSnapshot {
    state.snapshot()
}

#[tauri::command]
fn respond_to_hook(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    id: String,
    response: HookReply,
) -> OperationResult {
    let sender = state.pending.lock().expect("pending lock").remove(&id);
    let Some(sender) = sender else {
        return OperationResult::error("Hook 已断开或已经处理");
    };
    let bridge_response = BridgeResponse {
        decision: Some(response.decision),
        reason: response.reason,
        answers: response.answers,
        ..BridgeResponse::acknowledged(id.clone())
    };
    if sender.send(bridge_response).is_err() {
        state.emit_snapshot(&app);
        return OperationResult::error("Hook 连接已经关闭");
    }
    if let Err(error) = state
        .storage
        .lock()
        .expect("storage lock")
        .resolve_event(&id, response.decision)
    {
        return OperationResult::error(error);
    }
    state.emit_snapshot(&app);
    OperationResult::ok()
}

#[tauri::command]
fn dismiss_event(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    id: String,
) -> OperationResult {
    if state
        .pending
        .lock()
        .expect("pending lock")
        .contains_key(&id)
    {
        return OperationResult::error("等待响应的 Hook 不能移除");
    }
    match state
        .storage
        .lock()
        .expect("storage lock")
        .dismiss_event(&id)
    {
        Ok(()) => {
            state.emit_snapshot(&app);
            OperationResult::ok()
        }
        Err(error) => OperationResult::error(error),
    }
}

#[tauri::command]
fn focus_event_terminal(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    id: String,
) -> OperationResult {
    let target = state
        .storage
        .lock()
        .expect("storage lock")
        .events()
        .into_iter()
        .find(|event| event.id == id)
        .and_then(|event| event.terminal_target);
    match target {
        Some(target) => match terminal_focus::focus_terminal(&target) {
            Ok(()) => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.hide();
                }
                OperationResult::ok()
            }
            Err(error) => OperationResult::error(error),
        },
        None => OperationResult::error("该通知没有可用的 HookDock Terminal Pane"),
    }
}

#[tauri::command]
fn update_settings(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    patch: AppSettingsPatch,
) -> Result<AppSettings, String> {
    let settings = state
        .storage
        .lock()
        .expect("storage lock")
        .update_settings(patch)?;
    let autostart = app.autolaunch();
    if settings.open_at_login {
        autostart.enable().map_err(|error| error.to_string())?;
    } else {
        autostart.disable().map_err(|error| error.to_string())?;
    }
    state.emit_snapshot(&app);
    Ok(settings)
}

#[tauri::command]
fn update_source_profile(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    id: String,
    patch: SourceProfilePatch,
) -> Result<Vec<SourceProfile>, String> {
    let profiles = state
        .storage
        .lock()
        .expect("storage lock")
        .update_source_profile(&id, patch)?;
    state.emit_snapshot(&app);
    Ok(profiles)
}

#[tauri::command]
fn install_hooks(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    sources: Vec<String>,
) -> OperationResult {
    match state.installer.install(&sources) {
        Ok(_) => {
            state.emit_snapshot(&app);
            OperationResult::ok()
        }
        Err(error) => OperationResult::error(error),
    }
}

#[tauri::command]
fn uninstall_hooks(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    sources: Vec<String>,
) -> OperationResult {
    match state.installer.uninstall(&sources) {
        Ok(_) => {
            state.emit_snapshot(&app);
            OperationResult::ok()
        }
        Err(error) => OperationResult::error(error),
    }
}

#[tauri::command]
fn copy_generic_command(app: AppHandle, state: State<'_, Arc<RuntimeState>>) -> OperationResult {
    match app
        .clipboard()
        .write_text(state.installer.generic_command())
    {
        Ok(()) => OperationResult::ok(),
        Err(error) => OperationResult::error(error.to_string()),
    }
}

#[tauri::command]
fn send_test_event(app: AppHandle, state: State<'_, Arc<RuntimeState>>) -> OperationResult {
    let request = BridgeRequest {
        protocol: PROTOCOL_VERSION,
        id: format!("test-{}", Utc::now().timestamp_millis()),
        token: String::new(),
        source: "generic".to_owned(),
        event: Some("Notification".to_owned()),
        args: Vec::new(),
        environment: HashMap::new(),
        payload: Map::from_iter([
            (
                "title".to_owned(),
                Value::String("HookDock 测试通知".to_owned()),
            ),
            (
                "message".to_owned(),
                Value::String("Windows 通知链路工作正常".to_owned()),
            ),
        ]),
    };
    state.ingest(&app, normalize_hook(&request, Utc::now()));
    OperationResult::ok()
}

#[tauri::command]
fn open_folder(app: AppHandle, path: String) -> OperationResult {
    let path = Path::new(&path);
    if !path.is_dir() {
        return OperationResult::error("项目目录不存在");
    }
    match app
        .opener()
        .open_path(path.to_string_lossy().into_owned(), None::<&str>)
    {
        Ok(()) => OperationResult::ok(),
        Err(error) => OperationResult::error(error.to_string()),
    }
}

#[tauri::command]
fn open_url(app: AppHandle, url: String) -> OperationResult {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return OperationResult::error("Only http and https URLs are allowed");
    }
    match app.opener().open_url(url, None::<&str>) {
        Ok(()) => OperationResult::ok(),
        Err(error) => OperationResult::error(error.to_string()),
    }
}

#[tauri::command]
fn hide_window(app: AppHandle) -> OperationResult {
    match app.get_webview_window("main").map(|window| window.hide()) {
        Some(Ok(())) => OperationResult::ok(),
        Some(Err(error)) => OperationResult::error(error.to_string()),
        None => OperationResult::error("找不到主窗口"),
    }
}

#[tauri::command]
fn set_compact(
    app: AppHandle,
    state: State<'_, Arc<RuntimeState>>,
    compact: bool,
) -> OperationResult {
    state.compact.store(compact, Ordering::Relaxed);
    let Some(window) = app.get_webview_window("main") else {
        return OperationResult::error("找不到主窗口");
    };
    apply_window_mode(&window, compact);
    position_window(&window, &state);
    let _ = window.show();
    if !compact {
        let _ = window.set_focus();
    }
    state.emit_snapshot(&app);
    OperationResult::ok()
}

#[tauri::command]
fn snap_window(app: AppHandle, state: State<'_, Arc<RuntimeState>>) -> OperationResult {
    let Some(window) = app.get_webview_window("main") else {
        return OperationResult::error("找不到主窗口");
    };
    match save_and_snap_window(&window, &state) {
        Ok(()) => OperationResult::ok(),
        Err(error) => OperationResult::error(error),
    }
}

fn apply_window_mode(window: &WebviewWindow, compact: bool) {
    let size = if compact {
        tauri::LogicalSize::new(COMPACT_WIDTH, COMPACT_HEIGHT)
    } else {
        tauri::LogicalSize::new(EXPANDED_WIDTH, EXPANDED_HEIGHT)
    };
    let minimum_size = if compact {
        tauri::LogicalSize::new(COMPACT_WIDTH, COMPACT_HEIGHT)
    } else {
        tauri::LogicalSize::new(420.0, 300.0)
    };
    let _ = window.set_min_size(Some(minimum_size));
    let _ = window.set_size(size);
    let _ = window.set_resizable(!compact);
}

fn position_window(window: &WebviewWindow, state: &RuntimeState) {
    let placement = state.storage.lock().expect("storage lock").placement();
    let monitor = window
        .available_monitors()
        .ok()
        .and_then(|monitors| {
            placement.monitor_name.as_ref().and_then(|name| {
                monitors
                    .iter()
                    .find(|monitor| monitor.name() == Some(name))
                    .cloned()
            })
        })
        .or_else(|| window.current_monitor().ok().flatten())
        .or_else(|| window.primary_monitor().ok().flatten());
    let (Some(monitor), Ok(size)) = (monitor, window.outer_size()) else {
        return;
    };
    let work_area = monitor.work_area();
    let margin = (EDGE_MARGIN as f64 * monitor.scale_factor()).round() as i32;
    let min_y = work_area.position.y + margin;
    let available_y = (work_area.size.height as i32 - size.height as i32 - margin * 2).max(0);
    let y = min_y + (available_y as f64 * placement.vertical_ratio.clamp(0.0, 1.0)).round() as i32;
    let x = match placement.edge {
        DockEdge::Left => work_area.position.x + margin,
        DockEdge::Right => {
            work_area.position.x + work_area.size.width as i32 - size.width as i32 - margin
        }
    };
    let _ = window.set_position(PhysicalPosition::new(x, y));
}

fn save_and_snap_window(window: &WebviewWindow, state: &RuntimeState) -> Result<(), String> {
    let monitor = window
        .current_monitor()
        .map_err(|error| error.to_string())?
        .ok_or_else(|| "无法识别当前显示器".to_owned())?;
    let position = window.outer_position().map_err(|error| error.to_string())?;
    let size = window.outer_size().map_err(|error| error.to_string())?;
    let work_area = monitor.work_area();
    let window_center = position.x as i64 + size.width as i64 / 2;
    let work_center = work_area.position.x as i64 + work_area.size.width as i64 / 2;
    let edge = if window_center < work_center {
        DockEdge::Left
    } else {
        DockEdge::Right
    };
    let margin = (EDGE_MARGIN as f64 * monitor.scale_factor()).round() as i32;
    let min_y = work_area.position.y + margin;
    let available_y = (work_area.size.height as i32 - size.height as i32 - margin * 2).max(1);
    let vertical_ratio = ((position.y - min_y) as f64 / available_y as f64).clamp(0.0, 1.0);
    state
        .storage
        .lock()
        .expect("storage lock")
        .set_placement(SavedPlacement {
            monitor_name: monitor.name().cloned(),
            edge,
            vertical_ratio,
        })?;
    position_window(window, state);
    Ok(())
}

fn reveal_window(app: &AppHandle, state: &RuntimeState, expanded: bool) {
    if let Some(window) = app.get_webview_window("main") {
        if expanded {
            state.compact.store(false, Ordering::Relaxed);
        }
        let compact = state.compact.load(Ordering::Relaxed);
        apply_window_mode(&window, compact);
        position_window(&window, state);
        let _ = window.show();
        if expanded {
            let _ = window.set_focus();
        }
    }
    state.emit_snapshot(app);
}

fn build_tray(app: &AppHandle, state: Arc<RuntimeState>) -> tauri::Result<()> {
    let show = MenuItem::with_id(app, "show", "打开通知面板", true, None::<&str>)?;
    let test = MenuItem::with_id(app, "test", "发送测试通知", true, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(app)?;
    let quit = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show, &test, &separator, &quit])?;
    let state_for_menu = state.clone();
    let state_for_click = state;
    TrayIconBuilder::with_id("main")
        .tooltip("HookDock · 正在监听 Hook")
        .icon(app.default_window_icon().expect("app icon").clone())
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id.as_ref() {
            "show" => reveal_window(app, &state_for_menu, true),
            "test" => {
                let request = BridgeRequest {
                    protocol: PROTOCOL_VERSION,
                    id: format!("test-{}", Utc::now().timestamp_millis()),
                    token: String::new(),
                    source: "generic".to_owned(),
                    event: Some("Notification".to_owned()),
                    args: Vec::new(),
                    environment: HashMap::new(),
                    payload: Map::from_iter([
                        (
                            "title".to_owned(),
                            Value::String("HookDock 测试通知".to_owned()),
                        ),
                        (
                            "message".to_owned(),
                            Value::String("Windows 通知链路工作正常".to_owned()),
                        ),
                    ]),
                };
                state_for_menu.ingest(app, normalize_hook(&request, Utc::now()));
            }
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(move |tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                reveal_window(tray.app_handle(), &state_for_click, true);
            }
        })
        .build(app)?;
    Ok(())
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(state) = app.try_state::<Arc<RuntimeState>>() {
                reveal_window(app, &state, true);
            }
        }))
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_autostart::Builder::new()
                .args(["--autostart"])
                .build(),
        )
        .invoke_handler(tauri::generate_handler![
            get_state,
            respond_to_hook,
            dismiss_event,
            focus_event_terminal,
            update_settings,
            update_source_profile,
            install_hooks,
            uninstall_hooks,
            copy_generic_command,
            send_test_event,
            open_folder,
            open_url,
            hide_window,
            set_compact,
            snap_window,
        ])
        .setup(|app| {
            let data_directory = dirs::data_dir()
                .ok_or_else(|| std::io::Error::other("找不到用户应用数据目录"))?
                .join(APP_DATA_DIRECTORY);
            let home_directory =
                dirs::home_dir().ok_or_else(|| std::io::Error::other("找不到用户主目录"))?;
            let bridge_path = app
                .path()
                .resolve("resources/hookdock-hook.exe", BaseDirectory::Resource)?;
            let started_from_autostart = std::env::args().any(|argument| argument == "--autostart");
            let state = Arc::new(RuntimeState::new(
                data_directory,
                HookInstaller::new(home_directory, bridge_path),
                started_from_autostart,
            ));
            app.manage(state.clone());
            hook_server::start(app.handle().clone(), state.clone())
                .map_err(std::io::Error::other)?;
            build_tray(app.handle(), state.clone())?;
            if let Some(window) = app.get_webview_window("main") {
                apply_window_mode(&window, started_from_autostart);
                position_window(&window, &state);
                window.show()?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("failed to build HookDock")
        .run(|app, event| {
            if let tauri::RunEvent::Exit = event {
                if let Some(state) = app.try_state::<Arc<RuntimeState>>() {
                    state.shutting_down.store(true, Ordering::Relaxed);
                    for (id, sender) in state.pending.lock().expect("pending lock").drain() {
                        let _ = sender.send(BridgeResponse {
                            decision: Some(Decision::Cancel),
                            reason: Some("HookDock is closing".to_owned()),
                            ..BridgeResponse::acknowledged(id)
                        });
                    }
                    let _ = std::fs::remove_file(&state.runtime_path);
                }
            }
        });
}
