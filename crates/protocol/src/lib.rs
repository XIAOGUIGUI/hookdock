use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use uuid::Uuid;

pub const PROTOCOL_VERSION: u8 = 1;
pub const HOOKDOCK_TERMINAL_PROTOCOL_VERSION: u8 = 1;
pub const MAX_HOOK_BYTES: u64 = 2 * 1024 * 1024;
pub const DEFAULT_HOOK_PORT: u16 = 37_129;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeConfig {
    pub protocol: u8,
    pub host: String,
    pub port: u16,
    pub token: String,
    pub pid: u32,
    pub updated_at: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeRequest {
    pub protocol: u8,
    pub id: String,
    pub token: String,
    pub source: String,
    #[serde(default)]
    pub event: Option<String>,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub payload: Map<String, Value>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BridgeResponse {
    pub protocol: u8,
    pub id: String,
    pub acknowledged: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision: Option<Decision>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub answers: Option<HashMap<String, String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

impl BridgeResponse {
    pub fn acknowledged(id: impl Into<String>) -> Self {
        Self {
            protocol: PROTOCOL_VERSION,
            id: id.into(),
            acknowledged: true,
            ..Self::default()
        }
    }

    pub fn rejected(id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            protocol: PROTOCOL_VERSION,
            id: id.into(),
            acknowledged: false,
            error: Some(error.into()),
            ..Self::default()
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Decision {
    Approve,
    ApproveForSession,
    Deny,
    Answer,
    Cancel,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum EventStatus {
    Idle,
    Working,
    Notification,
    WaitingApproval,
    WaitingInput,
    Completed,
    Error,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum TerminalTargetKind {
    #[serde(rename = "hookdockTerminal")]
    HookDockTerminal,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TerminalTarget {
    pub kind: TerminalTargetKind,
    pub session_id: String,
    pub protocol: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct QuestionOption {
    pub id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookQuestion {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    pub prompt: String,
    pub options: Vec<QuestionOption>,
    pub multi_select: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_freeform: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub is_secret: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HookEvent {
    pub id: String,
    pub source: String,
    pub provider_name: String,
    pub event_type: String,
    pub session_key: String,
    pub project: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cwd: Option<String>,
    pub title: String,
    pub body: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_target: Option<TerminalTarget>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub terminal_context_observed: bool,
    pub status: EventStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_use_id: Option<String>,
    pub questions: Vec<HookQuestion>,
    pub expects_response: bool,
    pub should_notify: bool,
    pub received_at: DateTime<Utc>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<DateTime<Utc>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<Decision>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn codex_terminal_target(request: &BridgeRequest, source: &str) -> Option<TerminalTarget> {
    if source != "codex"
        || request.environment.get("HOOKDOCK_TERMINAL")?.trim() != "1"
        || request
            .environment
            .get("HOOKDOCK_TERMINAL_PROTOCOL")?
            .trim()
            .parse::<u8>()
            .ok()?
            != HOOKDOCK_TERMINAL_PROTOCOL_VERSION
    {
        return None;
    }

    let session_id = Uuid::parse_str(request.environment.get("WT_SESSION")?.trim()).ok()?;
    Some(TerminalTarget {
        kind: TerminalTargetKind::HookDockTerminal,
        session_id: session_id.hyphenated().to_string(),
        protocol: HOOKDOCK_TERMINAL_PROTOCOL_VERSION,
    })
}

fn terminal_context_observed(request: &BridgeRequest, source: &str) -> bool {
    source == "codex"
        && [
            "WT_SESSION",
            "HOOKDOCK_TERMINAL",
            "HOOKDOCK_TERMINAL_PROTOCOL",
        ]
        .iter()
        .any(|key| request.environment.contains_key(*key))
}

fn as_object(value: Option<&Value>) -> Option<&Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn non_empty(value: Option<&Value>) -> Option<String> {
    value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn first_string(values: impl IntoIterator<Item = Option<String>>) -> Option<String> {
    values.into_iter().flatten().find(|value| !value.is_empty())
}

fn argument_value(args: &[String], name: &str) -> Option<String> {
    args.iter()
        .position(|argument| argument == name)
        .and_then(|index| args.get(index + 1))
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn compact_value(value: Option<&Value>, limit: usize) -> Option<String> {
    let value = value?;
    let raw = match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(value).ok()?,
    };
    let compact = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.is_empty() {
        None
    } else if compact.chars().count() > limit {
        Some(format!(
            "{}…",
            compact
                .chars()
                .take(limit.saturating_sub(1))
                .collect::<String>()
        ))
    } else {
        Some(compact)
    }
}

fn normalize_tool_name(value: Option<&String>) -> String {
    value
        .map(|value| value.to_lowercase().replace(['_', '-'], ""))
        .unwrap_or_default()
}

fn provider_name(source: &str) -> &'static str {
    match source {
        "claude" => "Claude Code",
        "codex" => "Codex",
        "gemini" => "Gemini CLI",
        _ => "Custom Hook",
    }
}

fn project_name(cwd: Option<&String>) -> String {
    cwd.and_then(|value| value.rsplit(['\\', '/']).find(|part| !part.is_empty()))
        .unwrap_or("Unknown project")
        .to_owned()
}

fn is_codex_user_input_event(source: &str, event_type: &str) -> bool {
    if source != "codex" {
        return false;
    }
    matches!(
        event_type
            .chars()
            .filter(|character| character.is_ascii_alphanumeric())
            .flat_map(char::to_lowercase)
            .collect::<String>()
            .as_str(),
        "userinputrequest" | "requestuserinput"
    )
}

fn detect_questions(payload: &Map<String, Value>) -> Vec<HookQuestion> {
    let tool_input = as_object(payload.get("tool_input"));
    let raw_questions = tool_input
        .and_then(|input| input.get("questions"))
        .or_else(|| payload.get("questions"))
        .and_then(Value::as_array);

    raw_questions
        .into_iter()
        .flatten()
        .enumerate()
        .filter_map(|(index, entry)| {
            let question = entry.as_object()?;
            let prompt = first_string([
                non_empty(question.get("question")),
                non_empty(question.get("title")),
                non_empty(question.get("prompt")),
            ])?;
            let id = first_string([
                non_empty(question.get("id")),
                non_empty(question.get("header")),
            ])
            .unwrap_or_else(|| format!("question-{}", index + 1));
            let options = question
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
                .filter_map(|(option_index, option)| {
                    let (label, description) = match option {
                        Value::String(label) if !label.trim().is_empty() => {
                            (label.trim().to_owned(), None)
                        }
                        Value::Object(object) => (
                            first_string([
                                non_empty(object.get("label")),
                                non_empty(object.get("title")),
                                non_empty(object.get("value")),
                            ])?,
                            non_empty(object.get("description")),
                        ),
                        _ => return None,
                    };
                    Some(QuestionOption {
                        id: format!("{id}:{option_index}"),
                        label,
                        description,
                    })
                })
                .collect();
            Some(HookQuestion {
                id,
                header: non_empty(question.get("header")),
                prompt,
                options,
                multi_select: question
                    .get("multiSelect")
                    .or_else(|| question.get("multi_select"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                allow_freeform: question
                    .get("isOther")
                    .or_else(|| question.get("is_other"))
                    .or_else(|| question.get("allowFreeform"))
                    .or_else(|| question.get("allow_freeform"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                is_secret: question
                    .get("isSecret")
                    .or_else(|| question.get("is_secret"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn detect_status(
    event_type: &str,
    payload: &Map<String, Value>,
    source: &str,
    questions: &[HookQuestion],
) -> EventStatus {
    let event = event_type.to_lowercase();
    let explicit = non_empty(payload.get("status")).map(|value| value.to_lowercase());
    if explicit
        .as_deref()
        .is_some_and(|value| value.contains("error") || value.contains("fail"))
    {
        return EventStatus::Error;
    }
    if !questions.is_empty() {
        return EventStatus::WaitingInput;
    }
    if event.contains("permission") || event.contains("approval") {
        return EventStatus::WaitingApproval;
    }
    if event.contains("fail") || payload.contains_key("error") {
        return EventStatus::Error;
    }
    if matches!(event.as_str(), "stop" | "afteragent" | "sessionend") {
        return EventStatus::Completed;
    }
    if event.contains("notification") || source == "generic" {
        return EventStatus::Notification;
    }
    if event.contains("start")
        || event.contains("submit")
        || event.contains("beforeagent")
        || event.contains("tool")
    {
        return EventStatus::Working;
    }
    EventStatus::Idle
}

fn detect_title(
    provider: &str,
    event_type: &str,
    status: EventStatus,
    tool_name: Option<&String>,
    payload: &Map<String, Value>,
) -> String {
    if let Some(title) = non_empty(payload.get("title")) {
        return title;
    }
    match status {
        EventStatus::WaitingInput => format!("{provider} is waiting for an answer"),
        EventStatus::WaitingApproval => tool_name
            .map(|tool| format!("{provider} requests {tool}"))
            .unwrap_or_else(|| format!("{provider} requests an action")),
        EventStatus::Completed => format!("{provider} completed the task"),
        EventStatus::Error => format!("{provider} failed"),
        EventStatus::Notification => first_string([non_empty(payload.get("notification_title"))])
            .unwrap_or_else(|| format!("{provider} notification")),
        _ => first_string([
            non_empty(payload.get("session_title")),
            non_empty(payload.get("title")),
        ])
        .unwrap_or_else(|| format!("{provider} · {event_type}")),
    }
}

fn detect_body(
    payload: &Map<String, Value>,
    tool_name: Option<&String>,
    questions: &[HookQuestion],
    status: EventStatus,
) -> String {
    if let Some(question) = questions.first() {
        return question.prompt.clone();
    }
    let tool_input = as_object(payload.get("tool_input"));
    let common = || {
        first_string([
            non_empty(payload.get("body")),
            non_empty(payload.get("message")),
            non_empty(payload.get("last_assistant_message")),
            non_empty(payload.get("reason")),
            non_empty(payload.get("prompt")),
            compact_value(payload.get("tool_result"), 420),
            compact_value(payload.get("error"), 420),
            tool_name.cloned(),
        ])
    };
    if status == EventStatus::WaitingApproval {
        first_string([
            non_empty(payload.get("reason")),
            tool_input.and_then(|input| non_empty(input.get("command"))),
            tool_input.and_then(|input| non_empty(input.get("file_path"))),
            non_empty(payload.get("command")),
            compact_value(payload.get("tool_input"), 420),
            tool_name.cloned(),
        ])
        .unwrap_or_else(|| "The agent is waiting for your decision".to_owned())
    } else {
        common().unwrap_or_else(|| "A new Hook event arrived".to_owned())
    }
}

pub fn normalize_hook(request: &BridgeRequest, now: DateTime<Utc>) -> HookEvent {
    let source = match request.source.as_str() {
        "claude" | "codex" | "gemini" | "generic" => request.source.as_str(),
        _ => "generic",
    };
    let event_type = first_string([
        request.event.clone(),
        non_empty(request.payload.get("hook_event_name")),
        non_empty(request.payload.get("event")),
        non_empty(request.payload.get("type")),
        argument_value(&request.args, "--event"),
    ])
    .unwrap_or_else(|| {
        if request.payload.contains_key("questions") {
            "UserInputRequest".to_owned()
        } else {
            "UnknownEvent".to_owned()
        }
    });
    let cwd = first_string([
        non_empty(request.payload.get("cwd")),
        non_empty(request.payload.get("workspace")),
        request.environment.get("PWD").cloned(),
    ]);
    let raw_session_id = first_string([
        non_empty(request.payload.get("session_id")),
        non_empty(request.payload.get("sessionId")),
        non_empty(request.payload.get("thread_id")),
        non_empty(request.payload.get("threadId")),
        request.environment.get("CLAUDE_SESSION_ID").cloned(),
        request.environment.get("CODEX_THREAD_ID").cloned(),
        cwd.clone(),
    ])
    .unwrap_or_else(|| "default".to_owned());
    let tool_name = first_string([
        non_empty(request.payload.get("tool_name")),
        non_empty(request.payload.get("toolName")),
    ]);
    let questions = detect_questions(&request.payload);
    let status = detect_status(&event_type, &request.payload, source, &questions);
    let is_question_tool = matches!(
        normalize_tool_name(tool_name.as_ref()).as_str(),
        "askuserquestion" | "askfollowupquestion" | "ask"
    );
    let supports_question_answers = matches!(source, "claude" | "generic");
    let is_codex_user_input_request =
        is_codex_user_input_event(source, &event_type) && !questions.is_empty();
    let expects_response = (source != "gemini" && status == EventStatus::WaitingApproval)
        || (supports_question_answers && is_question_tool && !questions.is_empty())
        || is_codex_user_input_request;
    let provider = provider_name(source).to_owned();

    HookEvent {
        id: if request.id.trim().is_empty() {
            Uuid::new_v4().to_string()
        } else {
            request.id.clone()
        },
        source: source.to_owned(),
        provider_name: provider.clone(),
        event_type: event_type.clone(),
        session_key: format!("{source}:{raw_session_id}"),
        project: first_string([
            non_empty(request.payload.get("project")),
            Some(project_name(cwd.as_ref())),
        ])
        .unwrap_or_else(|| "Unknown project".to_owned()),
        cwd,
        title: detect_title(
            &provider,
            &event_type,
            status,
            tool_name.as_ref(),
            &request.payload,
        ),
        body: detect_body(&request.payload, tool_name.as_ref(), &questions, status),
        url: non_empty(request.payload.get("url")),
        terminal_target: codex_terminal_target(request, source),
        terminal_context_observed: terminal_context_observed(request, source),
        status,
        tool_name,
        tool_use_id: first_string([
            non_empty(request.payload.get("tool_use_id")),
            non_empty(request.payload.get("toolUseId")),
            non_empty(request.payload.get("call_id")),
            non_empty(request.payload.get("callId")),
        ]),
        questions,
        expects_response,
        should_notify: matches!(
            status,
            EventStatus::WaitingApproval
                | EventStatus::WaitingInput
                | EventStatus::Completed
                | EventStatus::Error
                | EventStatus::Notification
        ),
        received_at: now,
        resolved_at: None,
        resolution: None,
    }
}

pub fn provider_stdout(
    source: &str,
    explicit_event: Option<&str>,
    payload: &Map<String, Value>,
    response: &BridgeResponse,
) -> String {
    let Some(decision) = response.decision else {
        return "{}".to_owned();
    };
    if decision == Decision::Cancel {
        return "{}".to_owned();
    }
    let event = explicit_event
        .map(ToOwned::to_owned)
        .or_else(|| non_empty(payload.get("hook_event_name")))
        .or_else(|| non_empty(payload.get("event")))
        .or_else(|| non_empty(payload.get("type")))
        .unwrap_or_else(|| "PermissionRequest".to_owned());

    if decision == Decision::Answer {
        let provided_answers = response.answers.clone().unwrap_or_default();
        if is_codex_user_input_event(source, &event) {
            let questions = detect_questions(payload);
            if questions.is_empty() {
                return "{}".to_owned();
            }
            let mut answers = Map::new();
            for question in questions {
                let Some(answer) = provided_answers
                    .get(&question.id)
                    .map(|answer| answer.trim())
                    .filter(|answer| !answer.is_empty())
                else {
                    return "{}".to_owned();
                };
                let values = if question.multi_select {
                    answer
                        .split(", ")
                        .map(str::trim)
                        .filter(|value| !value.is_empty())
                        .map(ToOwned::to_owned)
                        .collect::<Vec<_>>()
                } else {
                    vec![answer.to_owned()]
                };
                if values.is_empty() {
                    return "{}".to_owned();
                }
                answers.insert(question.id, json!({ "answers": values }));
            }
            return json!({ "answers": answers }).to_string();
        }
        let mut answers = HashMap::new();
        let raw_questions = as_object(payload.get("tool_input"))
            .and_then(|input| input.get("questions"))
            .or_else(|| payload.get("questions"))
            .and_then(Value::as_array);
        for (key, value) in provided_answers {
            let output_key = raw_questions
                .into_iter()
                .flatten()
                .enumerate()
                .find_map(|(index, question)| {
                    let question = question.as_object()?;
                    let id = first_string([
                        non_empty(question.get("id")),
                        non_empty(question.get("header")),
                    ])
                    .unwrap_or_else(|| format!("question-{}", index + 1));
                    (id == key).then(|| {
                        first_string([
                            non_empty(question.get("question")),
                            non_empty(question.get("prompt")),
                            non_empty(question.get("id")),
                        ])
                        .unwrap_or_else(|| key.clone())
                    })
                })
                .unwrap_or(key);
            answers.insert(output_key, value);
        }
        if source == "codex" {
            return json!({ "answers": answers }).to_string();
        }
        let mut updated_input = as_object(payload.get("tool_input"))
            .cloned()
            .unwrap_or_default();
        updated_input.insert("answers".to_owned(), json!(answers));
        if event.contains("Question") || event == "UserInputRequest" || event == "UserPromptSubmit"
        {
            return json!({
                "hookSpecificOutput": {
                    "hookEventName": event,
                    "permissionDecision": "allow",
                    "updatedInput": updated_input
                }
            })
            .to_string();
        }
        return json!({
            "hookSpecificOutput": {
                "hookEventName": event,
                "decision": {
                    "behavior": "allow",
                    "updatedInput": updated_input
                }
            }
        })
        .to_string();
    }

    let allow = matches!(decision, Decision::Approve | Decision::ApproveForSession);
    if source == "codex" && event != "PermissionRequest" {
        return match decision {
            Decision::ApproveForSession => json!({ "decision": "acceptForSession" }).to_string(),
            Decision::Approve => json!({ "decision": "accept" }).to_string(),
            _ => json!({ "decision": "decline" }).to_string(),
        };
    }

    let mut provider_decision = json!({ "behavior": if allow { "allow" } else { "deny" } });
    if !allow {
        provider_decision["message"] = Value::String(
            response
                .reason
                .clone()
                .unwrap_or_else(|| "Denied from HookDock".to_owned()),
        );
    }
    json!({
        "hookSpecificOutput": {
            "hookEventName": "PermissionRequest",
            "decision": provider_decision
        }
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request(source: &str, payload: Value) -> BridgeRequest {
        BridgeRequest {
            protocol: PROTOCOL_VERSION,
            id: "request-1".to_owned(),
            token: "token".to_owned(),
            source: source.to_owned(),
            event: None,
            args: vec![],
            environment: HashMap::new(),
            payload: payload.as_object().cloned().unwrap(),
        }
    }

    #[test]
    fn maps_claude_permission_request() {
        let event = normalize_hook(
            &request(
                "claude",
                json!({
                    "hook_event_name": "PermissionRequest",
                    "session_id": "abc",
                    "cwd": "C:\\work\\demo",
                    "tool_name": "PowerShell",
                    "tool_input": { "command": "npm test" }
                }),
            ),
            Utc::now(),
        );
        assert_eq!(event.project, "demo");
        assert_eq!(event.status, EventStatus::WaitingApproval);
        assert!(event.expects_response);
        assert_eq!(event.body, "npm test");
    }

    #[test]
    fn gemini_notifications_never_block() {
        let event = normalize_hook(
            &request(
                "gemini",
                json!({ "event": "PermissionRequest", "message": "waiting" }),
            ),
            Utc::now(),
        );
        assert!(!event.expects_response);
    }

    #[test]
    fn codex_question_payload_does_not_claim_hook_response_support() {
        let event = normalize_hook(
            &request(
                "codex",
                json!({
                    "hook_event_name": "PermissionRequest",
                    "tool_name": "AskUserQuestion",
                    "tool_input": { "questions": [{ "id": "choice", "question": "Choose one" }] }
                }),
            ),
            Utc::now(),
        );
        assert!(!event.expects_response);
    }

    #[test]
    fn codex_user_input_request_is_blocking_and_preserves_question_metadata() {
        let event = normalize_hook(
            &request(
                "codex",
                json!({
                    "hook_event_name": "UserInputRequest",
                    "tool_name": "request_user_input",
                    "call_id": "call-123",
                    "questions": [{
                        "id": "environment",
                        "header": "Environment",
                        "question": "Where should I deploy?",
                        "options": [{ "label": "Staging", "description": "Safe test environment" }],
                        "isOther": true,
                        "isSecret": true
                    }]
                }),
            ),
            Utc::now(),
        );
        assert_eq!(event.status, EventStatus::WaitingInput);
        assert!(event.expects_response);
        assert_eq!(event.tool_use_id.as_deref(), Some("call-123"));
        assert_eq!(event.questions[0].header.as_deref(), Some("Environment"));
        assert!(event.questions[0].allow_freeform);
        assert!(event.questions[0].is_secret);
    }

    #[test]
    fn formats_codex_user_input_answers() {
        let payload = json!({
            "hook_event_name": "UserInputRequest",
            "questions": [
                { "id": "environment", "question": "Where?", "options": [] },
                { "id": "features", "question": "Which?", "multiSelect": true }
            ]
        })
        .as_object()
        .cloned()
        .unwrap();
        let response = BridgeResponse {
            decision: Some(Decision::Answer),
            answers: Some(HashMap::from([
                ("environment".to_owned(), "Staging".to_owned()),
                ("features".to_owned(), "Logs, Metrics".to_owned()),
            ])),
            ..BridgeResponse::acknowledged("id")
        };
        let output: Value =
            serde_json::from_str(&provider_stdout("codex", None, &payload, &response)).unwrap();
        assert_eq!(
            output,
            json!({
                "answers": {
                    "environment": { "answers": ["Staging"] },
                    "features": { "answers": ["Logs", "Metrics"] }
                }
            })
        );
    }

    #[test]
    fn incomplete_codex_user_input_answers_fail_open() {
        let payload = json!({
            "hook_event_name": "UserInputRequest",
            "questions": [{ "id": "environment", "question": "Where?" }]
        })
        .as_object()
        .cloned()
        .unwrap();
        let response = BridgeResponse {
            decision: Some(Decision::Answer),
            answers: Some(HashMap::new()),
            ..BridgeResponse::acknowledged("id")
        };
        assert_eq!(provider_stdout("codex", None, &payload, &response), "{}");
    }

    #[test]
    fn captures_a_marked_hookdock_terminal_session_for_codex() {
        let mut request = request("codex", json!({ "thread_id": "thread-1" }));
        request.environment.extend([
            ("HOOKDOCK_TERMINAL".to_owned(), "1".to_owned()),
            ("HOOKDOCK_TERMINAL_PROTOCOL".to_owned(), "1".to_owned()),
            (
                "WT_SESSION".to_owned(),
                "{7F6B3978-25F1-4519-8B02-8FE67F35991F}".to_owned(),
            ),
        ]);

        let event = normalize_hook(&request, Utc::now());
        let target = event.terminal_target.unwrap();
        assert!(event.terminal_context_observed);
        assert_eq!(target.kind, TerminalTargetKind::HookDockTerminal);
        assert_eq!(target.protocol, HOOKDOCK_TERMINAL_PROTOCOL_VERSION);
        assert_eq!(target.session_id, "7f6b3978-25f1-4519-8b02-8fe67f35991f");
    }

    #[test]
    fn ignores_unmarked_or_invalid_terminal_sessions() {
        let mut unmarked = request("codex", json!({ "thread_id": "thread-1" }));
        unmarked.environment.insert(
            "WT_SESSION".to_owned(),
            "7f6b3978-25f1-4519-8b02-8fe67f35991f".to_owned(),
        );
        assert!(normalize_hook(&unmarked, Utc::now())
            .terminal_target
            .is_none());
        assert!(normalize_hook(&unmarked, Utc::now()).terminal_context_observed);

        let mut invalid = unmarked;
        invalid
            .environment
            .insert("HOOKDOCK_TERMINAL".to_owned(), "1".to_owned());
        invalid
            .environment
            .insert("HOOKDOCK_TERMINAL_PROTOCOL".to_owned(), "1".to_owned());
        invalid
            .environment
            .insert("WT_SESSION".to_owned(), "not-a-guid".to_owned());
        assert!(normalize_hook(&invalid, Utc::now())
            .terminal_target
            .is_none());

        invalid.source = "claude".to_owned();
        invalid.environment.insert(
            "WT_SESSION".to_owned(),
            "7f6b3978-25f1-4519-8b02-8fe67f35991f".to_owned(),
        );
        assert!(normalize_hook(&invalid, Utc::now())
            .terminal_target
            .is_none());
    }

    #[test]
    fn extracts_question_options() {
        let event = normalize_hook(
            &request(
                "claude",
                json!({
                    "hook_event_name": "PermissionRequest",
                    "tool_name": "AskUserQuestion",
                    "tool_input": { "questions": [{
                        "header": "stack",
                        "question": "Choose a stack",
                        "options": [{ "label": "React", "description": "Web UI" }]
                    }]}
                }),
            ),
            Utc::now(),
        );
        assert_eq!(event.status, EventStatus::WaitingInput);
        assert_eq!(event.questions[0].id, "stack");
        assert_eq!(event.questions[0].options[0].label, "React");
    }

    #[test]
    fn formats_codex_session_approval() {
        let response = BridgeResponse {
            decision: Some(Decision::ApproveForSession),
            ..BridgeResponse::acknowledged("id")
        };
        assert_eq!(
            provider_stdout("codex", Some("PreToolUse"), &Map::new(), &response),
            r#"{"decision":"acceptForSession"}"#
        );
    }

    #[test]
    fn claude_answer_preserves_questions_and_uses_prompt_as_answer_key() {
        let payload = json!({
            "hook_event_name": "PermissionRequest",
            "tool_input": {
                "questions": [{ "header": "stack", "question": "Choose a stack" }]
            }
        })
        .as_object()
        .cloned()
        .unwrap();
        let response = BridgeResponse {
            decision: Some(Decision::Answer),
            answers: Some(HashMap::from([("stack".to_owned(), "React".to_owned())])),
            ..BridgeResponse::acknowledged("id")
        };
        let output: Value = serde_json::from_str(&provider_stdout(
            "claude",
            Some("PermissionRequest"),
            &payload,
            &response,
        ))
        .unwrap();
        assert_eq!(
            output["hookSpecificOutput"]["decision"]["updatedInput"]["questions"][0]["question"],
            "Choose a stack"
        );
        assert_eq!(
            output["hookSpecificOutput"]["decision"]["updatedInput"]["answers"]["Choose a stack"],
            "React"
        );
    }
}
