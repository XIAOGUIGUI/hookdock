import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import fs from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const nsisDirectory = path.join(root, "target", "release", "bundle", "nsis");
const releaseDirectory = path.join(root, "release");
const npmDirectory = path.join(root, "npm");
const installerName = "HookDock-Setup-x64.exe";

const installers = fs
  .readdirSync(nsisDirectory, { withFileTypes: true })
  .filter((entry) => entry.isFile() && entry.name.toLowerCase().endsWith(".exe"));

if (installers.length !== 1) {
  throw new Error(`Expected one NSIS installer in ${nsisDirectory}, found ${installers.length}`);
}

fs.mkdirSync(releaseDirectory, { recursive: true });
for (const entry of fs.readdirSync(releaseDirectory, { withFileTypes: true })) {
  if (
    entry.isFile() &&
    (entry.name === installerName ||
      entry.name === "checksums.txt" ||
      entry.name === "manifest.json" ||
      entry.name.endsWith(".tgz"))
  ) {
    fs.rmSync(path.join(releaseDirectory, entry.name));
  }
}

const installerPath = path.join(releaseDirectory, installerName);
fs.copyFileSync(path.join(nsisDirectory, installers[0].name), installerPath);

const npmCommand = process.platform === "win32" ? "npm.cmd" : "npm";
const packed = spawnSync(
  npmCommand,
  ["pack", "--json", "--pack-destination", releaseDirectory],
  { cwd: npmDirectory, encoding: "utf8", shell: process.platform === "win32" },
);
if (packed.error) throw packed.error;
if (packed.status !== 0) {
  process.stderr.write(packed.stdout ?? "");
  process.stderr.write(packed.stderr ?? "");
  process.exit(packed.status ?? 1);
}

const packResult = JSON.parse(packed.stdout);
if (!Array.isArray(packResult) || packResult.length !== 1 || !packResult[0].filename) {
  throw new Error("npm pack did not return exactly one package");
}

const npmPackage = JSON.parse(fs.readFileSync(path.join(npmDirectory, "package.json"), "utf8"));
const tarballPath = path.join(releaseDirectory, packResult[0].filename);
const assets = [installerPath, tarballPath].map((assetPath) => ({
  file: path.basename(assetPath),
  sha256: createHash("sha256").update(fs.readFileSync(assetPath)).digest("hex"),
}));

fs.writeFileSync(
  path.join(releaseDirectory, "checksums.txt"),
  `${assets.map((asset) => `${asset.sha256}  ${asset.file}`).join("\n")}\n`,
  "ascii",
);
fs.writeFileSync(
  path.join(releaseDirectory, "manifest.json"),
  `${JSON.stringify(
    {
      version: npmPackage.version,
      npmPackage: npmPackage.name,
      npmTarball: path.basename(tarballPath),
      assets,
    },
    null,
    2,
  )}\n`,
);

if (process.env.GITHUB_OUTPUT) {
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `npm_tarball=${tarballPath}\n`);
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `installer=${installerPath}\n`);
}

process.stdout.write(`Staged ${assets.map((asset) => asset.file).join(", ")}\n`);
