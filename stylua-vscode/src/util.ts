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
    "^styluau(-[\\w\\-.]+)?-" + platformPattern + "(-" + archPattern + ")?\\.zip$"
  );
};

export const getAssetFilenamePattern = () => {
  return getAssetFilenamePatternForPlatform(os.platform(), process.arch);
};

export const getDesiredVersion = (): string => {
  const config = vscode.workspace.getConfiguration("styluau");
  const targetVersion = config.get<string>("targetReleaseVersion", "").trim();
  if (targetVersion.length === 0) {
    // TODO: Backwards compatibility to support deprecated setting `styluau.releaseVersion`
    return config.get<string>("releaseVersion", "latest");
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
