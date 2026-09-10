import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import {
  downloadTerminalInstaller,
  parseArguments,
  parseAssetManifest,
  usage,
} from "../lib/downloader.mjs";

test("parses the explicit download command", () => {
  const options = parseArguments(["download", "--output", "./release", "--force"], "0.1.0");
  assert.equal(options.command, "download");
  assert.equal(options.force, true);
  assert.equal(options.version, "0.1.0");
  assert.equal(options.output.endsWith("release"), true);
});

test("parses the download-terminal command", () => {
  const options = parseArguments(["download-terminal", "--output", "./terminal"], "0.1.0");
  assert.equal(options.command, "download-terminal");
  assert.equal(options.force, false);
  assert.equal(options.output.endsWith("terminal"), true);
});

test("requires an explicit supported command", () => {
  assert.throws(
    () => parseArguments([], "0.1.0"),
    /expected the "download" or "download-terminal" command/,
  );
});

test("documents both downloader commands", () => {
  assert.match(usage(), /hookdock download /);
  assert.match(usage(), /hookdock download-terminal /);
});

test("parses and normalizes the bundled asset manifest", () => {
  const checksum = "A".repeat(64);
  const manifest = parseAssetManifest(JSON.stringify({
    schemaVersion: 1,
    products: {
      terminal: {
        version: "0.1.0.0",
        file: "HookDockTerminal_0.1.0.0_x64_unsigned.msix",
        sha256: checksum,
      },
    },
  }));
  assert.equal(manifest.products.terminal.sha256, checksum.toLowerCase());
});

test("rejects unsafe bundled artifact paths", () => {
  assert.throws(
    () => parseAssetManifest(JSON.stringify({
      schemaVersion: 1,
      products: {
        terminal: {
          version: "0.1.0.0",
          file: "../outside.msix",
          sha256: "a".repeat(64),
        },
      },
    })),
    /metadata for terminal is invalid/,
  );
});

test("copies a verified terminal installer without network access", async () => {
  const fixtureDirectory = await mkdtemp(path.join(os.tmpdir(), "hookdock-assets-"));
  const outputDirectory = path.join(fixtureDirectory, "output");
  const assetName = "HookDockTerminal_0.1.0.0_x64_unsigned.msix";
  const assetBytes = Buffer.from("test terminal package");
  const checksum = createHash("sha256").update(assetBytes).digest("hex");

  try {
    await writeFile(path.join(fixtureDirectory, assetName), assetBytes);
    await writeFile(
      path.join(fixtureDirectory, "manifest.json"),
      JSON.stringify({
        schemaVersion: 1,
        products: {
          terminal: {
            version: "0.1.0.0",
            file: assetName,
            sha256: checksum,
          },
        },
      }),
    );

    const options = parseArguments(["download-terminal", "--output", outputDirectory], "0.1.0");
    const result = await downloadTerminalInstaller(options, fixtureDirectory);
    assert.equal(result.version, "0.1.0.0");
    assert.equal(result.destination, path.join(outputDirectory, assetName));
    assert.deepEqual(await readFile(result.destination), assetBytes);

    await assert.rejects(
      () => downloadTerminalInstaller(options, fixtureDirectory),
      /already exists; pass --force/,
    );
  } finally {
    await rm(fixtureDirectory, { recursive: true, force: true });
  }
});

test("rejects a bundled terminal installer with the wrong checksum", async () => {
  const fixtureDirectory = await mkdtemp(path.join(os.tmpdir(), "hookdock-assets-"));
  try {
    await writeFile(path.join(fixtureDirectory, "terminal.msix"), "tampered");
    await writeFile(
      path.join(fixtureDirectory, "manifest.json"),
      JSON.stringify({
        schemaVersion: 1,
        products: {
          terminal: {
            version: "0.1.0.0",
            file: "terminal.msix",
            sha256: "0".repeat(64),
          },
        },
      }),
    );

    await assert.rejects(
      () => downloadTerminalInstaller(
        { output: fixtureDirectory, force: false },
        fixtureDirectory,
      ),
      /failed SHA-256 verification/,
    );
  } finally {
    await rm(fixtureDirectory, { recursive: true, force: true });
  }
});
