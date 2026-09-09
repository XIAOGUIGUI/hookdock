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
try {
  const install = spawnSync(
    npmCommand,
    ["install", "--ignore-scripts", "--no-audit", "--no-fund", "--prefix", smokeDirectory, path.join(releaseDirectory, tarballs[0])],
    { stdio: "inherit" },
  );
  if (install.status !== 0) process.exit(install.status ?? 1);

  const executable = path.join(
    smokeDirectory,
    "node_modules",
    ".bin",
    process.platform === "win32" ? "hookdock.cmd" : "hookdock",
  );
  const help = spawnSync(executable, ["--help"], { encoding: "utf8" });
  if (help.status !== 0 || !help.stdout.includes("Download the verified HookDock Windows installer")) {
    process.stderr.write(help.stdout ?? "");
    process.stderr.write(help.stderr ?? "");
    throw new Error("Packed HookDock CLI smoke test failed");
  }
  process.stdout.write(help.stdout);
} finally {
  fs.rmSync(smokeDirectory, { recursive: true, force: true });
}
