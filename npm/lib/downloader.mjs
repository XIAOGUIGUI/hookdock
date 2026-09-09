import { createHash } from "node:crypto";
import { createWriteStream, readFileSync } from "node:fs";
import { access, mkdir, readFile, rename, rm } from "node:fs/promises";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

const OWNER = "XIAOGUIGUI";
const REPOSITORY = "hookdock";
const ASSET_NAME = "HookDock-Setup-x64.exe";
const CHECKSUM_NAME = "checksums.txt";

export function usage() {
  return [
    "Download the verified HookDock Windows installer.",
    "",
    "Usage:",
    "  npx @chenronggui/hookdock download [--output <directory>] [--force]",
  ].join("\n");
}

export function parseArguments(argumentsList, version = packageVersion()) {
  if (argumentsList.includes("--help") || argumentsList.includes("-h")) {
    return { help: true, force: false, output: process.cwd(), version };
  }
  if (argumentsList[0] !== "download") {
    throw new Error(`expected the \"download\" command\n\n${usage()}`);
  }
  let output = process.cwd();
  let force = false;
  for (let index = 1; index < argumentsList.length; index += 1) {
    const argument = argumentsList[index];
    if (argument === "--force") {
      force = true;
    } else if (argument === "--output") {
      const directory = argumentsList[index + 1];
      if (!directory) throw new Error("--output requires a directory");
      output = path.resolve(directory);
      index += 1;
    } else {
      throw new Error(`unknown option: ${argument}`);
    }
  }
  return { help: false, force, output, version };
}

export function parseChecksum(contents, assetName = ASSET_NAME) {
  for (const line of contents.split(/\r?\n/)) {
    const match = line.trim().match(/^([a-fA-F0-9]{64})\s+\*?(.+)$/);
    if (match && match[2] === assetName) return match[1].toLowerCase();
  }
  throw new Error(`checksum for ${assetName} was not found`);
}

export async function downloadInstaller(options) {
  const tag = `v${options.version}`;
  const releaseRoot = `https://github.com/${OWNER}/${REPOSITORY}/releases/download/${tag}`;
  const destinationDirectory = path.resolve(options.output);
  const destination = path.join(destinationDirectory, ASSET_NAME);
  const temporary = `${destination}.download-${process.pid}`;
  await mkdir(destinationDirectory, { recursive: true });
  if (!options.force && await exists(destination)) {
    throw new Error(`${destination} already exists; pass --force to replace it`);
  }

  const checksumResponse = await fetch(`${releaseRoot}/${CHECKSUM_NAME}`, { redirect: "follow" });
  if (!checksumResponse.ok) throw new Error(`could not fetch checksums (${checksumResponse.status})`);
  const expectedChecksum = parseChecksum(await checksumResponse.text());

  const installerResponse = await fetch(`${releaseRoot}/${ASSET_NAME}`, { redirect: "follow" });
  if (!installerResponse.ok || !installerResponse.body) {
    throw new Error(`could not download installer (${installerResponse.status})`);
  }

  try {
    await pipeline(Readable.fromWeb(installerResponse.body), createWriteStream(temporary, { flags: "wx" }));
    const actualChecksum = createHash("sha256").update(await readFile(temporary)).digest("hex");
    if (actualChecksum !== expectedChecksum) {
      throw new Error("downloaded installer failed SHA-256 verification");
    }
    if (options.force) await rm(destination, { force: true });
    await rename(temporary, destination);
    return destination;
  } catch (error) {
    await rm(temporary, { force: true });
    throw error;
  }
}

async function exists(target) {
  try {
    await access(target);
    return true;
  } catch {
    return false;
  }
}

function packageVersion() {
  const packagePath = fileURLToPath(new URL("../package.json", import.meta.url));
  return JSON.parse(readFileSync(packagePath, "utf8")).version;
}
