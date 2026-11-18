// .vscode-test.js
const { defineConfig } = require("@vscode/test-cli");
const { mkdtempSync, rmSync } = require("node:fs");
const { tmpdir } = require("node:os");
const { join } = require("node:path");

const userData = mkdtempSync(join(tmpdir(), "styluau-test-"));
process.on("exit", () => rmSync(userData, { recursive: true, force: true }));

module.exports = defineConfig({
  files: "out/test/**/*.test.js",
  launchArgs: ["--user-data-dir", userData],
});
