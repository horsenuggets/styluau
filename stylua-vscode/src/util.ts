// Based off https://github.com/Kampfkarren/selene/blob/master/selene-vscode/src/util.ts
// Licensed under https://github.com/Kampfkarren/selene/blob/master/LICENSE.md
import * as vscode from "vscode";
import * as os from "os";

export const parseStyluauVersion = (output: string): string | undefined => {
  return /^styluau\s+(\S+)$/i.exec(output.trim())?.[1];
};

export const getDownloadOutputFilename = () => {
  switch (os.platform()) {
    case "win32":
      return "styluau.exe";
    case "linux":
    case "darwin":
      return "styluau";
    default:
      throw new Error("platform not supported");
  }
};

export const getAssetFilenamePatternForPlatform = (
  platform: string,
  machine: string
) => {
  var platformPattern: string;
  switch (platform) {
    case "win32":
      platformPattern = "(windows|win64)";
      break;
    case "linux":
      platformPattern = "linux";
      break;
    case "darwin":
      platformPattern = "macos";
      break;
    default:
      throw new Error("platform not supported");
  }

  var archPattern: string;
  switch (machine) {
    case "arm64":
      archPattern = "aarch64";
      break;
    case "x64":
      archPattern = "x86_64";
      break;
    default:
      archPattern = "";
  }

  return new RegExp(
    "^styluau(-[\\w\\-.]+)?-" +
      platformPattern +
      "(-" +
      archPattern +
      ")?\\.zip$"
  );
};

export const getAssetFilenamePattern = () => {
  return getAssetFilenamePatternForPlatform(os.platform(), process.arch);
};

export interface InspectedSetting<T> {
  defaultValue?: T;
  globalValue?: T;
  workspaceValue?: T;
  workspaceFolderValue?: T;
  defaultLanguageValue?: T;
  globalLanguageValue?: T;
  workspaceLanguageValue?: T;
  workspaceFolderLanguageValue?: T;
}

// Mirrors VS Code's precedence for explicit values, ignoring declared defaults so a legacy
// value can win over the new setting's default.
const explicitValue = <T>(
  setting: InspectedSetting<T> | undefined
): T | undefined => {
  for (const value of [
    setting?.workspaceFolderLanguageValue,
    setting?.workspaceLanguageValue,
    setting?.globalLanguageValue,
    setting?.workspaceFolderValue,
    setting?.workspaceValue,
    setting?.globalValue,
  ]) {
    if (value !== undefined) {
      return value;
    }
  }
  return undefined;
};

/**
 * Picks a setting value from the StyLuau and legacy StyLua configurations, preferring an
 * explicitly set new value. Declared defaults are ignored so a legacy value can still apply.
 */
export const resolveSetting = <T>(
  current: InspectedSetting<T> | undefined,
  legacy: InspectedSetting<T> | undefined,
  defaultValue: T
): T => {
  const currentValue = explicitValue(current);
  if (currentValue !== undefined) {
    return currentValue;
  }
  const legacyValue = explicitValue(legacy);
  return legacyValue !== undefined ? legacyValue : defaultValue;
};

/**
 * Reads a StyLuau setting, falling back to a legacy `stylua.*` value the user set
 * explicitly. New settings always take precedence.
 */
export const getSetting = <T>(key: string, defaultValue: T): T =>
  resolveSetting(
    vscode.workspace.getConfiguration("styluau").inspect<T>(key),
    vscode.workspace
      .getConfiguration("stylua")
      .inspect<T>(key === "styluauPath" ? "styluaPath" : key),
    defaultValue
  );

export const getDesiredVersion = (): string => {
  const targetVersion = getSetting<string>("targetReleaseVersion", "").trim();
  if (targetVersion.length === 0) {
    // TODO: Backwards compatibility to support deprecated setting `styluau.releaseVersion`
    return getSetting<string>("releaseVersion", "latest");
  }
  return targetVersion;
};

export const fileExists = (path: vscode.Uri | string): Thenable<boolean> => {
  const uri = path instanceof vscode.Uri ? path : vscode.Uri.file(path);
  return vscode.workspace.fs.stat(uri).then(
    () => true,
    () => false
  );
};
