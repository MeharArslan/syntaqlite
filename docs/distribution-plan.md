# Distribution & Language Bindings Plan

## Problem

syntaqlite has three distinct audiences with different needs:

1. **Embedders** — want to parse/tokenize SQL in C/C++/Zig with zero dependencies
2. **Devtool authors** — want formatting, validation, completions in any language
3. **Dialect authors** — want to define new SQL dialects on top of the core engine

Today we have a Rust crate and a hand-written WASM layer. We need a coherent
distribution story that covers C, C++, Go, Zig, TypeScript, and Python without
creating an unsustainable maintenance burden.

## Prior Art

### SQLite (the model we're closest to)

- `sqlite-amalgamation-*.zip` → `sqlite3.c`, `sqlite3.h`, `sqlite3ext.h`
- `sqlite-autoconf-*.tar.gz` → Autotools build for `libsqlite3.so/.a`
- Three tiers (embed, link, extend) from two headers and one .c file
- Extension authors include `sqlite3ext.h`, receive function pointers at runtime

### DuckDB

- Prefix-namespaced release artifacts: `duckdb_cli-*`, `libduckdb-*`, `static-libs-*`
- Two-header split: `duckdb.h` (users) / `duckdb_extension.h` (extenders)
- Language bindings shipped via ecosystem package managers (PyPI, npm, Maven),
  not GitHub releases
- Extension template repo with C API that receives function pointers, no
  link-time dependency on DuckDB

### Hugo

- Edition as filename infix: `hugo_extended_withdeploy_0.157.0_linux-amd64.tar.gz`
- Self-documenting filenames, no matrix picker needed
- No extension API — composition via themes/modules, not C plugins

### Tree-sitter (cautionary tale)

- No dedicated embed artifact — C users must clone the repo
- NxM packaging problem: every grammar × every language ecosystem
- Grammar authoring requires Node.js regardless of target language
- Community-acknowledged "packaging mess"

### Key Lessons

1. SQLite's amalgamation insight: `gcc syntaqlite.c app.c -o app` is worth the
   generator complexity
2. DuckDB's prefix namespacing (`duckdb_cli-`, `libduckdb-`) is unambiguous
3. The extension header split (`sqlite3ext.h` / `duckdb_extension.h`) is a
   proven pattern — both SQLite and DuckDB converged on it independently
4. Tree-sitter's NxM grammar×language matrix is exactly what we want to avoid

## Architecture

### The Two APIs

syntaqlite's internal structure is:

```
┌─────────────────────────────────────────────────────┐
│  High-level Rust API                                │
│  format, validate, complete, semantic tokens, LSP   │
│  (Rust: Parser, Formatter, AnalysisHost, etc.)      │
├─────────────────────────────────────────────────────┤
│  Low-level C core                                   │
│  parse, tokenize, AST arena, node access            │
│  (C: syntaqlite_create_parser, syntaqlite_parse...) │
└─────────────────────────────────────────────────────┘
```

These are two different products at two different abstraction levels:

- **The C core** is a parser engine. It gives you a parse tree. Self-contained,
  no dependencies, compiles anywhere C compiles.
- **The Rust layer** adds intelligence: formatting (bytecode interpreter),
  validation (schema checks, fuzzy suggestions), completions, semantic tokens.

The C→Rust→C path is not circular. The low-level C core is a parser state
machine. The high-level C API is a developer tool interface. They share a
language at the boundary but are completely different abstraction levels — like
how SQLite's parser is C and `sqlite3_exec()` is also C, but they serve
different purposes.

## The Three Use Cases

### Use Case 1: Embed a SQL parser (C amalgamation)

> "I want to parse SQLite SQL in my C/C++/Zig project with zero dependencies."

**What the user gets:**

```
syntaqlite-amalgamation-{version}.zip
├── syntaqlite.h              ← public parser/tokenizer API
├── syntaqlite.c              ← core engine + SQLite dialect, single TU
└── USAGE.md
```

**How they use it:**

```c
#include "syntaqlite.h"

// parse, walk AST, access tokens — all from two files
```

```sh
gcc -c syntaqlite.c -o syntaqlite.o
gcc myapp.c syntaqlite.o -o myapp
```

Same model as `sqlite3.c` + `sqlite3.h`. Two files, zero decisions.

### Use Case 2: Devtools (high-level library)

> "I want SQL formatting, validation, and completions in Go/Python/C++/etc."

**What the user gets depends on language:**

| Language | Install | What ships |
|----------|---------|------------|
| Rust | `cargo add syntaqlite` | Native crate (already exists) |
| C / C++ | download `libsyntaqlite-{platform}.zip` | `syntaqlite_api.h` + `.so/.dylib/.a` |
| Zig | download same zip | `@cImport("syntaqlite_api.h")` + link `.a` |
| Go | `go get ...syntaqlite` | cgo wrapper over `syntaqlite_api.h` + `.a` |
| Python | `pip install syntaqlite` | cffi wrapper, bundled `.so` |
| TypeScript | `npm install syntaqlite` | WASM bundle (already exists) |

**The high-level C API (`syntaqlite_api.h`):**

```c
// Opaque engine handle — owns parser, formatter, validator internally
typedef struct SyntaqliteEngine SyntaqliteEngine;

typedef struct {
    unsigned int line_width;    // 0 = default (80)
    unsigned int keyword_case;  // 0=preserve, 1=UPPER, 2=lower
    unsigned int semicolons;    // 0=off, 1=on
} SyntaqliteFormatConfig;

// Lifecycle
SyntaqliteEngine* syntaqlite_engine_new(void);
void              syntaqlite_engine_free(SyntaqliteEngine* e);

// Core operations — return malloc'd strings, free with syntaqlite_string_free
char* syntaqlite_format(SyntaqliteEngine* e, const char* sql, size_t len,
                        const SyntaqliteFormatConfig* config);
char* syntaqlite_ast_json(SyntaqliteEngine* e, const char* sql, size_t len);
char* syntaqlite_diagnostics(SyntaqliteEngine* e, const char* sql, size_t len);
char* syntaqlite_completions(SyntaqliteEngine* e, const char* sql, size_t len,
                             unsigned int offset);
char* syntaqlite_semantic_tokens(SyntaqliteEngine* e, const char* sql,
                                 size_t len);
void  syntaqlite_string_free(char* s);

// Configuration
int syntaqlite_set_version(SyntaqliteEngine* e, const char* version);
int syntaqlite_set_schema_ddl(SyntaqliteEngine* e, const char* ddl, size_t len);
int syntaqlite_set_schema_json(SyntaqliteEngine* e, const char* json,
                               size_t len);
int syntaqlite_clear_schema(SyntaqliteEngine* e);
int syntaqlite_set_cflag(SyntaqliteEngine* e, const char* name);
int syntaqlite_clear_cflag(SyntaqliteEngine* e, const char* name);
```

~15 functions. String in, string/JSON out. Every language with a C FFI can
consume this trivially.

### Use Case 3: Dialect authoring (extension API)

> "I have a SQLite-based database with custom syntax. I want syntaqlite to
> understand my dialect."

**What the user gets:**

```
syntaqlite-amalgamation-{version}.zip
├── syntaqlite.h              ← parser/tokenizer API (same as Use Case 1)
├── syntaqlite.c              ← core engine (without SQLite dialect baked in)
├── syntaqlite_ext.h          ← dialect SPI: what a dialect must provide
└── USAGE.md
```

Wait — this raises a question. See Open Questions below.

**How they use it:**

1. Write `.synq` files describing their dialect's AST nodes, enums, flags, fmt
2. Run `syntaqlite codegen --dialect mydialect --input nodes.synq --output ./generated/`
3. This produces `syntaqlite_mydialect.c` + `syntaqlite_mydialect.h`
4. Compile: `gcc syntaqlite.c syntaqlite_mydialect.c myapp.c -o myapp`

The `syntaqlite_ext.h` header defines the contract: what tables and functions a
dialect must provide (node metadata, token types, fmt bytecode, function
catalog). The codegen tool produces code that satisfies this contract.

## Naming Conventions

### File naming

Following SQLite and DuckDB precedent:

| File | Purpose | Audience |
|------|---------|----------|
| `syntaqlite.h` | Low-level parser/tokenizer API | Embedders, dialect authors |
| `syntaqlite.c` | Core engine amalgamation | Embedders, dialect authors |
| `syntaqlite_ext.h` | Dialect extension SPI | Dialect authors only |
| `syntaqlite_api.h` | High-level devtools API | Devtool authors (all languages) |

### Release artifact naming

Following DuckDB's prefix pattern + Hugo's platform convention:

```
# Amalgamation (embed story)
syntaqlite-amalgamation-{version}.zip

# High-level library (devtools story)
libsyntaqlite-{version}-linux-x64.tar.gz
libsyntaqlite-{version}-linux-arm64.tar.gz
libsyntaqlite-{version}-macos-arm64.tar.gz
libsyntaqlite-{version}-macos-x64.tar.gz
libsyntaqlite-{version}-windows-x64.zip

# CLI tool
syntaqlite-cli-{version}-linux-x64.tar.gz
syntaqlite-cli-{version}-macos-arm64.tar.gz
...

# WASM (web/Node)
syntaqlite-wasm-{version}.tar.gz
```

The version token is semver (`1.0.0`), not SQLite's integer encoding, since
we're already using semver for the Rust crate.

## Language Binding Maintenance

### Tier 1: Zero extra work (C header IS the binding)

| Language | How | Maintenance |
|----------|-----|-------------|
| C | `syntaqlite.h` or `syntaqlite_api.h` directly | None — headers are the source of truth |
| C++ | Same headers (C linkage compatible), optional RAII wrapper | ~50 lines |
| Zig | `@cImport("syntaqlite_api.h")`, link `.a` | None |

### Tier 2: Thin wrapper (~100-150 lines each)

| Language | How | Maintenance |
|----------|-----|-------------|
| Go | cgo package wrapping `syntaqlite_api.h` | Low — API is ~15 functions |
| Python | cffi loading `.so` + Pythonic class wrapper | Low |

### Tier 3: Already exists / different path

| Language | How | Maintenance |
|----------|-----|-------------|
| Rust | Native crate | Normal Rust development |
| TypeScript | WASM (Emscripten, already built) | Already maintained |

**Total new code for all 7 languages: ~400 lines** beyond the `syntaqlite-capi`
Rust crate (~200 lines) that implements `syntaqlite_api.h`.

## Implementation: syntaqlite-capi crate

A new Rust crate in the workspace:

```
syntaqlite-capi/
├── Cargo.toml        # depends on syntaqlite with features=["fmt","lsp","validation"]
├── cbindgen.toml     # generates syntaqlite_api.h automatically
└── src/
    └── lib.rs        # ~200 lines: extern "C" fns wrapping AnalysisHost/Formatter
```

This crate is structurally identical to what `syntaqlite-wasm` already does
(opaque engine state, string in/out, JSON for structured data) but targets
native instead of WASM. The two implementations share the same Rust library
underneath and naturally stay in sync.

## Open Questions

### 1. Amalgamation: one file or two?

**Option A:** Ship `syntaqlite.c` with SQLite dialect baked in (like sqlite3.c
includes everything). Simpler for 99% of users.

**Option B:** Ship `syntaqlite.c` (core) + `syntaqlite_sqlite.c` (dialect)
separately. Enables faster incremental compilation when only the dialect changes.
Also cleaner for dialect authors who don't want SQLite.

Recommendation: **Option A for the default amalgamation zip, Option B available
in a separate "dialect development" zip.** Most users want one file. Dialect
authors are power users who can handle two files.

### 2. Should `syntaqlite.c` (amalgamation) and `syntaqlite_api.h` (high-level)
coexist?

Can someone use the amalgamation AND the high-level API? Probably not — the
high-level API requires Rust (formatter, validation). These are separate
products for separate audiences. We should be clear about this in docs.

### 3. Dialect amalgamation: does it include SQLite as base?

For Perfetto (which extends SQLite), should `syntaqlite_perfetto.c` include
the SQLite dialect tables, or should users compile both
`syntaqlite_sqlite.c` and `syntaqlite_perfetto.c`? The "one file per dialect"
model suggests baking the base in.

### 4. WASM vs native for TypeScript

Currently WASM is built via Emscripten. Should the npm package also offer a
native Node.js addon (via napi-rs or N-API) for server-side use where WASM
overhead matters? Probably not initially — WASM is fast enough and avoids
platform-specific binaries in npm.

### 5. Should Go/Python wrappers live in-repo or separate repos?

- **In-repo:** easier to keep in sync, version together, test in CI
- **Separate repos:** cleaner for Go modules (which want repo=module),
  independent release cadence

DuckDB keeps language clients in separate repos. SQLite has no official
language bindings. Recommendation: **start in-repo, extract later if needed.**
