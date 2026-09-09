# HookDock

HookDock is a Windows notification hub for AI coding tools and local automation. It receives Claude Code, Codex, Gemini CLI, generic stdin hooks, and authenticated localhost HTTP requests, then presents them in an always-on-top edge panel.

[简体中文](README.zh-CN.md)

![HookDock pixel beacon](public/mascots/hookdock.png)

## Features

- Connect or remove Claude Code, Codex, and Gemini CLI hooks without replacing unrelated configuration.
- Forward approval and question responses when the provider hook supports them.
- Accept notifications and blocking requests over an authenticated `127.0.0.1` HTTP API.
- Customize each source's name, glyph, accent color, toast, sound, auto-expand, and enabled state.
- Snap a compact 196 × 52 capsule to either edge of any monitor and remember its position.
- Follow the Windows display language or switch between English and Simplified Chinese.
- Keep the most recent 50 events locally under `%APPDATA%\HookDock`.

## Download

Download the x64 NSIS installer from GitHub Actions or a tagged GitHub Release. The npm downloader fetches the release matching its own version, verifies SHA-256, and does not run the installer:

```powershell
npx @chenronggui/hookdock@0.1.0 download
npx @chenronggui/hookdock@0.1.0 download --output .\dist
```

The initial installer is unsigned, so Windows SmartScreen may identify it as coming from an unknown publisher.

## Local development

Requires Node.js 22.12 or newer, Rust, and the Windows WebView2 runtime.

```powershell
npm ci
npm run check
npm run dev
```

Build the installer with `npm run dist:win`. Low-memory machines can run **Actions → HookDock Windows CI → Run workflow** and download the `HookDock-Windows-x64` artifact instead.

## Integrations

Open Settings to connect the supported provider hooks. A generic stdin hook can send JSON to the bundled bridge:

```powershell
'{"title":"Build complete","message":"All checks passed","project":"demo"}' |
  & "$env:LOCALAPPDATA\HookDock\resources\hookdock-hook.exe" --source generic --event Notification
```

For local HTTP clients, read `%APPDATA%\HookDock\runtime.json`, then use its port and token. See [HTTP API](docs/http-api.md).

HookDock only listens on loopback and creates a new random token for every app launch. If HookDock is closed, provider hook commands return `{}` quickly so the calling tool can continue its normal flow.

## Current limits

The first release supports Windows 10/11 x64 and NSIS only. It does not include ARM64, MSIX, Microsoft Store distribution, code signing, automatic updates, cloud relay, or a conditional rules engine. Exclusive fullscreen apps and the Windows UAC secure desktop can temporarily cover an always-on-top window.

## License

Apache-2.0. HookDock contains modified source from [Ping Island](https://github.com/erha19/ping-island); see [NOTICE](NOTICE) for attribution.
