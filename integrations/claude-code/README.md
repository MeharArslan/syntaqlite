# syntaqlite — Claude Code Plugin

SQLite SQL language support for [Claude Code](https://claude.ai/code). Provides
diagnostics, formatting, completions, and semantic tokens via the syntaqlite LSP
server.

## Features

- **LSP integration** — Automatically starts the syntaqlite language server for
  `.sql` files, providing diagnostics, formatting, completions, and semantic
  highlighting.
- **`/syntaqlite` skill** — Format SQL, inspect parse trees, and analyze queries
  directly from the Claude Code prompt.

## Prerequisites

The `syntaqlite` binary must be on your `PATH`. Install via any of:

```bash
# Homebrew
brew install LalitMaganti/tap/syntaqlite

# Cargo
cargo install syntaqlite-cli

# pip
pip install syntaqlite

# Download script
curl -fsSL https://syntaqlite.com/install.sh | sh
```

## Installation

```bash
claude plugin add syntaqlite
```

Or from the GitHub marketplace:

```bash
claude plugin marketplace add LalitMaganti/syntaqlite
```

## Usage

Once installed, the LSP server starts automatically when you open `.sql` files.
Use the `/syntaqlite` skill for CLI operations:

```
/syntaqlite format this file
/syntaqlite parse and show the AST for this query
```

## License

Apache 2.0. See [LICENSE](../../LICENSE) for details.
