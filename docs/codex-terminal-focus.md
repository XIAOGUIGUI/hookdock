# Codex notification → exact terminal pane

HookDock can return to the exact Codex pane when Codex is running inside the
companion HookDock Terminal build.

The terminal gives every ConPTY child the standard `WT_SESSION` GUID plus two
capability markers:

```text
HOOKDOCK_TERMINAL=1
HOOKDOCK_TERMINAL_PROTOCOL=1
```

The hook bridge includes those values in its local authenticated request.
HookDock accepts a terminal target only for Codex, only with both markers, and
only when `WT_SESSION` parses as a GUID. The newest known target is retained per
Codex thread in the 50-event local history so events with no terminal context
can reuse it. Seeing an unmarked or invalid terminal context clears that reuse,
so resuming the thread in another terminal cannot jump back to the old pane.

On Windows, HookDock keeps the native toast activation handle. Clicking the
toast runs this argument vector directly, without `cmd.exe` or PowerShell:

```text
hookdock-terminal.exe
--focus-session
<WT_SESSION>
```

HookDock Terminal searches its live windows and pane trees by connection
session ID, then selects the containing tab, focuses the pane, and summons the
window. Moving a tab or pane does not invalidate the target. If the companion
alias is unavailable, HookDock opens its own event panel instead. A stale GUID
is a safe no-op in the terminal and never creates a new tab.

No Codex source patch is required: Codex notify hooks inherit the pane's
environment, and the existing HookDock bridge already runs in that environment.

The companion source, build instructions, package identity, App Installer
template, and weekly Microsoft Stable synchronization workflow live in the
sibling `hookdock-terminal` repository.
