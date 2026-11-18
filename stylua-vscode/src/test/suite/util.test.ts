import * as assert from "assert";
import {
  getAssetFilenamePatternForPlatform,
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

    // More specific scopes win within each configuration
    const precedence: (keyof InspectedSetting<boolean>)[] = [
      "workspaceFolderLanguageValue",
      "workspaceFolderValue",
      "workspaceLanguageValue",
      "workspaceValue",
      "globalLanguageValue",
      "globalValue",
      "defaultLanguageValue",
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
