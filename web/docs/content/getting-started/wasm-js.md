+++
title = "Browser / JavaScript"
description = "Use syntaqlite from JavaScript via WebAssembly."
weight = 6
+++

# Using syntaqlite in the browser

syntaqlite compiles to WebAssembly and provides a TypeScript/JavaScript API.
This is the same engine that powers the
[online playground](https://playground.syntaqlite.com).

## Install

```bash
npm install syntaqlite
```

The package includes WASM binary files (`syntaqlite-runtime.js`,
`syntaqlite-runtime.wasm`, and dialect modules) in `wasm/` — serve these
alongside your app.

## Set up the engine

```typescript
import { Engine, DialectManager } from "syntaqlite";

const engine = new Engine({
  runtimeJsPath: "./wasm/syntaqlite-runtime.js",
  runtimeWasmPath: "./wasm/syntaqlite-runtime.wasm",
});
await engine.load();

const dialectMgr = new DialectManager();
await dialectMgr.loadDefault(engine);
```

## Format SQL

```typescript
const result = engine.runFmt(
  "select id,name from users where active=1",
  { lineWidth: 80, keywordCase: 1, semicolons: true }
);
console.log(result.text);
// SELECT id, name
// FROM users
// WHERE active = 1;
```

`keywordCase` values: `0` = preserve, `1` = UPPER, `2` = lower.

## Validate against a schema

Load a schema, then run diagnostics on a query:

```typescript
engine.setSessionContextDdl(
  "CREATE TABLE users (id INTEGER, name TEXT, email TEXT);"
);

const diags = engine.runDiagnostics("SELECT nme FROM users");
for (const d of diags.diagnostics) {
  console.log(`[${d.severity}] ${d.message}`);
  if (d.help) console.log(`  help: ${d.help}`);
}
// [warning] unknown column 'nme'
//   help: did you mean 'name'?
```

## Parse to AST

```typescript
const textResult = engine.runAst("SELECT 1 + 2");
console.log(textResult.text);

// Or as structured JSON:
const jsonResult = engine.runAstJson("SELECT 1 + 2");
```

## Get completions

```typescript
const completions = engine.runCompletions("SELECT ", 7);
for (const item of completions.items) {
  console.log(item);
}
```

## Semantic tokens

For syntax highlighting (compatible with Monaco editor):

```typescript
const tokens = engine.runSemanticTokens("SELECT * FROM users");
// Returns Uint32Array in LSP semantic tokens encoding
```

## Next steps

- [JavaScript API reference](@/reference/js-api.md) — all methods on
  `Engine`, format options, diagnostic types
- [Online playground](https://playground.syntaqlite.com) — try it without
  installing anything
