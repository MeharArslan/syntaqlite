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
let schemaStatusItem: vscode.StatusBarItem | undefined;

/**
 * Update the status bar item to show syntaqlite config status.
 */
function updateStatusItem(workspaceRoot: string | undefined): void {
  if (!schemaStatusItem) return;
  if (workspaceRoot) {
    const configPath = path.join(workspaceRoot, "syntaqlite.toml");
    if (fs.existsSync(configPath)) {
      schemaStatusItem.text = "$(database) syntaqlite.toml";
      schemaStatusItem.tooltip = `syntaqlite config: ${configPath} (click to open)`;
      schemaStatusItem.show();
      return;
    }
  }
  schemaStatusItem.text = "$(database) No config";
  schemaStatusItem.tooltip =
    "syntaqlite: No syntaqlite.toml found. Create one to configure schemas and formatting.";
  schemaStatusItem.show();
}

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

  const workspaceRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;

  // Build LSP args. Pass --config if we found a syntaqlite.toml so the server
  // doesn't have to rely on its cwd (which VS Code doesn't guarantee).
  const lspArgs: string[] = [];
  if (workspaceRoot) {
    const configPath = path.join(workspaceRoot, "syntaqlite.toml");
    if (fs.existsSync(configPath)) {
      lspArgs.push("--config", configPath);
    }
  }
  lspArgs.push("lsp");

  const serverOptions: ServerOptions = {
    command: serverCommand,
    args: lspArgs,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "sql" },
      { scheme: "file", language: "sqlite" },
      { scheme: "untitled", language: "sql" },
      { scheme: "untitled", language: "sqlite" },
    ],
    outputChannel,
  };

  client = new LanguageClient(
    "syntaqlite",
    "syntaqlite",
    serverOptions,
    clientOptions,
  );
  schemaStatusItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100,
  );
  schemaStatusItem.command = "syntaqlite.openConfig";
  updateStatusItem(workspaceRoot);
  context.subscriptions.push(schemaStatusItem);

  // Register open config command
  context.subscriptions.push(
    vscode.commands.registerCommand("syntaqlite.openConfig", async () => {
      const wsRoot = vscode.workspace.workspaceFolders?.[0]?.uri.fsPath;
      if (!wsRoot) {
        void vscode.window.showInformationMessage(
          "No workspace folder open.",
        );
        return;
      }
      const configPath = path.join(wsRoot, "syntaqlite.toml");
      if (fs.existsSync(configPath)) {
        const uri = vscode.Uri.file(configPath);
        await vscode.window.showTextDocument(uri);
      } else {
        void vscode.window.showInformationMessage(
          "No syntaqlite.toml found. Create one in your project root to configure schemas and formatting.",
        );
      }
    }),
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

  // Show/hide status bar based on active editor language
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor && (editor.document.languageId === "sql" || editor.document.languageId === "sqlite")) {
        updateStatusItem(workspaceRoot);
      } else {
        schemaStatusItem?.hide();
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
