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
const npmAssetsDirectory = path.join(npmDirectory, "assets");
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

fs.rmSync(npmAssetsDirectory, { recursive: true, force: true });
fs.mkdirSync(npmAssetsDirectory, { recursive: true });
const bundledInstallerPath = path.join(npmAssetsDirectory, installerName);
fs.copyFileSync(installerPath, bundledInstallerPath);

const npmPackage = JSON.parse(fs.readFileSync(path.join(npmDirectory, "package.json"), "utf8"));
const bundledProducts = {
  hookdock: assetMetadata(bundledInstallerPath, npmPackage.version),
};

const terminalAssetSource = process.env.HOOKDOCK_TERMINAL_ASSET;
if (terminalAssetSource) {
  const terminalVersion = process.env.HOOKDOCK_TERMINAL_VERSION;
  if (!/^\d+\.\d+\.\d+\.\d+$/.test(terminalVersion ?? "")) {
    throw new Error("HOOKDOCK_TERMINAL_VERSION must use A.B.C.D");
  }
  if (!fs.statSync(terminalAssetSource).isFile()) {
    throw new Error(`HookDock Terminal asset is not a file: ${terminalAssetSource}`);
  }
  const terminalName = path.basename(terminalAssetSource);
  if (!/^HookDockTerminal_\d+\.\d+\.\d+\.\d+_x64_unsigned\.msix$/.test(terminalName)) {
    throw new Error(`Unexpected HookDock Terminal asset name: ${terminalName}`);
  }
  const bundledTerminalPath = path.join(npmAssetsDirectory, terminalName);
  fs.copyFileSync(terminalAssetSource, bundledTerminalPath);
  bundledProducts.terminal = assetMetadata(bundledTerminalPath, terminalVersion);
}

fs.writeFileSync(
  path.join(npmAssetsDirectory, "manifest.json"),
  `${JSON.stringify({ schemaVersion: 1, products: bundledProducts }, null, 2)}\n`,
);

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
const packEntries = Array.isArray(packResult) ? packResult : Object.values(packResult);
if (packEntries.length !== 1 || !packEntries[0].filename) {
  throw new Error("npm pack did not return exactly one package");
}

const tarballPath = path.join(releaseDirectory, packEntries[0].filename);
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

function assetMetadata(assetPath, version) {
  return {
    version,
    file: path.basename(assetPath),
    sha256: createHash("sha256").update(fs.readFileSync(assetPath)).digest("hex"),
  };
}
