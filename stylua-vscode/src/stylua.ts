import * as vscode from "vscode";
import { spawn, exec } from "child_process";
import { getSetting } from "./util";

export function formatCode(
  outputChannel: vscode.LogOutputChannel,
  path: string,
  code: string,
  filePath?: string,
  cwd?: string,
  startPos?: number,
  endPos?: number
): Promise<string> {
  return new Promise((resolve, reject) => {
    const args = ["--respect-ignores"];

    if (filePath) {
      args.push("--stdin-filepath");
      args.push(filePath);
    }

    if (startPos) {
      args.push("--range-start");
      args.push(startPos.toString());
    }
    if (endPos) {
      args.push("--range-end");
      args.push(endPos.toString());
    }

    const configPath = getSetting<string>("configPath", "").trim();
    if (configPath !== "") {
      args.push("--config-path");
      args.push(configPath);
    }

    if (getSetting<boolean>("searchParentDirectories", false)) {
      args.push("--search-parent-directories");
    }
    if (getSetting<boolean>("verify", false)) {
      args.push("--verify");
    }

    args.push("-");

    outputChannel.debug(`${path} ${args.join(" ")}`);

    const child = spawn(`${path}`, args, {
      cwd,
    });
    let output = "";
    child.stdout.on("data", (data) => {
      output += data.toString();
    });
    child.stdout.on("close", () => {
      resolve(output);
    });
    child.stderr.on("data", (data) => reject(data.toString()));
    child.on("error", (err) =>
      reject(`Failed to start StyLuau: ${err.message}`)
    );
    child.stdin.on("error", () => {
      // Suppress EPIPE when the process failed to spawn and stdin is already closed
    });

    child.stdin.write(code);
    child.stdin.end();
  });
}

export function executeStylua(
  path: string,
  args?: string[],
  cwd?: string
): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = exec(
      `"${path}" ${args?.join(" ") ?? ""}`,
      {
        cwd,
      },
      (err, stdout) => {
        if (err) {
          reject(err);
          return;
        }
        resolve(stdout);
      }
    );
  });
}
