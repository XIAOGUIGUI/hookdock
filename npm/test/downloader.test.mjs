import test from "node:test";
import assert from "node:assert/strict";
import { parseArguments, parseChecksum } from "../lib/downloader.mjs";

test("parses the explicit download command", () => {
  const options = parseArguments(["download", "--output", "./release", "--force"], "0.1.0");
  assert.equal(options.force, true);
  assert.equal(options.version, "0.1.0");
  assert.equal(options.output.endsWith("release"), true);
});

test("requires an explicit command", () => {
  assert.throws(() => parseArguments([], "0.1.0"), /expected the "download" command/);
});

test("extracts the installer checksum", () => {
  const checksum = "a".repeat(64);
  assert.equal(parseChecksum(`${checksum}  HookDock-Setup-x64.exe\n`), checksum);
});
