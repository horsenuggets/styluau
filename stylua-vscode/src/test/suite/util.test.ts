import * as assert from "assert";
import { readFileSync } from "fs";
import * as path from "path";
import { runInNewContext } from "vm";
import * as vscode from "vscode";
import {
  getAssetFilenamePatternForPlatform,
  getSetting,
  InspectedSetting,
  parseStyluauVersion,
  resolveSetting,
} from "../../util";

suite("Utilities testing", () => {
  test("falls back to legacy settings without overriding explicit values", () => {
    const empty: InspectedSetting<boolean> = {};
    assert.strictEqual(resolveSetting(empty, empty, false), false);
    assert.strictEqual(resolveSetting(empty, empty, true), true);

    // A declared default is not an explicit value, so the legacy setting still applies
    const declaredDefault: InspectedSetting<boolean> = { defaultValue: false };
    const legacy: InspectedSetting<boolean> = { workspaceValue: true };
    assert.strictEqual(resolveSetting(declaredDefault, legacy, false), true);

    // Any explicit new value wins, including `false`
    const explicit: InspectedSetting<boolean> = { workspaceValue: false };
    assert.strictEqual(resolveSetting(explicit, legacy, true), false);
    assert.strictEqual(
      resolveSetting({ defaultLanguageValue: false }, legacy, false),
      true
    );
    assert.strictEqual(
      resolveSetting<string | null>(
        { workspaceValue: null },
        { globalValue: "legacy-binary" },
        "default-binary"
      ),
      null
    );

    // More specific scopes win within each configuration
    const precedence: (keyof InspectedSetting<boolean>)[] = [
      "workspaceFolderLanguageValue",
      "workspaceLanguageValue",
      "globalLanguageValue",
      "workspaceFolderValue",
      "workspaceValue",
      "globalValue",
    ];
    for (const [index, key] of precedence.entries()) {
      const roots = precedence.slice(index + 1);
      const setting: Record<string, boolean> = { [key]: true };
      for (const root of roots) {
        setting[root] = false;
      }
      assert.strictEqual(
        resolveSetting(setting as InspectedSetting<boolean>, empty, false),
        true,
        `${key} should take precedence`
      );
    }
  });

  test("reads the renamed legacy binary path without overriding an explicit null", () => {
    const getConfiguration = vscode.workspace.getConfiguration;
    let current: InspectedSetting<string | null> = { defaultValue: null };
    vscode.workspace.getConfiguration = (section) =>
      ({
        inspect: (key: string) => {
          if (section === "styluau" && key === "styluauPath") {
            return current;
          }
          if (section === "stylua" && key === "styluaPath") {
            return { workspaceValue: "legacy-binary" };
          }
          return undefined;
        },
      } as vscode.WorkspaceConfiguration);
    try {
      assert.strictEqual(getSetting("styluauPath", null), "legacy-binary");
      current = { workspaceValue: null };
      assert.strictEqual(getSetting("styluauPath", null), null);
      current = { workspaceValue: "new-binary" };
      assert.strictEqual(getSetting("styluauPath", null), "new-binary");
    } finally {
      vscode.workspace.getConfiguration = getConfiguration;
    }
  });

  test("builds with GitHub origins, an explicit endpoint, or no origin", () => {
    const configPath = path.resolve(__dirname, "../../../webpack.config.js");
    const source = readFileSync(configPath, "utf8");
    const endpoint = "https://api.github.com/repos/example/formatter/releases";
    for (const [origin, explicit, expected] of [
      ["https://github.com/example/formatter.git", undefined, endpoint],
      ["git@github.com:example/formatter.git", undefined, endpoint],
      ["ssh://git@github.com/example/formatter.git", undefined, endpoint],
      [undefined, undefined, null],
      ["unsupported-origin", undefined, null],
      [undefined, endpoint, endpoint],
      ["https://github.com/other/formatter.git", endpoint, endpoint],
    ]) {
      const module = {
        exports: [] as { plugins: { definitions: Record<string, string> }[] }[],
      };
      runInNewContext(source, {
        // eslint-disable-next-line @typescript-eslint/naming-convention
        __dirname: path.dirname(configPath),
        module,
        process: {
          env: {
            // eslint-disable-next-line @typescript-eslint/naming-convention
            STYLUAU_RELEASES_URL: explicit,
          },
        },
        require: (id: string) => {
          if (id === "path") {
            return path;
          }
          if (id === "child_process") {
            return {
              execFileSync: () => {
                if (!origin) {
                  throw new Error("No origin");
                }
                return origin;
              },
            };
          }
          if (id === "webpack") {
            return {
              // eslint-disable-next-line @typescript-eslint/naming-convention
              DefinePlugin: class {
                constructor(public definitions: Record<string, string>) {}
              },
            };
          }
          throw new Error(`Unexpected module "${id}".`);
        },
      });
      assert.strictEqual(
        module.exports[0].plugins[0].definitions.STYLUAU_RELEASES_URL,
        JSON.stringify(expected)
      );
    }
  });

  test("parses fork versions and rejects other executables", () => {
    assert.strictEqual(
      parseStyluauVersion("styluau 2.6.0-horse.1.6\n"),
      "2.6.0-horse.1.6"
    );
    assert.strictEqual(parseStyluauVersion("stylua 2.6.0"), undefined);
    assert.strictEqual(parseStyluauVersion("unexpected output"), undefined);
  });

  test("does not select another archive or architecture", () => {
    const pattern = getAssetFilenamePatternForPlatform("linux", "x64");
    assert(pattern.test("styluau-linux-x86_64.zip"));
    assert.strictEqual(pattern.test("stylua-linux-x86_64.zip"), false);
    assert.strictEqual(pattern.test("styluau-linux-aarch64.zip"), false);
    assert.strictEqual(pattern.test("styluau-linux-x86_64-musl.zip"), false);
  });
  test("asset filename pattern matches legacy name with version", () => {
    const pattern = getAssetFilenamePatternForPlatform("win32", "x64");
    assert(pattern.test("styluau-0.12.2-win64.zip"));
  });

  test("asset filename pattern matches legacy name without version", () => {
    const pattern = getAssetFilenamePatternForPlatform("win32", "x64");
    assert(pattern.test("styluau-win64.zip"));
  });

  test("asset filename pattern matches name with version and machine", () => {
    const pattern = getAssetFilenamePatternForPlatform("win32", "x64");
    assert(pattern.test("styluau-windows-x86_64.zip"));
  });

  test("asset filename pattern does not match for wrong platform", () => {
    const pattern = getAssetFilenamePatternForPlatform("win32", "x64");
    assert.strictEqual(pattern.test("styluau-linux.zip"), false);
  });
});
