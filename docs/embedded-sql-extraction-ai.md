# Embedded SQL Extraction: Language Analysis

Status: Research notes
Date: 2026-03-01

## Key Finding

String extraction complexity is concentrated in Python and JS/TS. Most target languages (Go, Rust, Java, C++, Kotlin, Swift) have no interpolation inside string literals or only simple interpolation — making extraction trivial.

## Per-Language Analysis

| Language | SQL string pattern | Interpolation | Holes in string? | Extraction difficulty |
|----------|-------------------|---------------|-------------------|----------------------|
| Python | `f"SELECT ... {x}"` | `{}` with arbitrary exprs | Yes | Medium (brace matching, nested exprs, dict literals, strings inside holes) |
| JS/TS | `` sql`SELECT ... ${x}` `` | `${}` with arbitrary exprs | Yes | Easy if using tsserver; medium otherwise |
| Go | `"SELECT ... " + x` | None | No | Trivial (find string literals) |
| Rust | `format!("SELECT ... {}", x)` / `sqlx::query!("...")` | None in the string | No | Trivial (find string literals) |
| Java | `"SELECT ... " + x` | None (text blocks are literal) | No | Trivial (find string literals) |
| Kotlin | `"SELECT ... $x"` / `"${expr}"` | `$` / `${}` | Yes | Easy (simple brace matching) |
| Swift | `"SELECT ... \(x)"` | `\()` | Yes | Easy (paren matching) |
| C++ | `"SELECT ... "` | None | No | Trivial (find string literals) |

## Extraction Strategy

### Default: server-side extraction in Rust

For most languages, a lightweight Rust string-literal finder (50–100 lines per language) is sufficient. No external parser dependency needed — just match quote characters, handle escapes, and optionally match simple interpolation delimiters.

This covers: Go, Rust, Java, C++, Kotlin, Swift, and Python.

### Exception: JS/TS can use client-side extraction via TS plugin

TypeScript's own language service already has perfect AST knowledge of template literals. Microsoft's `typescript-template-language-service-decorator` library provides:
- AST traversal to find tagged template literals matching configured tag names
- Coordinate mapping between template-local offsets and file-absolute positions
- `${expr}` substitution handling with offset preservation
- Proxying of LanguageService methods

This means a thin TS plugin (~50 lines of glue) can send `EmbeddedSqlFragment` data to syntaqlite, avoiding any JS/TS parsing in Rust. The CLI (`syntaqlite validate file.js`) would still need a simple server-side fallback — but template literal tokenizing is straightforward (match `` ` `` and `${`/`}`).

### The unified interface

Regardless of where extraction happens, the contract is the same:

```rust
pub struct EmbeddedSqlFragment {
    pub sql_range: Range<usize>,   // byte range of SQL content in host file
    pub sql_text: String,          // SQL text with holes replaced
    pub holes: Vec<Hole>,          // interpolation holes
}

pub struct Hole {
    pub host_range: Range<usize>,  // byte range in host file (e.g., `{user_id}`)
    pub sql_offset: usize,         // offset within sql_text
}
```

It doesn't matter whether this struct was built by Rust code inside the server or by a TS plugin sending it over LSP.

## Rust Library Options for Server-Side Extraction

### For Python f-strings

| Option | Published | License | Notes |
|--------|-----------|---------|-------|
| Hand-rolled (~150 lines) | N/A | N/A | Brace matching with edge cases: nested `{}`, strings inside holes, `{{`/`}}` escapes, triple-quoted strings |
| `tree-sitter` + `tree-sitter-python` | Yes | MIT | CST gives `string` → `interpolation` children with byte ranges. Handles all edge cases including PEP 701 nested f-strings. Compiles C grammar via `cc` |
| `rustpython-parser` | Yes (v0.4) | MIT | Full parser, gives `ExprJoinedStr` / `FormattedValue` AST with source ranges. Frozen pre-PEP 701 (no Python 3.12 nested f-strings). Medium weight (LALRPOP, bigint) |
| Ruff's `ruff_python_parser` | **Not published** | MIT | Best Python parser in Rust, fully implements PEP 701. Would require vendoring from ruff monorepo |

### For JS/TS (CLI fallback / non-editor use)

| Option | Published | License | Notes |
|--------|-----------|---------|-------|
| Hand-rolled (~100 lines) | N/A | N/A | Simple: match `` ` ``, find `${`, track brace depth to `}` |
| `tree-sitter` + `tree-sitter-javascript` | Yes | MIT | CST gives `template_string` → `template_substitution` children. Shares runtime with tree-sitter-python |
| `oxc_parser` | Yes | MIT | Fast, all-Rust, arena-allocated. `TemplateLiteral` AST with interleaved quasis + expressions |
| `swc_ecma_parser` | Yes | Apache-2.0 | Most downloaded, but heavy (macro-heavy AST, globals system) |

### Recommendation

For Python, start with a hand-rolled extractor. F-string brace matching is well-scoped and avoids adding a parser dependency. Fall back to tree-sitter-python later only if edge cases (PEP 701 nested f-strings) prove painful.

For JS/TS, use the TS plugin for editor contexts. For CLI, a hand-rolled template literal tokenizer is straightforward — `${}` hole detection is simpler than Python's `{}` (no ambiguity with dict literals).

Tree-sitter is the insurance policy: if hand-rolled extractors accumulate too many edge cases, swap to tree-sitter for both languages with one shared runtime dependency.
