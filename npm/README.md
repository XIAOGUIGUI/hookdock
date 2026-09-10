# @chenronggui/hookdock

Copies the HookDock and HookDock Terminal Windows x64 installers bundled inside
this npm package. It never contacts GitHub at runtime, so it works through a
corporate npm mirror on an isolated network. The CLI verifies each bundled
installer's SHA-256 checksum and never runs it.

Every download also writes `HookDock-安装与使用说明.md` beside the installer.
That offline Chinese guide covers installation, first-time setup, Codex pane
activation, corporate npm mirrors, upgrades, removal, and troubleshooting.

```bash
npx @chenronggui/hookdock@0.1.2 download
npx @chenronggui/hookdock@0.1.2 download --output ./dist
npx @chenronggui/hookdock@0.1.2 download-terminal
npx @chenronggui/hookdock@0.1.2 download-terminal --output ./dist
```

Use `--force` to replace an existing installer. Install the unsigned
HookDock Terminal package from an elevated Windows 11 PowerShell:

    Add-AppxPackage .\HookDockTerminal_*_x64_unsigned.msix -AllowUnsigned
