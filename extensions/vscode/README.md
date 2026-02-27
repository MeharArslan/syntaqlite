# syntaqlite VS Code Extension

SQL language support powered by the syntaqlite LSP server.

## Features

- Syntax error diagnostics
- Code formatting
- Completions (functions, keywords)

## Prerequisites

The `syntaqlite` CLI must be on your `PATH`. Build it from the repo root:

```sh
cargo build --release -p syntaqlite-cli
export PATH="$PWD/target/release:$PATH"
```

## Development

```sh
cd extensions/vscode
npm install
npm run compile
```

Then press **F5** in VS Code to launch an Extension Development Host with the extension loaded.
