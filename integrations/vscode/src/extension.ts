import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

/**
 * Resolve the syntaqlite binary path. Preference order:
 * 1. User setting `syntaqlite.serverPath`
 * 2. Bundled platform-specific binary in `server/`
 * 3. `syntaqlite` on PATH (development fallback)
 */
function resolveServerPath(context: vscode.ExtensionContext): string {
  // 1. Explicit user override
  const config = vscode.workspace.getConfiguration("syntaqlite");
  const userPath = config.get<string>("serverPath");
  if (userPath) {
    if (!fs.existsSync(userPath)) {
      throw new Error(
        `syntaqlite.serverPath points to "${userPath}" which does not exist.`,
      );
    }
    return userPath;
  }

  // 2. Bundled binary (platform-specific .vsix packaging puts it in server/)
  const bundledName =
    os.platform() === "win32" ? "syntaqlite.exe" : "syntaqlite";
  const bundledPath = path.join(
    context.extensionPath,
    "server",
    bundledName,
  );
  if (fs.existsSync(bundledPath)) {
    // Ensure executable on unix
    if (os.platform() !== "win32") {
      try {
        fs.chmodSync(bundledPath, 0o755);
      } catch {
        // ignore — may already be executable
      }
    }
    return bundledPath;
  }

  // 3. Fall back to PATH
  return "syntaqlite";
}

export async function activate(
  context: vscode.ExtensionContext,
): Promise<void> {
  const outputChannel = vscode.window.createOutputChannel("syntaqlite");

  let serverCommand: string;
  try {
    serverCommand = resolveServerPath(context);
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    void vscode.window.showErrorMessage(msg);
    outputChannel.appendLine(`Failed to resolve server path: ${msg}`);
    return;
  }

  outputChannel.appendLine(`Using server binary: ${serverCommand}`);

  const serverOptions: ServerOptions = {
    command: serverCommand,
    args: ["lsp"],
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "sql" },
      { scheme: "file", language: "sqlite" },
    ],
    outputChannel,
  };

  client = new LanguageClient(
    "syntaqlite",
    "syntaqlite",
    serverOptions,
    clientOptions,
  );

  // Register restart command
  context.subscriptions.push(
    vscode.commands.registerCommand("syntaqlite.restartServer", async () => {
      if (client) {
        outputChannel.appendLine("Restarting syntaqlite language server...");
        await client.restart();
        outputChannel.appendLine("Server restarted.");
      }
    }),
  );

  // Register format command (delegates to built-in formatDocument)
  context.subscriptions.push(
    vscode.commands.registerCommand("syntaqlite.formatDocument", async () => {
      const editor = vscode.window.activeTextEditor;
      if (editor && (editor.document.languageId === "sql" || editor.document.languageId === "sqlite")) {
        await vscode.commands.executeCommand(
          "editor.action.formatDocument",
        );
      }
    }),
  );

  try {
    await client.start();
    outputChannel.appendLine("syntaqlite language server started.");
  } catch (e: unknown) {
    const msg = e instanceof Error ? e.message : String(e);
    void vscode.window.showErrorMessage(
      `Failed to start syntaqlite language server: ${msg}. ` +
        `Make sure the \`syntaqlite\` binary is installed and on your PATH, ` +
        `or set \`syntaqlite.serverPath\` in settings.`,
    );
    outputChannel.appendLine(`Failed to start server: ${msg}`);
  }
}

export async function deactivate(): Promise<void> {
  if (client) {
    try {
      await client.stop();
    } catch {
      // Client may not be running (e.g. start failed) — ignore.
    }
    client = undefined;
  }
}
