+++
title = "JavaScript API reference"
description = "Types and methods for syntaqlite."
weight = 5
+++

# JavaScript API reference

The `syntaqlite` package provides a TypeScript API backed by WASM.

```bash
npm install syntaqlite
```

For a getting-started guide, see
[Browser / JavaScript](@/getting-started/wasm-js.md).

## Engine

| Type / Method | Description |
|---------------|-------------|
| `new Engine(config?)` | Create an engine. See `EngineConfig` below |
| `engine.load()` | `Promise<void>`. Load the WASM runtime — must complete before other calls |
| `engine.ready` | `boolean`. `true` after `load()` resolves |

### Formatting

| Method | Description |
|--------|-------------|
| `runFmt(sql, opts)` | Format SQL. Returns `FormatResult` |
| `runAst(sql)` | Dump AST as text. Returns `FormatResult` |
| `runAstJson(sql)` | Parse to JSON AST. Returns `AstResult` |

### Validation

| Method | Description |
|--------|-------------|
| `runDiagnostics(sql, version?)` | Run semantic analysis. Returns `DiagnosticsResult`. `version` is a SQLite version integer (use `versionToInt()`) |
| `setSessionContext(json)` | Apply schema context from a JSON string (see `SessionContextPayload`) |
| `setSessionContextDdl(sql)` | Apply schema context from DDL. Returns `{ok}` or `{ok, error}` |
| `clearSessionContext()` | Remove all schema context |

### Code intelligence

| Method | Description |
|--------|-------------|
| `runCompletions(sql, offset, version?)` | Completions at byte offset. Returns `CompletionsResult` |
| `runSemanticTokens(sql, start?, end?, version?)` | Semantic tokens (LSP/Monaco format). Each token: 5 × `u32` `(deltaLine, deltaStartChar, length, legendIndex, 0)`. Returns `Uint32Array` or `undefined` |

### SQLite version and compile flags

| Method | Description |
|--------|-------------|
| `setSqliteVersion(version)` | Set target version string (e.g., `"3.47.0"`) |
| `setCflag(name)` | Enable a compile-time flag |
| `clearCflag(name)` | Disable a compile-time flag |
| `clearAllCflags()` | Clear all enabled flags |
| `getCflagList()` | Returns `CflagEntry[]` with all available flags |

### Dialect management

| Method | Description |
|--------|-------------|
| `loadDialectFromUrl(url, symbol)` | Load dialect WASM module. Returns `Promise<DialectBinding>` |
| `setDialectPointer(ptr)` | Switch to a loaded dialect. Re-applies session context automatically |
| `clearDialectPointer()` | Revert to base SQLite |

### Embedded SQL (experimental)

| Method | Description |
|--------|-------------|
| `setLanguageMode(lang)` | `"sql"`, `"python"`, or `"typescript"` |
| `runExtract(source)` | Extract SQL fragments. Returns `EmbeddedExtractResult`. No-op in `"sql"` mode |

## DialectManager

Manages preset and custom dialect loading.

| Type / Method | Description |
|---------------|-------------|
| `new DialectManager(config?)` | Create with optional `DialectManagerConfig` |
| `getPresets()` | List available presets |
| `loadDefault(engine)` | Load the first preset |
| `selectPreset(engine, preset)` | Switch to a preset dialect |
| `loadFromFile(engine, file, symbol)` | Load from a `File` object. Returns error string or `undefined` |

`BUILTIN_PRESETS` is an exported array of default presets.

## DialectConfigManager

Manages SQLite version and compile-flag configuration.

| Type / Method | Description |
|---------------|-------------|
| `loadAvailableCflags(engine)` | Query engine for cflag metadata |
| `visibleCflagEntries(version)` | Get `CflagEntry[]` valid for a version string |
| `apply(engine, version, cflags)` | Apply version + cflag set to engine |

`VERSION_OPTIONS` is an exported array of supported version strings (`"latest"`,
`"3.47.0"`, ..., `"3.23.0"`).

`versionToInt(version)` converts a version string to SQLite's integer encoding.

## SchemaContextManager

| Type / Method | Description |
|---------------|-------------|
| `apply(engine, rawText, format, force?)` | Apply schema. `format` is `"simple"` or `"ddl"` |
| `parseError` | `string \| undefined`. Error from last DDL parse |
| `parsedTableCount` | `number \| undefined`. Tables parsed |

`parseSimple(rawText)` parses the simple schema format (`table: col1,col2` per
line, `#` comments).

## Types

### Configuration

```typescript
interface EngineConfig {
  runtimeJsPath?: string;
  runtimeWasmPath?: string;
}

interface DialectManagerConfig {
  presets?: DialectPreset[];
  onDialectChanged?: () => void;
}

interface DialectPreset {
  id: string;
  label: string;
  wasmUrl: string;
  symbol: string;
}

interface CflagEntry {
  name: string;
  minVersion: number;
  category: string;
}
```

### Format types

```typescript
type KeywordCase = 0 | 1 | 2; // preserve, upper, lower

interface FormatOptions {
  lineWidth: number;
  indentWidth: number;
  keywordCase: KeywordCase;
  semicolons: boolean;
}

interface FormatResult {
  ok: boolean;
  text: string;
}
```

### AST types

```typescript
type AstJsonNode = AstListNode | AstRegularNode;
type AstFieldValue = AstJsonNode | string | boolean | string[] | null;

interface AstListNode {
  type: string;
  count: number;
  children: AstJsonNode[];
}

interface AstRegularNode {
  type: string;
  [field: string]: AstFieldValue | undefined;
}

type AstResult =
  | { ok: true; statements: AstJsonNode[] }
  | { ok: false; error: string };
```

### Diagnostic types

```typescript
interface DiagnosticEntry {
  startOffset: number;
  endOffset: number;
  message: string;
  severity: "error" | "warning" | "info" | "hint";
  detail: DiagnosticDetail;
  help?: string;
  helpDetail?: HelpDetail;
}

type DiagnosticDetail =
  | { kind: "unknown_table"; name: string }
  | { kind: "unknown_column"; column: string; table?: string }
  | { kind: "unknown_function"; name: string }
  | { kind: "function_arity"; name: string; expected: number[]; got: number }
  | null;

type HelpDetail = { kind: "suggestion"; value: string } | null;

interface DiagnosticsResult {
  ok: boolean;
  diagnostics: DiagnosticEntry[];
}
```

### Completion types

```typescript
interface CompletionEntry {
  label: string;
  kind: "keyword" | "function" | "class";
}

interface CompletionsResult {
  ok: boolean;
  items: CompletionEntry[];
}
```

### Dialect types

```typescript
interface DialectBinding {
  symbol: string;
  ptr: number;
  label: string;
}
```

### Schema types

```typescript
type SchemaFormat = "simple" | "ddl";

interface SessionContextPayload {
  tables: { name: string; columns: string[] }[];
  views: never[];
  functions: never[];
}
```

### Embedded SQL types (experimental)

```typescript
type EmbeddedLanguage = "python" | "typescript";

interface EmbeddedFragment {
  start: number;
  end: number;
  sql: string;
  holes: EmbeddedHole[];
}

interface EmbeddedHole {
  start: number;
  end: number;
  placeholder: string;
}

interface EmbeddedExtractResult {
  ok: boolean;
  fragments: EmbeddedFragment[];
}
```
