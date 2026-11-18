import * as vscode from "vscode";
import * as unzip from "unzipper";
import * as util from "./util";
import * as semver from "semver";
import fetch from "node-fetch";
import { createWriteStream } from "fs";
import { executeStylua } from "./stylua";
import { GitHub, GitHubRelease } from "./github";
import which = require("which");

export enum ResolveMode {
  configuration = "configuration",
  path = "PATH",
  bundled = "bundled",
}

export interface StyluaInfo {
  path: string;
  resolveMode: ResolveMode;
  version?: string | undefined;
}

const getStyluaVersion = async (path: string, cwd?: string) => {
  try {
    const currentVersion = (
      await executeStylua(path, ["--version"], cwd)
    )?.trim();
    return util.parseStyluauVersion(currentVersion);
  } catch (err) {
    return undefined;
  }
};

export class StyluaDownloader implements vscode.Disposable {
  statusBarUpdateItem = vscode.window.createStatusBarItem(
    "styluau.installUpdate",
    vscode.StatusBarAlignment.Right
  );

  constructor(
    private readonly storageDirectory: vscode.Uri,
    private readonly github: GitHub,
    private readonly outputChannel: vscode.LogOutputChannel
  ) {}

  public async findStylua(cwd?: string): Promise<StyluaInfo> {
    // 1) If `styluau.styluauPath` has been specified, use that directly
    const settingPath = vscode.workspace
      .getConfiguration("styluau")
      .get<string | null>("styluauPath");
    if (settingPath) {
      this.outputChannel.info(
        `StyLuau path explicitly configured: ${settingPath}`
      );
      return { path: settingPath, resolveMode: ResolveMode.configuration };
    }

    // 2) Find a `styluau` binary available on PATH
    if (
      vscode.workspace
        .getConfiguration("styluau")
        .get<boolean>("searchBinaryInPATH")
    ) {
      this.outputChannel.info("Searching for styluau on PATH");
      const resolvedPath = await which("styluau", { nothrow: true });
      if (resolvedPath) {
        this.outputChannel.info(`StyLuau found on PATH: ${resolvedPath}`);
        if (await getStyluaVersion(resolvedPath, cwd)) {
          return { path: resolvedPath, resolveMode: ResolveMode.path };
        } else {
          this.outputChannel.error(
            "StyLuau binary found on PATH failed to execute or returned an invalid version"
          );
        }
      }
    }

    // 3) Fallback to bundled styluau version
    this.outputChannel.info("Falling back to bundled StyLuau version");
    const downloadPath = vscode.Uri.joinPath(
      this.storageDirectory,
      util.getDownloadOutputFilename()
    );
    return { path: downloadPath.fsPath, resolveMode: ResolveMode.bundled };
  }

  public async ensureStyluaExists(
    cwd?: string
  ): Promise<StyluaInfo | undefined> {
    const styluau = await this.findStylua(cwd);

    if (styluau.resolveMode === ResolveMode.bundled) {
      if (!(await util.fileExists(styluau.path))) {
        await vscode.workspace.fs.createDirectory(this.storageDirectory);
        await this.downloadStyLuauVisual(util.getDesiredVersion());
      }
      styluau.version = await getStyluaVersion(styluau.path, cwd);

      // Check bundled version matches requested version
      const desiredVersion = util.getDesiredVersion();
      if (styluau.version && desiredVersion !== "latest") {
        const desiredVersionSemver = semver.parse(desiredVersion);
        const styluauVersionSemver = semver.parse(styluau.version);
        if (
          desiredVersionSemver &&
          styluauVersionSemver &&
          semver.neq(desiredVersionSemver, styluauVersionSemver)
        ) {
          this.openIncorrectVersionPrompt(styluau.version, desiredVersion);
        }
      }

      // Check for latest version
      if (
        !vscode.workspace.getConfiguration("styluau").get("disableVersionCheck")
      ) {
        try {
          const latestRelease = await this.github.getRelease("latest");
          if (
            styluau.version !==
            (latestRelease.tagName.startsWith("v")
              ? latestRelease.tagName.substr(1)
              : latestRelease.tagName)
          ) {
            this.showUpdateAvailable(latestRelease);
          } else {
            this.statusBarUpdateItem.hide();
          }
        } catch (err) {
          vscode.window.showWarningMessage(
            `Error checking the selected StyLuau version, falling back to the currently installed version:\n${err}`
          );
          if (!this.github.authenticated) {
            const option = await vscode.window.showInformationMessage(
              "Authenticating with GitHub can fix rate limits.",
              "Authenticate with GitHub"
            );
            switch (option) {
              case "Authenticate with GitHub":
                if (await this.github.authenticate()) {
                  return this.ensureStyluaExists(cwd);
                }
            }
          }
        }
      } else {
        this.statusBarUpdateItem.hide();
      }
    } else if (styluau.resolveMode === ResolveMode.configuration) {
      if (!(await util.fileExists(styluau.path))) {
        vscode.window.showErrorMessage(
          `The path given for StyLuau (${styluau.path}) does not exist`
        );
        return;
      }
      styluau.version = await getStyluaVersion(styluau.path, cwd);
    } else if (styluau.resolveMode === ResolveMode.path) {
      styluau.version = await getStyluaVersion(styluau.path, cwd);
    }

    return styluau;
  }

  public downloadStyLuauVisual(version: string): Thenable<void> {
    return vscode.window.withProgress(
      {
        cancellable: false,
        location: vscode.ProgressLocation.Notification,
        title: `Downloading StyLuau (${version})`,
      },
      () => this.downloadStylua(version)
    );
  }

  private async downloadStylua(version: string): Promise<void> {
    const release = await this.github.getRelease(version);
    const assetFilename = util.getAssetFilenamePattern();
    const outputFilename = util.getDownloadOutputFilename();

    for (const asset of release.assets) {
      if (assetFilename.test(asset.name)) {
        const file = createWriteStream(
          vscode.Uri.joinPath(this.storageDirectory, outputFilename).fsPath,
          {
            mode: 0o755,
          }
        );

        return new Promise(async (resolve, reject) => {
          fetch(asset.downloadUrl, {
            headers: {
              // eslint-disable-next-line @typescript-eslint/naming-convention
              "User-Agent": "styluau-vscode",
            },
          })
            .then((res) => res.body.pipe(unzip.Parse()))
            .then((stream) => {
              stream.on("entry", (entry: unzip.Entry) => {
                if (entry.path !== outputFilename) {
                  entry.autodrain();
                  return;
                }

                entry.pipe(file).on("finish", resolve).on("error", reject);
              });
            });
        });
      }
    }
    throw new Error(
      `No compatible StyLuau archive found in release ${release.tagName}`
    );
  }

  private showUpdateAvailable(release: GitHubRelease) {
    this.statusBarUpdateItem.name = "StyLuau Update";
    this.statusBarUpdateItem.text = `StyLuau update available (${release.tagName}) $(cloud-download)`;
    this.statusBarUpdateItem.tooltip = "Click to update StyLuau";
    this.statusBarUpdateItem.command = {
      title: "Update StyLuau",
      command: "styluau.installUpdate",
      arguments: [release],
    };
    this.statusBarUpdateItem.backgroundColor = new vscode.ThemeColor(
      "statusBarItem.warningBackground"
    );
    this.statusBarUpdateItem.show();
  }

  private openIncorrectVersionPrompt(
    currentVersion: string,
    requestedVersion: string
  ) {
    vscode.window
      .showInformationMessage(
        `The currently installed version of StyLuau (${currentVersion}) does not match the requested version (${requestedVersion})`,
        "Install"
      )
      .then((option) => {
        switch (option) {
          case "Install":
            vscode.commands.executeCommand("styluau.reinstall");
            break;
        }
      });
  }

  dispose() {
    this.statusBarUpdateItem.dispose();
  }
}
