# HookDock

HookDock 是面向 Windows 的通知中枢，用来接管 AI 编码工具和本机自动化程序的通知。它可以接入 Claude Code、Codex、Gemini CLI、通用 stdin Hook 和带认证的本机 HTTP 请求，并将事件显示在贴边置顶面板中。

[English](README.md)

![HookDock 像素信标宠物](public/mascots/hookdock.png)

## 主要功能

- 接入或卸载 Claude Code、Codex、Gemini CLI Hook，并保留配置文件中的其他内容。
- 在来源 Hook 支持时，将权限选择和问题答案回传给调用方。
- 通过带令牌认证的 `127.0.0.1` HTTP API 接收普通通知和阻塞式交互请求。
- 分别配置各来源的名称、字符图标、颜色、Toast、声音、自动展开和启用状态。
- 将 196 × 52 的胶囊拖到任意显示器左右边缘，并记住位置。
- 跟随 Windows 语言，也可手动选择简体中文或英文。
- 最近 50 条事件保存在 `%APPDATA%\HookDock`，数据不会上传。
- 配合 HookDock Terminal，点击 Codex 通知可以返回发出事件的原始 Pane。

## 下载

可以从 GitHub Actions、带版本标签的 GitHub Release 或 npm 包获取 x64 安装包。正式发布的 npm tarball 内嵌 HookDock 和 HookDock Terminal，因此接入公司 npm 镜像后，以下命令运行时不会访问 GitHub。CLI 会先完成 SHA-256 校验，并且不会自动运行安装程序：

```powershell
npx @chenronggui/hookdock@0.1.1 download
npx @chenronggui/hookdock@0.1.1 download --output .\dist
npx @chenronggui/hookdock@0.1.1 download-terminal
npx @chenronggui/hookdock@0.1.1 download-terminal --output .\dist
```

Terminal 命令会保存一个供 Windows 11 内部测试使用的无签名 MSIX。请在管理员 PowerShell 中执行
`Add-AppxPackage .\HookDockTerminal_*_x64_unsigned.msix -AllowUnsigned`。

安装包尚未进行代码签名，因此 Windows 可能显示“未知发布者”。Terminal
MSIX 使用 Windows 11 明确支持的无签名包身份，不适合公开分发。

## 本地开发

需要 Node.js 22.12 或更高版本、Rust 和 Windows WebView2 Runtime。

```powershell
npm ci
npm run check
npm run dev
```

使用 `npm run dist:win` 生成安装包。内存较小的电脑可以打开 **Actions → HookDock Windows CI → Run workflow**，完成后下载 `HookDock-Windows-x64` artifact。

## 接入通知

在设置页可以接入支持的工具 Hook。其他程序可以把 JSON 写入随应用安装的 Bridge：

```powershell
'{"title":"构建完成","message":"所有检查已经通过","project":"demo"}' |
  & "$env:LOCALAPPDATA\HookDock\resources\hookdock-hook.exe" --source generic --event Notification
```

本机 HTTP 客户端先读取 `%APPDATA%\HookDock\runtime.json` 中的端口和令牌，然后调用 API。接口说明见 [HTTP API](docs/http-api.zh-CN.md)。

精确返回 Codex Pane 的工作方式见 [Codex 通知点击返回原 Pane](docs/codex-terminal-focus.zh-CN.md)。

HookDock 只监听回环地址，每次启动都会生成新的随机令牌。HookDock 未运行时，工具 Hook 会快速返回 `{}`，避免阻塞调用方。

## 首版限制

首版只支持 Windows 10/11 x64 和 NSIS，不包含 ARM64、MSIX、Microsoft Store、代码签名、自动更新、云端转发及条件规则引擎。全屏独占程序和 UAC 安全桌面可能暂时覆盖普通置顶窗口。

## 许可证

Apache-2.0。HookDock 包含从 [Ping Island](https://github.com/erha19/ping-island) 修改而来的源码，归属说明见 [NOTICE](NOTICE)。
