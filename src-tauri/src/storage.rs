use crate::file_replace::replace_file;
use chrono::Utc;
use hookdock_protocol::{Decision, HookEvent};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

const MAX_EVENT_HISTORY: usize = 50;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub notifications_enabled: bool,
    pub show_panel_on_attention: bool,
    pub open_at_login: bool,
    pub play_sound: bool,
    pub language: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            show_panel_on_attention: true,
            open_at_login: true,
            play_sound: true,
            language: "auto".to_owned(),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettingsPatch {
    pub notifications_enabled: Option<bool>,
    pub show_panel_on_attention: Option<bool>,
    pub open_at_login: Option<bool>,
    pub play_sound: Option<bool>,
    pub language: Option<String>,
}

impl AppSettings {
    pub fn apply(&mut self, patch: AppSettingsPatch) {
        if let Some(value) = patch.notifications_enabled {
            self.notifications_enabled = value;
        }
        if let Some(value) = patch.show_panel_on_attention {
            self.show_panel_on_attention = value;
        }
        if let Some(value) = patch.open_at_login {
            self.open_at_login = value;
        }
        if let Some(value) = patch.play_sound {
            self.play_sound = value;
        }
        if let Some(value) = patch.language {
            if matches!(value.as_str(), "auto" | "zh-CN" | "en") {
                self.language = value;
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct SourceProfile {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub accent_color: String,
    pub show_toast: bool,
    pub play_sound: bool,
    pub auto_expand: bool,
    pub enabled: bool,
}

impl Default for SourceProfile {
    fn default() -> Self {
        Self {
            id: "generic".to_owned(),
            name: "Custom Hook".to_owned(),
            icon: "⚓".to_owned(),
            accent_color: "#22d3ee".to_owned(),
            show_toast: true,
            play_sound: true,
            auto_expand: true,
            enabled: true,
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceProfilePatch {
    pub name: Option<String>,
    pub icon: Option<String>,
    pub accent_color: Option<String>,
    pub show_toast: Option<bool>,
    pub play_sound: Option<bool>,
    pub auto_expand: Option<bool>,
    pub enabled: Option<bool>,
}

fn default_source_profiles() -> Vec<SourceProfile> {
    [
        ("claude", "Claude Code", "C", "#d97757"),
        ("codex", "Codex", "⌁", "#22c55e"),
        ("gemini", "Gemini CLI", "✦", "#8b5cf6"),
        ("generic", "Custom Hook", "⚓", "#22d3ee"),
    ]
    .into_iter()
    .map(|(id, name, icon, accent_color)| SourceProfile {
        id: id.to_owned(),
        name: name.to_owned(),
        icon: icon.to_owned(),
        accent_color: accent_color.to_owned(),
        ..SourceProfile::default()
    })
    .collect()
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DockEdge {
    Left,
    #[default]
    Right,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedPlacement {
    pub monitor_name: Option<String>,
    pub edge: DockEdge,
    pub vertical_ratio: f64,
}

impl Default for SavedPlacement {
    fn default() -> Self {
        Self {
            monitor_name: None,
            edge: DockEdge::Right,
            vertical_ratio: 0.02,
        }
    }
}

pub struct Storage {
    directory: PathBuf,
    settings: AppSettings,
    placement: SavedPlacement,
    events: Vec<HookEvent>,
    source_profiles: Vec<SourceProfile>,
}

impl Storage {
    pub fn load(directory: PathBuf) -> Self {
        let settings = read_json(&directory.join("settings.json")).unwrap_or_default();
        let placement = read_json(&directory.join("window-placement.json")).unwrap_or_default();
        let events = read_json::<Vec<HookEvent>>(&directory.join("events.json"))
            .unwrap_or_default()
            .into_iter()
            .take(MAX_EVENT_HISTORY)
            .collect();
        let mut source_profiles = read_json::<Vec<SourceProfile>>(&directory.join("sources.json"))
            .unwrap_or_else(default_source_profiles);
        for default in default_source_profiles() {
            if !source_profiles
                .iter()
                .any(|profile| profile.id == default.id)
            {
                source_profiles.push(default);
            }
        }
        Self {
            directory,
            settings,
            placement,
            events,
            source_profiles,
        }
    }

    pub fn settings(&self) -> AppSettings {
        self.settings.clone()
    }

    pub fn source_profiles(&self) -> Vec<SourceProfile> {
        self.source_profiles.clone()
    }

    pub fn source_profile(&self, id: &str) -> SourceProfile {
        self.source_profiles
            .iter()
            .find(|profile| profile.id == id)
            .cloned()
            .unwrap_or_default()
    }

    pub fn update_source_profile(
        &mut self,
        id: &str,
        patch: SourceProfilePatch,
    ) -> Result<Vec<SourceProfile>, String> {
        let profile = self
            .source_profiles
            .iter_mut()
            .find(|profile| profile.id == id)
            .ok_or_else(|| format!("Unknown source profile: {id}"))?;
        if let Some(value) = patch.name {
            let value = value.trim();
            if !value.is_empty() && value.chars().count() <= 40 {
                profile.name = value.to_owned();
            }
        }
        if let Some(value) = patch.icon {
            let value = value.trim();
            if !value.is_empty() && value.chars().count() <= 8 {
                profile.icon = value.to_owned();
            }
        }
        if let Some(value) = patch.accent_color {
            if is_hex_color(&value) {
                profile.accent_color = value.to_ascii_lowercase();
            }
        }
        if let Some(value) = patch.show_toast {
            profile.show_toast = value;
        }
        if let Some(value) = patch.play_sound {
            profile.play_sound = value;
        }
        if let Some(value) = patch.auto_expand {
            profile.auto_expand = value;
        }
        if let Some(value) = patch.enabled {
            profile.enabled = value;
        }
        write_json_atomic(&self.directory.join("sources.json"), &self.source_profiles)?;
        Ok(self.source_profiles())
    }

    pub fn update_settings(&mut self, patch: AppSettingsPatch) -> Result<AppSettings, String> {
        self.settings.apply(patch);
        write_json_atomic(&self.directory.join("settings.json"), &self.settings)?;
        Ok(self.settings())
    }

    pub fn placement(&self) -> SavedPlacement {
        self.placement.clone()
    }

    pub fn set_placement(&mut self, placement: SavedPlacement) -> Result<(), String> {
        self.placement = placement;
        write_json_atomic(
            &self.directory.join("window-placement.json"),
            &self.placement,
        )
    }

    pub fn events(&self) -> Vec<HookEvent> {
        self.events.clone()
    }

    pub fn add_event(&mut self, mut event: HookEvent) -> Result<HookEvent, String> {
        if event.source == "codex"
            && event.terminal_target.is_none()
            && !event.terminal_context_observed
        {
            event.terminal_target = self
                .events
                .iter()
                .find(|candidate| candidate.session_key == event.session_key)
                .and_then(|candidate| candidate.terminal_target.clone());
        }
        self.events.retain(|candidate| candidate.id != event.id);
        self.events.insert(0, event.clone());
        self.events.truncate(MAX_EVENT_HISTORY);
        write_json_atomic(&self.directory.join("events.json"), &self.events)?;
        Ok(event)
    }

    pub fn resolve_event(&mut self, id: &str, decision: Decision) -> Result<bool, String> {
        let Some(event) = self.events.iter_mut().find(|event| event.id == id) else {
            return Ok(false);
        };
        event.resolution = Some(decision);
        event.resolved_at = Some(Utc::now());
        write_json_atomic(&self.directory.join("events.json"), &self.events)?;
        Ok(true)
    }

    pub fn dismiss_event(&mut self, id: &str) -> Result<(), String> {
        self.events.retain(|event| event.id != id);
        write_json_atomic(&self.directory.join("events.json"), &self.events)
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn is_hex_color(value: &str) -> bool {
    value.len() == 7
        && value.starts_with('#')
        && value.as_bytes()[1..].iter().all(u8::is_ascii_hexdigit)
}

fn write_json_atomic(path: &Path, value: &impl Serialize) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "invalid storage path".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temporary = path.with_extension("hookdock.tmp");
    let mut data = serde_json::to_vec_pretty(value).map_err(|error| error.to_string())?;
    data.push(b'\n');
    fs::write(&temporary, data).map_err(|error| error.to_string())?;
    replace_file(&temporary, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use hookdock_protocol::{TerminalTarget, TerminalTargetKind};
    use serde_json::json;
    use tempfile::tempdir;

    fn event(id: &str, terminal_target: Option<TerminalTarget>) -> HookEvent {
        let mut value = json!({
            "id": id,
            "source": "codex",
            "providerName": "Codex",
            "eventType": "Stop",
            "sessionKey": "codex:thread-1",
            "project": "hookdock",
            "title": "Codex completed",
            "body": "Done",
            "status": "completed",
            "questions": [],
            "expectsResponse": false,
            "shouldNotify": true,
            "receivedAt": "2026-09-10T00:00:00Z"
        });
        if let Some(target) = terminal_target {
            value["terminalTarget"] = serde_json::to_value(target).unwrap();
        }
        serde_json::from_value(value).unwrap()
    }

    #[test]
    fn source_profiles_persist_valid_updates() {
        let directory = tempdir().unwrap();
        let mut storage = Storage::load(directory.path().to_owned());
        let profiles = storage
            .update_source_profile(
                "codex",
                SourceProfilePatch {
                    name: Some("Build Agent".to_owned()),
                    accent_color: Some("#AABBCC".to_owned()),
                    show_toast: Some(false),
                    ..SourceProfilePatch::default()
                },
            )
            .unwrap();
        let codex = profiles
            .iter()
            .find(|profile| profile.id == "codex")
            .unwrap();
        assert_eq!(codex.name, "Build Agent");
        assert_eq!(codex.accent_color, "#aabbcc");
        assert!(!codex.show_toast);

        let reloaded = Storage::load(directory.path().to_owned());
        assert_eq!(reloaded.source_profile("codex").name, "Build Agent");
    }

    #[test]
    fn invalid_profile_values_do_not_replace_saved_values() {
        let directory = tempdir().unwrap();
        let mut storage = Storage::load(directory.path().to_owned());
        storage
            .update_source_profile(
                "generic",
                SourceProfilePatch {
                    name: Some(" ".to_owned()),
                    accent_color: Some("red".to_owned()),
                    ..SourceProfilePatch::default()
                },
            )
            .unwrap();
        let generic = storage.source_profile("generic");
        assert_eq!(generic.name, "Custom Hook");
        assert_eq!(generic.accent_color, "#22d3ee");
    }

    #[test]
    fn codex_events_reuse_the_latest_terminal_target_for_the_session() {
        let directory = tempdir().unwrap();
        let mut storage = Storage::load(directory.path().to_owned());
        let target = TerminalTarget {
            kind: TerminalTargetKind::HookDockTerminal,
            session_id: "7f6b3978-25f1-4519-8b02-8fe67f35991f".to_owned(),
            protocol: 1,
        };
        storage
            .add_event(event("first", Some(target.clone())))
            .unwrap();
        let inherited = storage.add_event(event("second", None)).unwrap();

        assert_eq!(inherited.terminal_target, Some(target.clone()));
        assert_eq!(
            Storage::load(directory.path().to_owned()).events()[0].terminal_target,
            Some(target)
        );
    }

    #[test]
    fn observed_non_capable_terminal_context_clears_a_cached_target() {
        let directory = tempdir().unwrap();
        let mut storage = Storage::load(directory.path().to_owned());
        let target = TerminalTarget {
            kind: TerminalTargetKind::HookDockTerminal,
            session_id: "7f6b3978-25f1-4519-8b02-8fe67f35991f".to_owned(),
            protocol: 1,
        };
        storage.add_event(event("first", Some(target))).unwrap();
        let mut moved_to_another_terminal = event("second", None);
        moved_to_another_terminal.terminal_context_observed = true;

        let stored = storage.add_event(moved_to_another_terminal).unwrap();
        assert!(stored.terminal_target.is_none());
    }
}
