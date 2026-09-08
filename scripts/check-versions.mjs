import fs from "node:fs";
import process from "node:process";

const desktopPackage = JSON.parse(fs.readFileSync("package.json", "utf8"));
const tauriConfig = JSON.parse(fs.readFileSync("src-tauri/tauri.conf.json", "utf8"));
const downloaderPackage = JSON.parse(fs.readFileSync("npm/package.json", "utf8"));
const cargoManifest = fs.readFileSync("Cargo.toml", "utf8");
const cargoVersion = cargoManifest.match(/^version\s*=\s*"([^"]+)"/m)?.[1];
const versions = new Map([
  ["desktop package", desktopPackage.version],
  ["Tauri", tauriConfig.version],
  ["npm downloader", downloaderPackage.version],
  ["Cargo workspace", cargoVersion],
]);
const expected = desktopPackage.version;
const mismatches = [...versions].filter(([, version]) => version !== expected);
if (mismatches.length > 0) {
  for (const [name, version] of versions) process.stderr.write(`${name}: ${version ?? "missing"}\n`);
  process.exit(1);
}

const tag = process.env.GITHUB_REF_TYPE === "tag" ? process.env.GITHUB_REF_NAME : undefined;
if (tag && tag !== `v${expected}`) {
  process.stderr.write(`release tag ${tag} does not match version v${expected}\n`);
  process.exit(1);
}
process.stdout.write(`HookDock versions match: ${expected}\n`);
