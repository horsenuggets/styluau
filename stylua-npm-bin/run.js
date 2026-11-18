#!/usr/bin/env node
"use strict";

const { spawnSync } = require("child_process");
const path = require("path");

const PLATFORMS = {
  "linux-x64":    { pkg: "styluau-bin-linux-x64",   bin: "styluau" },
  "linux-arm64":  { pkg: "styluau-bin-linux-arm64",  bin: "styluau" },
  "darwin-x64":   { pkg: "styluau-bin-darwin-x64",  bin: "styluau" },
  "darwin-arm64": { pkg: "styluau-bin-darwin-arm64", bin: "styluau" },
  "win32-x64":    { pkg: "styluau-bin-win32-x64",   bin: "styluau.exe" },
};

const key = `${process.platform}-${process.arch}`;
const platform = PLATFORMS[key];

if (!platform) {
  console.error(`Unsupported platform: ${key}`);
  process.exit(1);
}

let binPath;
try {
  const pkgDir = path.dirname(require.resolve(`${platform.pkg}/package.json`));
  binPath = path.join(pkgDir, platform.bin);
} catch {
  console.error(
    `Could not find the StyLuau binary for your platform (${key}).\n` +
    `Try reinstalling styluau-bin.`
  );
  process.exit(1);
}

const result = spawnSync(binPath, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  console.error(result.error);
}

process.exitCode = result.error ? 1 : (result.status ?? 1);
