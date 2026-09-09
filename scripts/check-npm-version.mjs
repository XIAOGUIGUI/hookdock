import fs from "node:fs";
import process from "node:process";

const packageJson = JSON.parse(fs.readFileSync("npm/package.json", "utf8"));
const registry = process.env.npm_config_registry ?? "https://registry.npmjs.org/";
const packageUrl = new URL(
  `${encodeURIComponent(packageJson.name)}/${encodeURIComponent(packageJson.version)}`,
  registry.endsWith("/") ? registry : `${registry}/`,
);
const response = await fetch(packageUrl, { headers: { accept: "application/json" } });

let published;
if (response.ok) {
  published = true;
} else if (response.status === 404) {
  published = false;
} else {
  const details = (await response.text()).slice(0, 1_000);
  throw new Error(`npm registry lookup failed (${response.status}): ${details}`);
}

if (process.env.GITHUB_OUTPUT) {
  fs.appendFileSync(process.env.GITHUB_OUTPUT, `published=${published}\n`);
}
process.stdout.write(
  published
    ? `${packageJson.name}@${packageJson.version} is already published; npm publish will be skipped.\n`
    : `${packageJson.name}@${packageJson.version} is available.\n`,
);
