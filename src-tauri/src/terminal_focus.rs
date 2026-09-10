use hookdock_protocol::{TerminalTarget, TerminalTargetKind, HOOKDOCK_TERMINAL_PROTOCOL_VERSION};
use std::process::Command;

const HOOKDOCK_TERMINAL_EXECUTABLE: &str = "hookdock-terminal.exe";

fn command_spec(target: &TerminalTarget) -> Result<(&'static str, [String; 2]), String> {
    if target.kind != TerminalTargetKind::HookDockTerminal
        || target.protocol != HOOKDOCK_TERMINAL_PROTOCOL_VERSION
    {
        return Err("不支持的终端聚焦协议".to_owned());
    }

    Ok((
        HOOKDOCK_TERMINAL_EXECUTABLE,
        ["--focus-session".to_owned(), target.session_id.clone()],
    ))
}

pub fn focus_terminal(target: &TerminalTarget) -> Result<(), String> {
    let (executable, arguments) = command_spec(target)?;

    let status = Command::new(executable)
        .args(arguments)
        .status()
        .map_err(|error| format!("无法启动 HookDock Terminal: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("HookDock Terminal 返回错误: {status}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_unknown_protocol_before_launching() {
        let target = TerminalTarget {
            kind: TerminalTargetKind::HookDockTerminal,
            session_id: "7f6b3978-25f1-4519-8b02-8fe67f35991f".to_owned(),
            protocol: HOOKDOCK_TERMINAL_PROTOCOL_VERSION + 1,
        };
        assert_eq!(focus_terminal(&target).unwrap_err(), "不支持的终端聚焦协议");
    }

    #[test]
    fn builds_the_exact_focus_session_command_without_a_shell() {
        let target = TerminalTarget {
            kind: TerminalTargetKind::HookDockTerminal,
            session_id: "7f6b3978-25f1-4519-8b02-8fe67f35991f".to_owned(),
            protocol: HOOKDOCK_TERMINAL_PROTOCOL_VERSION,
        };
        let (executable, arguments) = command_spec(&target).unwrap();

        assert_eq!(executable, "hookdock-terminal.exe");
        assert_eq!(arguments[0], "--focus-session");
        assert_eq!(arguments[1], target.session_id);
    }
}
