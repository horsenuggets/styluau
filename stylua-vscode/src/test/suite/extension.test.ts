import * as assert from "assert";
import { tmpdir } from "os";
import * as vscode from "vscode";
import { StyluaDownloader } from "../../download";
import { activate } from "../../extension";

suite("Extension Test Suite", () => {
  vscode.window.showInformationMessage("Start all tests.");

  test("registers recovery commands when the initial download fails", async () => {
    const ensureStyluaExists = StyluaDownloader.prototype.ensureStyluaExists;
    const showErrorMessage = vscode.window.showErrorMessage;
    const createOutputChannel = vscode.window.createOutputChannel;
    const errors: string[] = [];
    let outputShown = false;
    const subscriptions: vscode.Disposable[] = [];
    const context = {
      globalStorageUri: vscode.Uri.file(tmpdir()),
      subscriptions,
    } as unknown as vscode.ExtensionContext;
    StyluaDownloader.prototype.ensureStyluaExists = async () => {
      throw new Error("No compatible archive");
    };
    vscode.window.createOutputChannel = (() =>
      ({
        dispose: () => {},
        error: (message: string) => errors.push(message),
        info: () => {},
        show: () => {
          outputShown = true;
        },
      } as unknown as vscode.LogOutputChannel)) as typeof vscode.window.createOutputChannel;
    vscode.window.showErrorMessage = ((message: string) => {
      errors.push(message);
      return Promise.resolve(undefined);
    }) as typeof vscode.window.showErrorMessage;
    try {
      await activate(context);
      const commands = await vscode.commands.getCommands(true);
      assert.ok(commands.includes("styluau.reinstall"));
      assert.ok(commands.includes("styluau.selectVersion"));
      assert.ok(commands.includes("styluau.showOutputChannel"));
      assert.ok(
        errors.some((message) => message.includes("No compatible archive"))
      );
      await vscode.commands.executeCommand("styluau.showOutputChannel");
      assert.strictEqual(outputShown, true);
    } finally {
      for (const subscription of subscriptions.reverse()) {
        subscription.dispose();
      }
      StyluaDownloader.prototype.ensureStyluaExists = ensureStyluaExists;
      vscode.window.showErrorMessage = showErrorMessage;
      vscode.window.createOutputChannel = createOutputChannel;
    }
  });
});
