#!/usr/bin/env node

import process from "node:process";
import {
  downloadInstaller,
  downloadTerminalInstaller,
  parseArguments,
  usage,
} from "../lib/downloader.mjs";

try {
  const options = parseArguments(process.argv.slice(2));
  if (options.help) {
    process.stdout.write(`${usage()}\n`);
  } else if (options.command === "download-terminal") {
    const result = await downloadTerminalInstaller(options);
    process.stdout.write(`HookDock Terminal ${result.version} downloaded to ${result.destination}\n`);
  } else {
    const destination = await downloadInstaller(options);
    process.stdout.write(`HookDock ${options.version} downloaded to ${destination}\n`);
  }
} catch (error) {
  process.stderr.write(`hookdock: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
