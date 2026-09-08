#!/usr/bin/env node

import process from "node:process";
import { downloadInstaller, parseArguments, usage } from "../lib/downloader.mjs";

try {
  const options = parseArguments(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(`${usage()}\n`);
  } else {
    const destination = await downloadInstaller(options);
    process.stdout.write(`HookDock ${options.version} downloaded to ${destination}\n`);
  }
} catch (error) {
  process.stderr.write(`hookdock: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
