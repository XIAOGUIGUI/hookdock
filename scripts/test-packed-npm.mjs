import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const releaseDirectory = path.join(root, "release");
const tarballs = fs.readdirSync(releaseDirectory).filter((name) => name.endsWith(".tgz"));
if (tarballs.length !== 1) {
  throw new Error(`Expected one npm tarball in ${releaseDirectory}, found ${tarballs.length}`);
}

const smokeDirectory = fs.mkdtempSync(path.join(os.tmpdir(), "hookdock-npm-smoke-"));
const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const guideName = "HookDock-安装与使用说明.md";
try {
  const install = spawnSync(
    npmCommand,
    ["install", "--ignore-scripts", "--no-audit", "--no-fund", "--prefix", smokeDirectory, path.join(releaseDirectory, tarballs[0])],
    { stdio: "inherit", shell: process.platform === "win32" },
  );
  if (install.error) throw install.error;
  if (install.status !== 0) process.exit(install.status ?? 1);

  const executable = path.join(
    smokeDirectory,
    "node_modules",
    ".bin",
    process.platform === "win32" ? "hookdock.cmd" : "hookdock",
  );
  const help = spawnSync(executable, ["--help"], {
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  if (help.error) throw help.error;
  if (
    help.status !== 0 ||
    !help.stdout.includes("Download verified HookDock Windows installers") ||
    !help.stdout.includes("download-terminal")
  ) {
    process.stderr.write(help.stdout ?? "");
    process.stderr.write(help.stderr ?? "");
    throw new Error("Packed HookDock CLI smoke test failed");
  }
  process.stdout.write(help.stdout);

  const downloadDirectory = path.join(smokeDirectory, "downloads");
  const download = spawnSync(executable, ["download", "--output", downloadDirectory], {
    encoding: "utf8",
    shell: process.platform === "win32",
  });
  if (
    download.error ||
    download.status !== 0 ||
    !fs.existsSync(path.join(downloadDirectory, "HookDock-Setup-x64.exe")) ||
    !fs.existsSync(path.join(downloadDirectory, guideName))
  ) {
    process.stderr.write(download.stdout ?? "");
    process.stderr.write(download.stderr ?? "");
    throw download.error ?? new Error("Packed HookDock installer download failed");
  }

  if (process.env.HOOKDOCK_TERMINAL_ASSET) {
    const terminalDownload = spawnSync(
      executable,
      ["download-terminal", "--output", downloadDirectory],
      { encoding: "utf8", shell: process.platform === "win32" },
    );
    const expectedTerminal = path.join(downloadDirectory, path.basename(process.env.HOOKDOCK_TERMINAL_ASSET));
    if (
      terminalDownload.error ||
      terminalDownload.status !== 0 ||
      !fs.existsSync(expectedTerminal) ||
      !terminalDownload.stdout.includes(guideName)
    ) {
      process.stderr.write(terminalDownload.stdout ?? "");
      process.stderr.write(terminalDownload.stderr ?? "");
      throw terminalDownload.error ?? new Error("Packed HookDock Terminal download failed");
    }
  }
} finally {
  fs.rmSync(smokeDirectory, { recursive: true, force: true });
}
