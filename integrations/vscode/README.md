# syntaqlite VS Code Extension

SQL language support powered by the syntaqlite language server.

## Features

- **Diagnostics** — syntax errors and semantic warnings as you type
- **Formatting** — format SQL documents via `Format Document` or on save
- **Completions** — SQL keywords and built-in functions
- **Semantic highlighting** — context-aware token coloring

## Installation

Install from the VS Code Marketplace. The extension includes the
syntaqlite binary for your platform — no additional setup required.

## Configuration

Schemas and formatting are configured via `syntaqlite.toml` in your project
root — the LSP reads it automatically. See the
[main README](../../README.md#project-configuration) for the full format.

```toml
[schemas]
"src/**/*.sql" = ["schema/main.sql"]

[format]
line-width = 100
keyword-case = "lower"
```

| VS Code Setting          | Default | Description                                   |
|--------------------------|---------|-----------------------------------------------|
| `syntaqlite.serverPath`  | `""`    | Override path to the syntaqlite binary.       |

## Commands

- **syntaqlite: Restart Language Server** — restart the LSP server
- **syntaqlite: Format Document** — format the active SQL file
- **syntaqlite: Open Config File** — open the project's `syntaqlite.toml`

## Development

```sh
cd integrations/vscode
npm install
npm run compile
```

Build the CLI and point the extension at it via `syntaqlite.serverPath`:

```sh
cargo build --release -p syntaqlite-cli
```

Then set `syntaqlite.serverPath` to the absolute path of `target/release/syntaqlite`
in your VS Code settings.

Press **F5** to launch an Extension Development Host.

### Packaging platform-specific .vsix

```sh
cargo build --release -p syntaqlite-cli
node scripts/package-target.mjs --target darwin-arm64 --binary ../../target/release/syntaqlite
```

Supported targets: `darwin-arm64`, `darwin-x64`, `linux-arm64`, `linux-x64`, `win32-x64`
