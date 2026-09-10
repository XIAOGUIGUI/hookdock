use hookdock_protocol::{
    provider_stdout, BridgeRequest, BridgeResponse, RuntimeConfig, MAX_HOOK_BYTES, PROTOCOL_VERSION,
};
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::{self, BufRead, IsTerminal, Read, Write};
use std::net::{Ipv4Addr, SocketAddrV4, TcpStream};
use std::path::PathBuf;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug)]
struct Arguments {
    source: String,
    event: Option<String>,
    runtime_path: Option<PathBuf>,
    raw: Vec<String>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("hookdock-hook: {error}");
        println!("{{}}");
    }
}

fn run() -> Result<(), String> {
    let arguments = parse_arguments(env::args().skip(1).collect())?;
    let payload = read_payload(&arguments.raw)?;
    let runtime = load_runtime(arguments.runtime_path.as_ref())?;
    let request = BridgeRequest {
        protocol: PROTOCOL_VERSION,
        id: Uuid::new_v4().to_string(),
        token: runtime.token.clone(),
        source: arguments.source.clone(),
        event: arguments.event.clone(),
        args: arguments.raw,
        environment: captured_environment(),
        payload: payload.clone(),
    };
    let response = send_request(&runtime, &request)?;
    if !response.acknowledged {
        return Err(response
            .error
            .unwrap_or_else(|| "request was rejected".to_owned()));
    }
    println!(
        "{}",
        provider_stdout(
            &arguments.source,
            arguments.event.as_deref(),
            &payload,
            &response,
        )
    );
    Ok(())
}

fn parse_arguments(raw: Vec<String>) -> Result<Arguments, String> {
    let mut source = None;
    let mut event = None;
    let mut runtime_path = None;
    let mut index = 0;
    while index < raw.len() {
        let target = match raw[index].as_str() {
            "--source" => Some("source"),
            "--event" => Some("event"),
            "--runtime" => Some("runtime"),
            _ => None,
        };
        if let Some(target) = target {
            let value = raw
                .get(index + 1)
                .cloned()
                .ok_or_else(|| format!("missing value for {}", raw[index]))?;
            match target {
                "source" => source = Some(value),
                "event" => event = Some(value),
                "runtime" => runtime_path = Some(PathBuf::from(value)),
                _ => unreachable!(),
            }
            index += 2;
        } else {
            index += 1;
        }
    }
    let source = match source.as_deref() {
        Some("claude" | "codex" | "gemini" | "generic") => source.unwrap(),
        Some(_) => "generic".to_owned(),
        None => return Err("--source is required".to_owned()),
    };
    Ok(Arguments {
        source,
        event,
        runtime_path,
        raw,
    })
}

fn read_payload(args: &[String]) -> Result<Map<String, Value>, String> {
    let mut data = Vec::new();
    if !io::stdin().is_terminal() {
        io::stdin()
            .take(MAX_HOOK_BYTES + 1)
            .read_to_end(&mut data)
            .map_err(|error| error.to_string())?;
        if data.len() as u64 > MAX_HOOK_BYTES {
            return Err("payload exceeds 2 MiB".to_owned());
        }
    }
    if data.iter().all(u8::is_ascii_whitespace) {
        if let Some(candidate) = args
            .iter()
            .rev()
            .find(|argument| argument.trim_start().starts_with('{'))
        {
            data = candidate.as_bytes().to_vec();
        }
    }
    if data.iter().all(u8::is_ascii_whitespace) {
        return Ok(Map::new());
    }
    serde_json::from_slice::<Map<String, Value>>(&data).map_err(|error| error.to_string())
}

fn load_runtime(custom_path: Option<&PathBuf>) -> Result<RuntimeConfig, String> {
    let path = match custom_path {
        Some(path) => path.clone(),
        None => PathBuf::from(env::var("APPDATA").map_err(|_| "APPDATA is not set")?)
            .join("HookDock")
            .join("runtime.json"),
    };
    let runtime: RuntimeConfig = serde_json::from_slice(
        &fs::read(&path).map_err(|error| format!("{}: {error}", path.display()))?,
    )
    .map_err(|error| error.to_string())?;
    if runtime.protocol != PROTOCOL_VERSION
        || runtime.host != "127.0.0.1"
        || runtime.token.is_empty()
    {
        return Err("invalid runtime configuration".to_owned());
    }
    Ok(runtime)
}

fn send_request(
    runtime: &RuntimeConfig,
    request: &BridgeRequest,
) -> Result<BridgeResponse, String> {
    let address = SocketAddrV4::new(Ipv4Addr::LOCALHOST, runtime.port);
    let mut stream = TcpStream::connect_timeout(&address.into(), Duration::from_secs(1))
        .map_err(|error| error.to_string())?;
    stream
        .set_write_timeout(Some(Duration::from_secs(2)))
        .map_err(|error| error.to_string())?;
    stream
        .set_read_timeout(Some(Duration::from_secs(24 * 60 * 60)))
        .map_err(|error| error.to_string())?;
    serde_json::to_writer(&mut stream, request).map_err(|error| error.to_string())?;
    stream.write_all(b"\n").map_err(|error| error.to_string())?;
    stream.flush().map_err(|error| error.to_string())?;

    let mut response_line = String::new();
    io::BufReader::new(stream)
        .take(1024 * 1024)
        .read_line(&mut response_line)
        .map_err(|error| error.to_string())?;
    let response: BridgeResponse =
        serde_json::from_str(&response_line).map_err(|error| error.to_string())?;
    if response.protocol != PROTOCOL_VERSION || response.id != request.id {
        return Err("mismatched bridge response".to_owned());
    }
    Ok(response)
}

const CAPTURED_ENVIRONMENT_KEYS: &[&str] = &[
    "PWD",
    "TERM",
    "TERM_PROGRAM",
    "WT_SESSION",
    "HOOKDOCK_TERMINAL",
    "HOOKDOCK_TERMINAL_PROTOCOL",
    "TTY",
    "CLAUDE_SESSION_ID",
    "CODEX_THREAD_ID",
    "VSCODE_PID",
    "CURSOR_TRACE_ID",
];

fn captured_environment() -> HashMap<String, String> {
    let mut values: HashMap<String, String> = CAPTURED_ENVIRONMENT_KEYS
        .iter()
        .filter_map(|key| env::var(key).ok().map(|value| ((*key).to_owned(), value)))
        .collect();
    if !values.contains_key("PWD") {
        if let Ok(cwd) = env::current_dir() {
            values.insert("PWD".to_owned(), cwd.display().to_string());
        }
    }
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_provider_and_event_arguments() {
        let arguments = parse_arguments(vec![
            "--source".to_owned(),
            "codex".to_owned(),
            "--event".to_owned(),
            "Stop".to_owned(),
        ])
        .unwrap();
        assert_eq!(arguments.source, "codex");
        assert_eq!(arguments.event.as_deref(), Some("Stop"));
    }

    #[test]
    fn unknown_provider_uses_generic_mapping() {
        let arguments =
            parse_arguments(vec!["--source".to_owned(), "custom-agent".to_owned()]).unwrap();
        assert_eq!(arguments.source, "generic");
    }

    #[test]
    fn captures_the_hookdock_terminal_capability_contract() {
        assert!(CAPTURED_ENVIRONMENT_KEYS.contains(&"WT_SESSION"));
        assert!(CAPTURED_ENVIRONMENT_KEYS.contains(&"HOOKDOCK_TERMINAL"));
        assert!(CAPTURED_ENVIRONMENT_KEYS.contains(&"HOOKDOCK_TERMINAL_PROTOCOL"));
    }
}
