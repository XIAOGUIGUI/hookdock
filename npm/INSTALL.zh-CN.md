# HookDock 安装与使用说明

这份说明随 `@chenronggui/hookdock` npm 包发布。下载命令会把它复制到安装包旁边，阅读时不需要访问 GitHub。

## 1. 环境要求

- Windows 11 x64。
- Node.js 18.17 或更高版本，用于从 npm 或公司 npm 镜像提取安装包。
- 安装 HookDock Terminal 时需要管理员 PowerShell。

HookDock Terminal 当前是供内部使用的无签名 MSIX。Windows 10 不支持这里采用的 `-AllowUnsigned` 安装方式。

## 2. 从 npm 下载文件

如果公司有 npm 镜像，先确认它已经同步或上传了 `@chenronggui/hookdock`：

```powershell
npm config get registry
npm view @chenronggui/hookdock@0.1.2 version
```

在准备保存安装包的目录中执行：

```powershell
npx --yes @chenronggui/hookdock@0.1.2 download
npx --yes @chenronggui/hookdock@0.1.2 download-terminal
```

也可以指定输出目录：

```powershell
npx --yes @chenronggui/hookdock@0.1.2 download --output .\dist
npx --yes @chenronggui/hookdock@0.1.2 download-terminal --output .\dist
```

这两个命令只从 npm 包内部复制文件，不会在运行时访问 GitHub，也不会自动执行安装程序。每个安装包都会先经过 SHA-256 校验。文件已存在时可加 `--force` 覆盖。

下载完成后应看到：

- `HookDock-Setup-x64.exe`
- `HookDockTerminal_0.1.0.0_x64_unsigned.msix`
- `HookDock-安装与使用说明.md`

## 3. 安装 HookDock

在 PowerShell 中启动安装程序：

```powershell
Start-Process .\HookDock-Setup-x64.exe
```

安装包目前没有代码签名，Windows 可能显示“未知发布者”。如果公司安全策略禁止运行未签名程序，需要由管理员放行或以后改用公司证书签名；不要绕过公司的终端安全策略。

安装完成后从开始菜单启动 **HookDock**。

## 4. 安装 HookDock Terminal

用“以管理员身份运行”打开 PowerShell，进入 MSIX 所在目录，然后执行：

```powershell
$package = Get-ChildItem .\HookDockTerminal_*_x64_unsigned.msix | Select-Object -First 1
Add-AppxPackage -Path $package.FullName -AllowUnsigned
```

安装后从开始菜单启动 **HookDock Terminal**。也可以检查命令别名：

```powershell
Get-Command hookdock-terminal.exe
```

HookDock Terminal 与微软商店版 Windows Terminal 使用不同的包身份，可以并存。

## 5. 配置 Codex Hook

1. 启动 HookDock。
2. 打开 HookDock 设置。
3. 在 Hook 管理中找到 **Codex**，点击安装。
4. HookDock 只管理包含 `hookdock-hook.exe` 的条目；已有的其他 Codex 配置和 Hook 会保留。
5. 关闭旧的 Codex 终端会话，然后在 HookDock Terminal 的新 Pane 中启动 Codex。

HookDock 运行时会把本机监听地址和随机令牌写入：

```text
%APPDATA%\HookDock\runtime.json
```

服务只监听 `127.0.0.1`。HookDock 未启动时，bridge 会快速返回 `{}`，Codex 可以继续使用自己的原生流程。

## 6. 点击通知返回原 Pane

只有在 **HookDock Terminal** 中启动的 Codex Pane 才能精确跳转。Terminal 会自动给 Pane 中的进程注入：

```powershell
$env:HOOKDOCK_TERMINAL
$env:HOOKDOCK_TERMINAL_PROTOCOL
$env:WT_SESSION
```

当 Codex Hook 发出通知时，bridge 会把合法的 `WT_SESSION` GUID 一起交给 HookDock。点击 Windows 通知后，HookDock Terminal 会在所有窗口、Tab 和 Pane 中按这个 GUID 查找并聚焦原 Pane。Tab 被拖动、重排或移动到另一个窗口后，GUID 仍然有效。

如果通知来自普通 Windows Terminal、PowerShell 窗口或旧 Pane，HookDock 不会冒充精确匹配；它会打开 HookDock 的事件详情作为降级行为。

## 7. Codex `request_user_input` 兼容性

- 支持接收 Codex Hook 的权限、完成和生命周期事件。
- 支持点击通知返回 HookDock Terminal 中的原 Pane。
- HookDock 已支持专用的阻塞事件 `UserInputRequest`，可以显示选项、自定义输入和敏感输入，并把答案回传给 Codex。
- 官方 Codex 0.153.4 还不会发出这个外部事件；需要安装实现了 `UserInputRequest` 契约的 Codex 版本。未修改的 Codex 会继续在原 Pane 中显示问题，不影响其他通知和点击跳转能力。
- HookDock 不可用、用户取消或回答无效时，定制 Codex 必须回退到自己的原生提问界面。

## 8. 更新和卸载

下载新版本时使用新的 npm 版本号，并按需加 `--force`。新版本 MSIX 的版本号更高时，再执行一次 `Add-AppxPackage -AllowUnsigned` 即可更新。

卸载 HookDock：打开 Windows 的“设置 → 应用 → 已安装的应用”。

卸载 HookDock Terminal：

```powershell
Get-AppxPackage HookDock.Terminal | Remove-AppxPackage
```

如不再使用 Codex 集成，建议先在 HookDock 设置中卸载 Codex Hook，以便只移除 HookDock 管理的配置条目。

## 9. 常见问题

### npm 返回 404

公司镜像尚未同步 scoped package。请让镜像管理员同步或上传 `@chenronggui/hookdock`，并确认 `npm view @chenronggui/hookdock version` 能返回版本。

### `Add-AppxPackage` 拒绝安装

确认系统是 Windows 11、PowerShell 已用管理员身份运行、命令带有 `-AllowUnsigned`，并确认公司策略允许无签名内部 MSIX。

### HookDock 收不到事件

确认 HookDock 正在运行、`%APPDATA%\HookDock\runtime.json` 存在，并在设置中重新安装对应 Hook。修改 Hook 后请新开一个 Codex 会话。

### 点击通知没有返回原 Pane

确认 Codex 是在 HookDock Terminal 的新 Pane 中启动，并执行：

```powershell
$env:HOOKDOCK_TERMINAL
$env:HOOKDOCK_TERMINAL_PROTOCOL
$env:WT_SESSION
```

前三项都应有值，其中 `WT_SESSION` 应是 GUID。普通 Windows Terminal 不提供 HookDock 的两个能力标记，不能使用精确 Pane 跳转。
