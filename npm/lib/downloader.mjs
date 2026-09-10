import { createHash } from "node:crypto";
import { constants, readFileSync } from "node:fs";
import { access, copyFile, mkdir, readFile, rename, rm } from "node:fs/promises";
import path from "node:path";
import process from "node:process";
import { fileURLToPath } from "node:url";

const ASSET_MANIFEST_NAME = "manifest.json";
export const INSTALLATION_GUIDE_NAME = "HookDock-安装与使用说明.md";
const HOOKDOCK_PRODUCT = "hookdock";
const TERMINAL_PRODUCT = "terminal";
const DEFAULT_ASSETS_DIRECTORY = fileURLToPath(new URL("../assets", import.meta.url));
const DEFAULT_GUIDE_PATH = fileURLToPath(new URL("../INSTALL.zh-CN.md", import.meta.url));

export function usage() {
  return [
    "Download verified HookDock Windows installers from this npm package.",
    "",
    "Usage:",
    "  npx @chenronggui/hookdock download [--output <directory>] [--force]",
    "  npx @chenronggui/hookdock download-terminal [--output <directory>] [--force]",
  ].join("\n");
}

export function parseArguments(argumentsList, version = packageVersion()) {
  if (argumentsList.includes("--help") || argumentsList.includes("-h")) {
    return { help: true, force: false, output: process.cwd(), version };
  }
  const command = argumentsList[0];
  if (command !== "download" && command !== "download-terminal") {
    throw new Error(`expected the \"download\" or \"download-terminal\" command\n\n${usage()}`);
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
  return { command, help: false, force, output, version };
}

export function parseAssetManifest(contents) {
  let manifest;
  try {
    manifest = JSON.parse(contents);
  } catch {
    throw new Error("bundled installer manifest is not valid JSON");
  }
  if (manifest?.schemaVersion !== 1 || typeof manifest.products !== "object") {
    throw new Error("bundled installer manifest has an unsupported schema");
  }

  for (const [product, artifact] of Object.entries(manifest.products)) {
    if (
      typeof artifact?.version !== "string" ||
      typeof artifact?.file !== "string" ||
      path.basename(artifact.file) !== artifact.file ||
      !/^[a-fA-F0-9]{64}$/.test(artifact?.sha256 ?? "")
    ) {
      throw new Error(`bundled installer metadata for ${product} is invalid`);
    }
    artifact.sha256 = artifact.sha256.toLowerCase();
  }
  return manifest;
}

export async function downloadInstaller(
  options,
  assetsDirectory = DEFAULT_ASSETS_DIRECTORY,
  guideSource = DEFAULT_GUIDE_PATH,
) {
  const result = await downloadBundledProduct(
    HOOKDOCK_PRODUCT,
    options,
    assetsDirectory,
    guideSource,
  );
  return result.destination;
}

export async function downloadTerminalInstaller(
  options,
  assetsDirectory = DEFAULT_ASSETS_DIRECTORY,
  guideSource = DEFAULT_GUIDE_PATH,
) {
  return downloadBundledProduct(TERMINAL_PRODUCT, options, assetsDirectory, guideSource);
}

async function downloadBundledProduct(product, options, assetsDirectory, guideSource) {
  let manifest;
  try {
    manifest = parseAssetManifest(
      await readFile(path.join(assetsDirectory, ASSET_MANIFEST_NAME), "utf8"),
    );
  } catch (error) {
    if (error?.code === "ENOENT") {
      throw new Error("this npm package does not contain bundled Windows installers");
    }
    throw error;
  }

  const artifact = manifest.products[product];
  if (!artifact) {
    throw new Error(`this npm package does not contain the ${product} installer`);
  }

  const source = path.join(assetsDirectory, artifact.file);
  let sourceBytes;
  try {
    sourceBytes = await readFile(source);
  } catch (error) {
    if (error?.code === "ENOENT") {
      throw new Error(`bundled installer ${artifact.file} is missing`);
    }
    throw error;
  }
  const actualChecksum = createHash("sha256").update(sourceBytes).digest("hex");
  if (actualChecksum !== artifact.sha256) {
    throw new Error(`bundled ${artifact.file} failed SHA-256 verification`);
  }

  const destinationDirectory = path.resolve(options.output);
  const destination = path.join(destinationDirectory, artifact.file);
  const guideDestination = path.join(destinationDirectory, INSTALLATION_GUIDE_NAME);
  const temporary = `${destination}.copy-${process.pid}`;
  await mkdir(destinationDirectory, { recursive: true });
  if (!options.force && await exists(destination)) {
    throw new Error(`${destination} already exists; pass --force to replace it`);
  }

  try {
    await copyFile(source, temporary, constants.COPYFILE_EXCL);
    if (options.force) await rm(destination, { force: true });
    await rename(temporary, destination);
    await copyInstallationGuide(guideSource, guideDestination, options.force);
    return { destination, guideDestination, version: artifact.version };
  } catch (error) {
    await rm(temporary, { force: true });
    throw error;
  }
}

async function copyInstallationGuide(source, destination, force) {
  if (!force && await exists(destination)) return;

  const temporary = `${destination}.copy-${process.pid}`;
  try {
    await copyFile(source, temporary, constants.COPYFILE_EXCL);
    if (force) await rm(destination, { force: true });
    await rename(temporary, destination);
  } catch (error) {
    await rm(temporary, { force: true });
    if (error?.code === "ENOENT") {
      throw new Error("this npm package does not contain the offline installation guide");
    }
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
