# @xiaoguigui/hookdock

Downloads the HookDock Windows x64 installer from the GitHub Release that exactly matches this package version. The CLI verifies the release SHA-256 checksum and never runs the installer.

```bash
npx @xiaoguigui/hookdock@0.1.0 download
npx @xiaoguigui/hookdock@0.1.0 download --output ./dist
```

Use `--force` to replace an existing `HookDock-Setup-x64.exe`.
