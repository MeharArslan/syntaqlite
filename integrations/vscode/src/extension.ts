import * as vscode from "vscode";
import * as path from "path";
import * as fs from "fs";
import * as os from "os";
import { minimatch } from "minimatch";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;
let lastSentSchemaPath: string | undefined;
let schemaStatusItem: vscode.StatusBarItem | undefined;

interface SchemaEntry {
  schema: string;
  files: string[];
}

/**
 * Resolve which schema path applies to a given document URI.
 *
 * Checks `syntaqlite.schemas` entries first (glob matching against workspace-
 * relative path), then falls back to `syntaqlite.schemaPath`.
 */
function resolveSchemaForUri(uri: vscode.Uri): string {
  const config = vscode.workspace.getConfiguration("syntaqlite");
  const schemas = config.get<SchemaEntry[]>("schemas", []);

  if (schemas.length > 0) {
    const workspaceFolder = vscode.workspace.getWorkspaceFolder(uri);
    const relativePath = workspaceFolder
      ? path.relative(workspaceFolder.uri.fsPath, uri.fsPath)
      : uri.fsPath;

    for (const entry of schemas) {
      for (const pattern of entry.files) {
        if (minimatch(relativePath, pattern)) {
          // Resolve schema path relative to workspace root if not absolute.
          if (path.isAbsolute(entry.schema)) {
            return entry.schema;
          }
          if (workspaceFolder) {
            return path.join(workspaceFolder.uri.fsPath, entry.schema);
          }
          return entry.schema;
        }
      }
    }
  }

  return config.get<string>("schemaPath", "");
}

/**
 * Send a didChangeConfiguration notification if the resolved schema for the
 * given URI differs from the last one sent.
 */
function sendSchemaIfChanged(
  uri: vscode.Uri,
  outputChannel: vscode.OutputChannel,
): void {
  if (!client) return;

  const resolved = resolveSchemaForUri(uri);
  if (resolved === lastSentSchemaPath) return;

  lastSentSchemaPath = resolved;
  void client.sendNotification("workspace/didChangeConfiguration", {
    settings: { schemaPath: resolved },
  });
  updateSchemaStatusItem(resolved);
  outputChannel.appendLine(
    `Schema path changed: ${resolved || "(cleared)"}`,
  );
}

/**
 * Update the status bar item with the current schema path.
 */
function updateSchemaStatusItem(schemaPath: string): void {
  if (!schemaStatusItem) return;
  if (schemaPath) {
    schemaStatusItem.text = `$(database) ${path.basename(schemaPath)}`;
    schemaStatusItem.tooltip = `syntaqlite schema: ${schemaPath} (click to open)`;
    schemaStatusItem.show();
  } else {
    schemaStatusItem.text = "$(database) No schema";
    schemaStatusItem.tooltip = "syntaqlite: No schema configured (click to configure)";
    schemaStatusItem.show();
  }
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

  const serverOptions: ServerOptions = {
    command: serverCommand,
    args: ["lsp"],
  };

  // Resolve initial schema from the active editor if available, else fall back
  // to the simple schemaPath setting.
  const activeUri = vscode.window.activeTextEditor?.document.uri;
  const initialSchemaPath = activeUri
    ? resolveSchemaForUri(activeUri)
    : vscode.workspace.getConfiguration("syntaqlite").get<string>("schemaPath", "");
  lastSentSchemaPath = initialSchemaPath;

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "sql" },
      { scheme: "file", language: "sqlite" },
    ],
    outputChannel,
    initializationOptions: {
      ...(initialSchemaPath ? { schemaPath: initialSchemaPath } : {}),
    },
    middleware: {
      didOpen: (document, next) => {
        sendSchemaIfChanged(document.uri, outputChannel);
        return next(document);
      },
    },
  };

  client = new LanguageClient(
    "syntaqlite",
    "syntaqlite",
    serverOptions,
    clientOptions,
  );

  // Status bar item showing active schema
  schemaStatusItem = vscode.window.createStatusBarItem(
    vscode.StatusBarAlignment.Right,
    100,
  );
  schemaStatusItem.command = "syntaqlite.openSchema";
  updateSchemaStatusItem(initialSchemaPath);
  context.subscriptions.push(schemaStatusItem);

  // Register open schema command
  context.subscriptions.push(
    vscode.commands.registerCommand("syntaqlite.openSchema", async () => {
      const editor = vscode.window.activeTextEditor;
      const resolved = editor
        ? resolveSchemaForUri(editor.document.uri)
        : vscode.workspace.getConfiguration("syntaqlite").get<string>("schemaPath", "");
      if (!resolved) {
        void vscode.window.showInformationMessage(
          "No schema configured. Set syntaqlite.schemaPath or syntaqlite.schemas in settings.",
        );
        return;
      }
      const uri = vscode.Uri.file(resolved);
      try {
        await vscode.window.showTextDocument(uri);
      } catch {
        void vscode.window.showErrorMessage(`Could not open schema file: ${resolved}`);
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

  // When the active editor changes, resolve and send the appropriate schema.
  context.subscriptions.push(
    vscode.window.onDidChangeActiveTextEditor((editor) => {
      if (editor && (editor.document.languageId === "sql" || editor.document.languageId === "sqlite")) {
        sendSchemaIfChanged(editor.document.uri, outputChannel);
        updateSchemaStatusItem(resolveSchemaForUri(editor.document.uri));
      } else {
        schemaStatusItem?.hide();
      }
    }),
  );

  // Watch for schemaPath/schemas configuration changes and notify the server.
  context.subscriptions.push(
    vscode.workspace.onDidChangeConfiguration((e) => {
      if (
        (e.affectsConfiguration("syntaqlite.schemaPath") ||
          e.affectsConfiguration("syntaqlite.schemas")) &&
        client
      ) {
        const editor = vscode.window.activeTextEditor;
        if (editor) {
          // Force re-resolution by clearing last-sent value.
          lastSentSchemaPath = undefined;
          sendSchemaIfChanged(editor.document.uri, outputChannel);
        } else {
          // No active editor — just send the fallback schemaPath.
          const updated = vscode.workspace.getConfiguration("syntaqlite");
          const newSchemaPath = updated.get<string>("schemaPath") || "";
          if (newSchemaPath !== lastSentSchemaPath) {
            lastSentSchemaPath = newSchemaPath;
            void client.sendNotification("workspace/didChangeConfiguration", {
              settings: { schemaPath: newSchemaPath },
            });
            updateSchemaStatusItem(newSchemaPath);
            outputChannel.appendLine(
              `Schema path changed: ${newSchemaPath || "(cleared)"}`,
            );
          }
        }
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
