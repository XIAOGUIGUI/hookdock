# Codex 通知点击返回原 Pane

当 Codex 运行在配套的 HookDock Terminal 中时，点击 HookDock 的 Windows
通知可以返回发出该事件的原始 Pane。

HookDock Terminal 会给每个 ConPTY 子进程注入标准 `WT_SESSION` GUID，以及
两个能力标记：

```text
HOOKDOCK_TERMINAL=1
HOOKDOCK_TERMINAL_PROTOCOL=1
```

Hook bridge 会通过现有的本机认证连接携带这些值。HookDock 只在来源为
Codex、两个标记都正确、并且 `WT_SESSION` 是合法 GUID 时接受终端目标。
同一个 Codex thread 最新的目标会随最近 50 条本地事件保存，后续事件即使
没有终端上下文也能复用。如果 Hook 明确携带了无标记或无效的终端上下文，
则不会复用旧目标，避免 thread 换到另一个终端后又跳回原 Pane。

在 Windows 上，HookDock 会保留系统 Toast 的点击句柄。点击通知后，它不经
`cmd.exe` 或 PowerShell，直接启动以下参数：

```text
hookdock-terminal.exe
--focus-session
<WT_SESSION>
```

HookDock Terminal 会按连接的 session ID 遍历当前所有窗口、Tab 和 Pane，
然后选中对应 Tab、聚焦原 Pane，并唤起所在窗口。Tab/Panes 被拖动或重排后，
目标仍然有效。若系统找不到 companion alias，HookDock 会改为打开自己的事件
面板；GUID 已过期时终端安全地不执行操作，不会新建 Tab 或误选其他 Pane。

不需要修改 Codex 源码：Codex 的 notify hook 会继承 Pane 环境，而 HookDock
现有 bridge 本来就在这个环境中运行。

配套源码、构建说明、独立包身份、App Installer 模板和每周同步微软 Stable
版本的工作流都放在相邻的 `hookdock-terminal` 仓库。
