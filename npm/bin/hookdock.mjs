#!/usr/bin/env node

import process from "node:process";
import path from "node:path";
import {
  downloadInstaller,
  downloadTerminalInstaller,
  INSTALLATION_GUIDE_NAME,
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
    process.stdout.write(`Installation and usage guide: ${result.guideDestination}\n`);
  } else {
    const destination = await downloadInstaller(options);
    process.stdout.write(`HookDock ${options.version} downloaded to ${destination}\n`);
    process.stdout.write(`Installation and usage guide: ${path.join(options.output, INSTALLATION_GUIDE_NAME)}\n`);
  }
} catch (error) {
  process.stderr.write(`hookdock: ${error instanceof Error ? error.message : String(error)}\n`);
  process.exitCode = 1;
}
