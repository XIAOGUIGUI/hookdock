use crate::file_replace::replace_file;
use crate::RuntimeState;
use chrono::Utc;
use hookdock_protocol::{
    normalize_hook, BridgeRequest, BridgeResponse, Decision, RuntimeConfig, DEFAULT_HOOK_PORT,
    MAX_HOOK_BYTES, PROTOCOL_VERSION,
};
use serde::Deserialize;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{Ipv4Addr, TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::thread;
use std::time::Duration;
use tauri::AppHandle;

const MAX_HEADER_BYTES: usize = 16 * 1024;
const RESPONSE_TIMEOUT: Duration = Duration::from_secs(24 * 60 * 60);

pub fn start(app: AppHandle, state: Arc<RuntimeState>) -> Result<u16, String> {
    let listener = TcpListener::bind((Ipv4Addr::LOCALHOST, DEFAULT_HOOK_PORT))
        .or_else(|_| TcpListener::bind((Ipv4Addr::LOCALHOST, 0)))
        .map_err(|error| error.to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let token = random_token();
    write_runtime_file(&state.runtime_path, port, &token)?;
    state.port.store(port, Ordering::Relaxed);
    state.listening.store(true, Ordering::Relaxed);

    thread::Builder::new()
        .name("hookdock-listener".to_owned())
        .spawn(move || {
            while !state.shutting_down.load(Ordering::Relaxed) {
                match listener.accept() {
                    Ok((stream, address)) if address.ip().is_loopback() => {
                        let app = app.clone();
                        let state = state.clone();
                        let token = token.clone();
                        let _ = thread::Builder::new()
                            .name("hookdock-request".to_owned())
                            .spawn(move || handle_connection(stream, &token, app, state));
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        thread::sleep(Duration::from_millis(40));
                    }
                    Err(error) => {
                        eprintln!("HookDock listener error: {error}");
                        thread::sleep(Duration::from_millis(200));
                    }
                }
            }
            state.listening.store(false, Ordering::Relaxed);
            let _ = fs::remove_file(&state.runtime_path);
        })
        .map_err(|error| error.to_string())?;
    Ok(port)
}

fn handle_connection(
    stream: TcpStream,
    expected_token: &str,
    app: AppHandle,
    state: Arc<RuntimeState>,
) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(3)));
    let mut reader = BufReader::new(stream);
    let mut first_line = Vec::new();
    let read_result = reader
        .by_ref()
        .take(MAX_HOOK_BYTES + 1)
        .read_until(b'\n', &mut first_line);
    if read_result.is_err() || first_line.len() as u64 > MAX_HOOK_BYTES {
        let _ = write_bridge_response(
            reader.get_mut(),
            &BridgeResponse::rejected("", "invalid_payload"),
        );
        return;
    }

    if first_line.starts_with(b"GET ") || first_line.starts_with(b"POST ") {
        handle_http(reader, &first_line, expected_token, app, state);
    } else {
        handle_bridge(reader.get_mut(), &first_line, expected_token, app, state);
    }
}

fn handle_bridge(
    stream: &mut TcpStream,
    bytes: &[u8],
    expected_token: &str,
    app: AppHandle,
    state: Arc<RuntimeState>,
) {
    let request: BridgeRequest = match serde_json::from_slice(bytes) {
        Ok(request) => request,
        Err(_) => {
            let _ = write_bridge_response(stream, &BridgeResponse::rejected("", "invalid_json"));
            return;
        }
    };
    if request.protocol != PROTOCOL_VERSION || !constant_time_eq(&request.token, expected_token) {
        let _ = write_bridge_response(
            stream,
            &BridgeResponse::rejected(request.id, "unauthorized"),
        );
        return;
    }
    let response = process_request(request, app, state);
    let _ = write_bridge_response(stream, &response);
}

fn process_request(
    request: BridgeRequest,
    app: AppHandle,
    state: Arc<RuntimeState>,
) -> BridgeResponse {
    let event = normalize_hook(&request, Utc::now());
    let event_id = event.id.clone();
    let receiver = event
        .expects_response
        .then(|| state.register_pending(event_id.clone()));
    state.ingest(&app, event);

    if let Some(receiver) = receiver {
        match receiver.recv_timeout(RESPONSE_TIMEOUT) {
            Ok(response) => response,
            Err(_) => {
                state.remove_pending(&event_id);
                state.emit_snapshot(&app);
                BridgeResponse {
                    decision: Some(Decision::Cancel),
                    reason: Some("HookDock response timed out".to_owned()),
                    ..BridgeResponse::acknowledged(event_id)
                }
            }
        }
    } else {
        BridgeResponse::acknowledged(event_id)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HttpNotification {
    #[serde(default = "default_source")]
    source: String,
    title: String,
    #[serde(alias = "message")]
    body: String,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct HttpInteraction {
    #[serde(default = "default_source")]
    source: String,
    title: String,
    #[serde(alias = "message", alias = "prompt")]
    body: String,
    #[serde(default = "default_request_kind")]
    kind: String,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    options: Vec<String>,
    #[serde(default)]
    project: Option<String>,
    #[serde(default)]
    url: Option<String>,
}

fn default_source() -> String {
    "generic".to_owned()
}

fn default_request_kind() -> String {
    "approval".to_owned()
}

fn handle_http(
    mut reader: BufReader<TcpStream>,
    request_line: &[u8],
    expected_token: &str,
    app: AppHandle,
    state: Arc<RuntimeState>,
) {
    let request_line = match std::str::from_utf8(request_line) {
        Ok(value) => value.trim_end(),
        Err(_) => {
            let _ = write_http_error(reader.get_mut(), 400, "invalid_request_line");
            return;
        }
    };
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default();
    let path = parts
        .next()
        .unwrap_or_default()
        .split('?')
        .next()
        .unwrap_or_default();
    let mut authorization = None;
    let mut content_length = None;
    let mut header_bytes = request_line.len();
    loop {
        let mut line = String::new();
        if !matches!(reader.read_line(&mut line), Ok(count) if count > 0) {
            let _ = write_http_error(reader.get_mut(), 400, "invalid_headers");
            return;
        }
        header_bytes += line.len();
        if header_bytes > MAX_HEADER_BYTES {
            let _ = write_http_error(reader.get_mut(), 431, "headers_too_large");
            return;
        }
        if line == "\r\n" || line == "\n" {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            match name.trim().to_ascii_lowercase().as_str() {
                "authorization" => authorization = Some(value.trim().to_owned()),
                "content-length" => content_length = value.trim().parse::<usize>().ok(),
                _ => {}
            }
        }
    }

    let supplied_token = authorization
        .as_deref()
        .and_then(|value| value.strip_prefix("Bearer "))
        .unwrap_or_default();
    if !constant_time_eq(supplied_token, expected_token) {
        let _ = write_http_error(reader.get_mut(), 401, "unauthorized");
        return;
    }

    if method == "GET" && path == "/v1/health" {
        let response = json!({
            "name": "HookDock",
            "version": env!("CARGO_PKG_VERSION"),
            "protocol": PROTOCOL_VERSION,
            "status": "ok"
        });
        let _ = write_http_json(reader.get_mut(), 200, &response);
        return;
    }

    if method != "POST" || !matches!(path, "/v1/notifications" | "/v1/requests") {
        let _ = write_http_error(reader.get_mut(), 404, "not_found");
        return;
    }
    let Some(content_length) = content_length else {
        let _ = write_http_error(reader.get_mut(), 411, "content_length_required");
        return;
    };
    if content_length as u64 > MAX_HOOK_BYTES {
        let _ = write_http_error(reader.get_mut(), 413, "payload_too_large");
        return;
    }
    let mut body = vec![0; content_length];
    if reader.read_exact(&mut body).is_err() {
        let _ = write_http_error(reader.get_mut(), 400, "incomplete_body");
        return;
    }

    if path == "/v1/notifications" {
        let input: HttpNotification = match serde_json::from_slice(&body) {
            Ok(value) => value,
            Err(_) => {
                let _ = write_http_error(reader.get_mut(), 400, "invalid_json");
                return;
            }
        };
        let request = match notification_request(input, expected_token) {
            Ok(value) => value,
            Err(error) => {
                let _ = write_http_error(reader.get_mut(), 422, error);
                return;
            }
        };
        let response = process_request(request, app, state);
        let payload = json!({ "id": response.id, "status": "accepted" });
        let _ = write_http_json(reader.get_mut(), 202, &payload);
    } else {
        let input: HttpInteraction = match serde_json::from_slice(&body) {
            Ok(value) => value,
            Err(_) => {
                let _ = write_http_error(reader.get_mut(), 400, "invalid_json");
                return;
            }
        };
        let request = match interaction_request(input, expected_token) {
            Ok(value) => value,
            Err(error) => {
                let _ = write_http_error(reader.get_mut(), 422, error);
                return;
            }
        };
        let response = process_request(request, app, state);
        let payload = json!({
            "id": response.id,
            "decision": response.decision,
            "reason": response.reason,
            "answers": response.answers
        });
        let _ = write_http_json(reader.get_mut(), 200, &payload);
    }
}

fn notification_request(
    input: HttpNotification,
    token: &str,
) -> Result<BridgeRequest, &'static str> {
    validate_text(&input.title, &input.body)?;
    let mut payload = Map::from_iter([
        ("title".to_owned(), Value::String(input.title)),
        ("body".to_owned(), Value::String(input.body)),
    ]);
    if let Some(level) = input.level {
        payload.insert("status".to_owned(), Value::String(level));
    }
    add_optional_fields(&mut payload, input.project, input.url);
    Ok(public_request(input.source, "Notification", token, payload))
}

fn interaction_request(input: HttpInteraction, token: &str) -> Result<BridgeRequest, &'static str> {
    validate_text(&input.title, &input.body)?;
    if !matches!(input.kind.as_str(), "approval" | "question") {
        return Err("kind_must_be_approval_or_question");
    }
    let mut payload = Map::from_iter([
        ("title".to_owned(), Value::String(input.title)),
        ("body".to_owned(), Value::String(input.body.clone())),
    ]);
    let event = if input.kind == "question" {
        payload.insert(
            "tool_name".to_owned(),
            Value::String("AskUserQuestion".to_owned()),
        );
        payload.insert(
            "questions".to_owned(),
            json!([{ "id": "answer", "question": input.body, "options": input.options }]),
        );
        "UserInputRequest"
    } else {
        if let Some(action) = input.action {
            payload.insert("tool_name".to_owned(), Value::String(action));
        }
        "PermissionRequest"
    };
    add_optional_fields(&mut payload, input.project, input.url);
    Ok(public_request(input.source, event, token, payload))
}

fn validate_text(title: &str, body: &str) -> Result<(), &'static str> {
    if title.trim().is_empty() || title.chars().count() > 160 {
        return Err("invalid_title");
    }
    if body.trim().is_empty() || body.chars().count() > 8_000 {
        return Err("invalid_body");
    }
    Ok(())
}

fn add_optional_fields(
    payload: &mut Map<String, Value>,
    project: Option<String>,
    url: Option<String>,
) {
    if let Some(project) = project.filter(|value| !value.trim().is_empty()) {
        payload.insert("project".to_owned(), Value::String(project));
    }
    if let Some(url) =
        url.filter(|value| value.starts_with("https://") || value.starts_with("http://"))
    {
        payload.insert("url".to_owned(), Value::String(url));
    }
}

fn public_request(
    source: String,
    event: &str,
    token: &str,
    payload: Map<String, Value>,
) -> BridgeRequest {
    BridgeRequest {
        protocol: PROTOCOL_VERSION,
        id: format!(
            "http-{}-{:x}",
            Utc::now().timestamp_millis(),
            rand::random::<u64>()
        ),
        token: token.to_owned(),
        source,
        event: Some(event.to_owned()),
        args: Vec::new(),
        environment: HashMap::new(),
        payload,
    }
}

fn write_bridge_response(stream: &mut TcpStream, response: &BridgeResponse) -> Result<(), String> {
    serde_json::to_writer(&mut *stream, response).map_err(|error| error.to_string())?;
    stream.write_all(b"\n").map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())
}

fn write_http_error(stream: &mut TcpStream, status: u16, error: &str) -> Result<(), String> {
    write_http_json(stream, status, &json!({ "error": error }))
}

fn write_http_json(stream: &mut TcpStream, status: u16, payload: &Value) -> Result<(), String> {
    let body = serde_json::to_vec(payload).map_err(|error| error.to_string())?;
    let reason = match status {
        200 => "OK",
        202 => "Accepted",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        411 => "Length Required",
        413 => "Payload Too Large",
        422 => "Unprocessable Content",
        431 => "Request Header Fields Too Large",
        _ => "Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        body.len()
    )
    .map_err(|error| error.to_string())?;
    stream.write_all(&body).map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())
}

fn random_token() -> String {
    rand::random::<[u8; 32]>()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn write_runtime_file(path: &Path, port: u16, token: &str) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "invalid runtime path".to_owned())?;
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let runtime = RuntimeConfig {
        protocol: PROTOCOL_VERSION,
        host: Ipv4Addr::LOCALHOST.to_string(),
        port,
        token: token.to_owned(),
        pid: std::process::id(),
        updated_at: Utc::now().to_rfc3339(),
    };
    let temporary = temporary_runtime_path(path);
    let mut bytes = serde_json::to_vec_pretty(&runtime).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    replace_file(&temporary, path)
}

fn temporary_runtime_path(path: &Path) -> PathBuf {
    path.with_extension("hookdock.tmp")
}

fn constant_time_eq(left: &str, right: &str) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.as_bytes()
        .iter()
        .zip(right.as_bytes())
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_comparison_rejects_different_values_and_lengths() {
        assert!(constant_time_eq("abc", "abc"));
        assert!(!constant_time_eq("abc", "abd"));
        assert!(!constant_time_eq("abc", "ab"));
    }

    #[test]
    fn public_notification_maps_to_bridge_request() {
        let request = notification_request(
            HttpNotification {
                source: "build".to_owned(),
                title: "Ready".to_owned(),
                body: "All checks passed".to_owned(),
                level: Some("success".to_owned()),
                project: Some("demo".to_owned()),
                url: Some("https://example.com/run".to_owned()),
            },
            "secret",
        )
        .unwrap();
        assert_eq!(request.token, "secret");
        assert_eq!(request.event.as_deref(), Some("Notification"));
        assert_eq!(request.payload["project"], "demo");
    }

    #[test]
    fn public_request_rejects_blank_text() {
        let error = notification_request(
            HttpNotification {
                source: default_source(),
                title: " ".to_owned(),
                body: "message".to_owned(),
                level: None,
                project: None,
                url: None,
            },
            "secret",
        )
        .unwrap_err();
        assert_eq!(error, "invalid_title");
    }
}
