import { spawnSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const windowsDirectory = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");

const build = spawnSync("cargo", ["build", "--release", "--package", "hookdock-hook"], {
  cwd: windowsDirectory,
  stdio: "inherit",
});
if (build.status !== 0) process.exit(build.status ?? 1);

const sourceName = process.platform === "win32" ? "hookdock-hook.exe" : "hookdock-hook";
const source = path.join(windowsDirectory, "target", "release", sourceName);
const destinationDirectory = path.join(windowsDirectory, "src-tauri", "resources");
fs.mkdirSync(destinationDirectory, { recursive: true });
fs.copyFileSync(source, path.join(destinationDirectory, "hookdock-hook.exe"));
