+++
title = "Browser / JavaScript"
description = "Use syntaqlite from JavaScript via WebAssembly."
weight = 6
+++

# Using syntaqlite in the browser

syntaqlite compiles to WebAssembly and provides a TypeScript/JavaScript API
via the
[`syntaqlite`](https://github.com/LalitMaganti/syntaqlite/tree/main/web/syntaqlite-js)
package. This is the same engine that powers the
[online playground](https://playground.syntaqlite.com).

## Install

```bash
npm install syntaqlite
```

The package has zero npm dependencies. You also need the WASM binary files
(`syntaqlite-runtime.js`, `syntaqlite-runtime.wasm`, and dialect WASM modules)
served alongside your app. These are included in the package's `wasm/`
directory.

## Quick start

```typescript
import { Engine, DialectManager } from "syntaqlite";

// 1. Create and load the engine
const engine = new Engine({
  runtimeJsPath: "./wasm/syntaqlite-runtime.js",
  runtimeWasmPath: "./wasm/syntaqlite-runtime.wasm",
});
await engine.load();

// 2. Load the SQLite dialect
const dialectMgr = new DialectManager();
await dialectMgr.loadDefault(engine);

// 3. Format SQL
const result = engine.runFmt(
  "select id,name from users where active=1",
  { lineWidth: 80, keywordCase: 1, semicolons: true }
);
console.log(result.text);
// SELECT id, name
// FROM users
// WHERE active = 1;
```

The `Engine` class
([`engine.ts`](https://github.com/LalitMaganti/syntaqlite/blob/main/web/syntaqlite-js/src/engine.ts))
is the main entry point. It manages the WASM runtime and exposes all
operations.

## Format options

The `keywordCase` parameter uses numeric values:

| Value | Meaning |
|-------|---------|
| `0` | Preserve original casing |
| `1` | UPPER CASE |
| `2` | lower case |

```typescript
import type { FormatOptions } from "syntaqlite";

const opts: FormatOptions = {
  lineWidth: 120,
  keywordCase: 2,   // lowercase
  semicolons: false,
};
const result = engine.runFmt("SELECT 1", opts);
```

## Run diagnostics

The WASM build includes the full semantic analyzer. To validate SQL against a
schema, define the schema first:

```typescript
import { SchemaContextManager } from "syntaqlite";

// Define schema (simple format: "table: col1,col2,col3")
const schema = new SchemaContextManager();
schema.apply(
  engine,
  "users: id,name,email\nposts: id,user_id,title",
  "simple"
);

// Or use DDL:
const ddlResult = engine.setSessionContextDdl(
  "CREATE TABLE users (id INTEGER, name TEXT, email TEXT);"
);

// Run diagnostics
const diags = engine.runDiagnostics("SELECT nme FROM users");
for (const d of diags.diagnostics) {
  console.log(`[${d.severity}] ${d.message} (${d.startOffset}..${d.endOffset})`);
  if (d.help) console.log(`  help: ${d.help}`);
}
```

Each diagnostic has the structure defined in
[`types.ts`](https://github.com/LalitMaganti/syntaqlite/blob/main/web/syntaqlite-js/src/types.ts):

```typescript
interface DiagnosticEntry {
  startOffset: number;
  endOffset: number;
  message: string;
  severity: "error" | "warning" | "info" | "hint";
  help?: string;
  // ... additional fields for structured detail
}
```

## Parse to AST

```typescript
// Text dump (same as CLI)
const textResult = engine.runAst("SELECT 1 + 2");
console.log(textResult.text);

// Structured JSON
const jsonResult = engine.runAstJson("SELECT 1 + 2");
if (jsonResult.ok) {
  console.log(jsonResult.statements[0]);
}
```

## Code completions

```typescript
const completions = engine.runCompletions("SELECT ", 7);
for (const item of completions.items) {
  console.log(item);
}
```

## Semantic tokens

For syntax highlighting (compatible with Monaco editor's semantic token
format):

```typescript
const tokens = engine.runSemanticTokens("SELECT * FROM users");
// Returns Uint32Array in LSP semantic tokens encoding
```

## SQLite version and compile flags

```typescript
engine.setSqliteVersion("3.47.0");
engine.setCflag("SQLITE_ENABLE_JSON1");

// List available compile flags
const flags = engine.getCflagList();
for (const f of flags) {
  console.log(`${f.name}: ${f.description}`);
}
```

## Building from source

If you need to build the WASM binaries yourself:

```bash
# Prerequisites
rustup target add wasm32-unknown-emscripten
brew install emscripten  # macOS
source "$(brew --prefix emscripten)/libexec/emsdk_env.sh"

# Build WASM + playground
tools/build-web-playground

# Build JS package
cd web/syntaqlite-js && npm install && npm run build
```

The build script outputs WASM files to `web/playground/wasm/`. For production
builds, use `tools/build-web-playground-prod` which includes type-checking and
Vite bundling.
