# Multi-Version SQLite Support

## 1. Problem

Syntaqlite currently extracts its tokenizer, keyword table, and character tables from a single vendored SQLite source tree (`third_party/src/sqlite/`). This pins every consumer to one SQLite version's exact tokenization and keyword recognition behavior.

Real consumers need version-specific behavior:

- **Embedded tools** targeting a specific SQLite version (e.g., Android's bundled 3.32, iOS's 3.39) need the tokenizer and keyword set to match that version exactly — bug-for-bug.
- **Developer tools** (LSPs, playgrounds, linters) need to switch target version at runtime without recompilation, e.g., "show me how SQLite 3.35 parses this query."

## 2. Approach

### 2.1 What varies across SQLite versions

SQLite's SQL surface changes across versions in three ways:

1. **Tokenizer behavior** — new token types are recognized (e.g., `->` and `->>` JSON operators added in 3.38, digit separators like `1_000` added in a later version).
2. **Keyword set** — new keywords are added (e.g., `MATERIALIZED` and `RETURNING` in 3.35, `WITHIN` in 3.46). Keywords are never removed.
3. **Grammar rules** — new syntax productions (e.g., window functions in 3.25, RETURNING clause in 3.35). Rules are never removed.

A critical simplification: **SQLite is purely additive**. Newer versions are strict supersets of older ones. There are no backwards-incompatible removals. Every version check is `>= threshold`, never anything more complex.

### 2.2 What we extract from SQLite source (the 10 fragments)

The current extraction pipeline in `sqlite_runtime_codegen.rs` pulls these fragments using `c_extractor`:

| Fragment                        | Source file       | Extractor call                       | Notes                           |
| ------------------------------- | ----------------- | ------------------------------------ | ------------------------------- |
| CC\_\* defines (30)             | `tokenize.c`      | `extract_specific_defines`           | Character class constants       |
| `aiClass[256]`                  | `tokenize.c`      | `extract_static_array`               | Maps bytes to character classes |
| `IdChar` macro                  | `tokenize.c`      | `extract_defines_with_ifdef_context` | Identifier character test       |
| `charMap` macro                 | `tokenize.c`      | `extract_defines_with_ifdef_context` | Case-folding for keywords       |
| `sqlite3GetToken()`             | `tokenize.c`      | `extract_function`                   | The tokenizer (~200 lines)      |
| `sqlite3CtypeMap[256]`          | `global.c`        | `extract_static_array`               | Character type classification   |
| `sqlite3UpperToLower[256]`      | `global.c`        | `extract_static_array`               | Case conversion table           |
| `sqlite3Is{space,digit,xdigit}` | `sqliteInt.h`     | `extract_specific_defines`           | Character test macros           |
| Keyword table (148 entries)     | `mkkeywordhash.c` | parsed by `mkkeyword` tool           | `{name, token, mask, priority}` |
| Keyword mask `#define` blocks   | `mkkeywordhash.c` | parsed by `mkkeyword` tool           | `SQLITE_OMIT_*` to mask bits    |

Most of these fragments are frozen across versions (character tables, macros). The version-variable ones are:

- `sqlite3GetToken()` — changes when new token types are added
- CC\_\* defines — changes when new character classes are added
- `aiClass[256]` — changes when character class assignments change
- Keyword table — changes when keywords are added or mask values change

### 2.3 Design: dual-mode version dispatch

One set of generated source files serves both compile-time and runtime version selection through a dispatch macro:

```c
// syntaqlite_ext/version_dispatch.h
#ifdef SYNQ_SQLITE_VERSION
  #define SYNQ_VER_GE(d, v)  (SYNQ_SQLITE_VERSION >= (v))
#else
  #define SYNQ_VER_GE(d, v)  ((d)->sqlite_version >= (v))
#endif
```

**Compile-time** (`-DSYNQ_SQLITE_VERSION=3035000`): the macro becomes a constant comparison. The C compiler eliminates all dead branches. Zero runtime cost — identical to hand-written `#if` guards.

**Runtime** (no define): the macro reads from the dialect struct. All version branches are compiled. An LSP can call `dialect.with_version(3_035_000)` to get a version-specific shallow copy of the dialect.

### 2.4 Strategy per component

**Tokenizer**: store complete function variants, not line-level diffs. `sqlite3GetToken` is ~200 lines. Across all supported versions there will be roughly 5 distinct variants. That's ~1000 lines total — trivially small. A dispatch function selects the right variant based on version. Compile-time: `#if` chain selects one, compiler eliminates the rest. Runtime: 2-3 integer comparisons, once per `GetToken` call (not per token).

**Keywords**: one superset hash table containing ALL keywords from ALL versions. A parallel `aKWVersion[]` array stores the introduction version of each keyword. The lookup function skips keywords newer than the target version. Same `SYNQ_VER_GE` macro — compile-time: comparisons are eliminated for always-present keywords; runtime: one extra int comparison per hash probe, negligible.

**Grammar**: Lemon generates a fixed state machine — can't branch at runtime. Two paths:

- _Compile-time_: `%ifdef` in `.y` files + Lemon `-D` flags strip rules for features that don't exist in the target version. A `version_to_omit_flags()` mapping converts version to synthetic `SQLITE_OMIT_*` flags.
- _Runtime_: always parse with the full (latest) grammar. A post-parse AST validation pass flags nodes that require a newer version: "RETURNING clause requires SQLite 3.35+". This is better UX than a cryptic parse error.

**CFlags** (SQLITE*OMIT*\*): same dual-mode pattern. `SYNQ_HAS_FEATURE(d, flag)` macro — compile-time constant or runtime struct field. For grammar: same AST validation path as version checking. Nobody switches cflags mid-session; they're set once at initialization.

### 2.5 Semi-automated workflow

The fully automated diff-to-ifdef approach is fragile (diff alignment errors, lost semantic boundaries). The fully manual approach doesn't scale. The sweet spot:

1. **Automated extraction + diffing** — tool downloads N SQLite versions, runs `c_extractor` on each, hashes fragments, produces a human-readable diff report showing exactly what changed and when.
2. **Manual annotation** — developer reads the report, copies variant files, writes `version_map.toml` and reviews `keywords.toml`. Guided by the report, this is straightforward.
3. **Automated oracle test generation** — tool compiles each real SQLite version, runs its tokenizer on a test corpus, captures token-by-token output as ground truth JSON. These oracle files are checked into the repo.
4. **CI verification** — tests run syntaqlite's version-dispatched tokenizer and keyword lookup against the oracle data for every supported version. Any annotation mistake is caught automatically.

---

## 3. Phase 1: Download and Analysis Tool

This is the first thing to build. Everything else follows from the data it produces.

### 3.1 Goal

Build a `syntaqlite` CLI subcommand that:

1. Downloads SQLite source archives for a configurable set of versions
2. Extracts the 10 code fragments from each version using the existing `c_extractor`
3. Hashes each fragment to identify distinct variants
4. Produces a structured analysis report showing what changed, when, and how

### 3.2 SQLite source acquisition

SQLite distributes source code as ZIP archives. We need individual source files (`src/tokenize.c`, `src/global.c`, `src/sqliteInt.h`, `tool/mkkeywordhash.c`).

There is a **GitHub mirror** at `https://github.com/sqlite/sqlite` with tags like `version-3.35.0`. Source files live at `src/tokenize.c`, `src/global.c`, `src/sqliteInt.h`, `tool/mkkeywordhash.c`.

There are also direct download ZIPs at:

```
https://www.sqlite.org/YYYY/sqlite-src-3XXYYZZ.zip
```

The year varies by release. The version encoding for `3.X.Y` is `3XXYY00`; for `3.X.Y.Z` it's `3XXYYZZ`.

#### Version list

The tool should support a configurable version list. A sensible default covers major feature boundaries:

```
3.24.0   # upsert (ON CONFLICT)
3.25.0   # window functions — big keyword addition
3.26.0   3.27.0   3.28.0   3.29.0
3.30.0   # generated columns (GENERATED, ALWAYS keywords)
3.31.0   3.32.0   3.33.0   3.34.0
3.35.0   # MATERIALIZED, RETURNING keywords
3.36.0   3.37.0
3.38.0   # -> and ->> JSON operators — tokenizer change
3.39.0   3.40.0   3.41.0   3.42.0   3.43.0   3.44.0   3.45.0
3.46.0   # WITHIN keyword (ordered-set aggregates)
3.47.0   3.48.0   3.49.0   3.50.0
3.51.0   3.51.2   # latest
```

For patch releases, generally only the latest patch matters — tokenizer/keyword changes happen in .0 releases. But the tool should accept arbitrary version lists.

### 3.3 Source acquisition strategy

Support multiple strategies:

1. **Pre-downloaded directory** (default, most reliable) — user provides a directory:

   ```
   sqlite-sources/
     3.24.0/src/tokenize.c
     3.24.0/src/global.c
     3.24.0/src/sqliteInt.h
     3.24.0/tool/mkkeywordhash.c
     3.25.0/src/...
     ...
   ```

2. **GitHub raw download** — fetch individual files from the GitHub mirror:

   ```
   https://raw.githubusercontent.com/sqlite/sqlite/version-3.35.0/src/tokenize.c
   ```

3. **SQLite.org ZIPs** — fetch source archives. Requires year-to-version mapping.

A simple download script can populate the directory:

```bash
#!/bin/bash
VERSIONS="3.24.0 3.25.0 3.30.0 3.35.0 3.38.0 3.46.0 3.51.0 3.51.2"

for ver in $VERSIONS; do
    tag="version-${ver}"
    dir="sqlite-sources/${ver}"
    mkdir -p "$dir/src" "$dir/tool"
    for f in src/tokenize.c src/global.c src/sqliteInt.h tool/mkkeywordhash.c; do
        curl -sL "https://raw.githubusercontent.com/sqlite/sqlite/${tag}/${f}" \
            -o "$dir/$f"
    done
    echo "Downloaded $ver"
done
```

### 3.4 Extraction

For each downloaded version, run the existing `c_extractor::CExtractor` against the three source files. The extraction logic already exists in `sqlite_runtime_codegen.rs::extract_tokenizer()`. Factor this into a reusable function:

```rust
pub struct ExtractedFragments {
    pub cc_defines: String,
    pub ai_class: String,
    pub id_char: String,
    pub char_map: String,
    pub get_token: String,
    pub ctype_map: String,
    pub upper_to_lower: String,
    pub is_macros: String,
}

pub fn extract_fragments(
    tokenize_c: &str,
    global_c: &str,
    sqliteint_h: &str,
) -> Result<ExtractedFragments, String> {
    let tok = c_extractor::CExtractor::new(tokenize_c);
    let glob = c_extractor::CExtractor::new(global_c);
    let sqint = c_extractor::CExtractor::new(sqliteint_h);
    // Reuses the exact same c_extractor calls from the current pipeline
    ...
}
```

The CC\_\* define names to extract are listed in `sqlite_runtime_codegen.rs` lines 26-57.

#### Keyword extraction

The keyword table in `mkkeywordhash.c` has a regular format. Each entry:

```c
  { "ABORT",            "TK_ABORT",        CONFLICT|TRIGGER, 0      },
```

And mask defines:

```c
#ifdef SQLITE_OMIT_RETURNING
#  define RETURNING  0
#else
#  define RETURNING  0x00400000
#endif
```

Parse these with regex or a simple C parser:

```rust
pub struct KeywordEntry {
    pub name: String,        // "RETURNING"
    pub token: String,       // "TK_RETURNING"
    pub mask_expr: String,   // "RETURNING" (the mask symbol)
    pub priority: u32,       // 10
}

pub struct MaskDefine {
    pub name: String,        // "RETURNING"
    pub omit_flag: String,   // "SQLITE_OMIT_RETURNING"
    pub bit_value: u32,      // 0x00400000
}

pub struct KeywordTable {
    pub keywords: Vec<KeywordEntry>,
    pub masks: Vec<MaskDefine>,
}
```

### 3.5 Hashing and variant grouping

Hash each fragment's normalized text (strip comments, normalize whitespace, SHA-256) to identify distinct variants. Group consecutive versions with identical hashes:

```rust
pub struct VariantGroup {
    pub hash: String,
    pub versions: Vec<SqliteVersion>,
    pub first: SqliteVersion,
    pub last: SqliteVersion,
    pub text: String,
}

pub struct FragmentAnalysis {
    pub fragment_name: String,
    pub variants: Vec<VariantGroup>,
}
```

### 3.6 Diff generation

For each fragment with more than one variant, produce a unified diff between consecutive variants using a standard diff algorithm (e.g., the `similar` crate).

### 3.7 Output

The tool produces two outputs:

#### 1. Structured data (JSON to stdout)

JSON output to stdout (pipe to `jq` or redirect to file). No TOML — `serde_json` is already a CLI dependency, no new crates needed in codegen.

```json
{
  "meta": {
    "versions_processed": ["3.24.0", "3.25.0", "...", "3.51.2"],
    "extraction_date": "2026-02-22"
  },
  "fragments": {
    "get_token": {
      "variant_count": 4,
      "variants": [
        {
          "id": "v1",
          "hash": "sha256:abc123...",
          "versions": ["3.24.0", "3.25.0", "...", "3.37.2"],
          "first": "3.24.0",
          "last": "3.37.2"
        },
        {
          "id": "v2",
          "hash": "sha256:def456...",
          "versions": ["3.38.0", "...", "3.48.0"],
          "first": "3.38.0",
          "last": "3.48.0"
        }
      ]
    },
    "ai_class": {
      "variant_count": 1,
      "variants": [
        {
          "id": "v1",
          "hash": "...",
          "versions": ["..."],
          "first": "3.24.0",
          "last": "3.51.2"
        }
      ]
    },
    "keywords": {
      "total_keywords_latest": 148,
      "additions": [
        {
          "version": "3.25.0",
          "added": ["CURRENT", "EXCLUDE", "FILTER", "FOLLOWING", "..."]
        },
        { "version": "3.35.0", "added": ["MATERIALIZED", "RETURNING"] },
        { "version": "3.46.0", "added": ["WITHIN"] }
      ]
    }
  }
}
```

No separate markdown report — `jq` on the JSON output is sufficient for human review. Can always add later if needed.

#### 2. Raw variant files

```
output/variants/
  get_token_v1.c
  get_token_v2.c
  ...
  ai_class.c
  keywords/
    keywords_3_24_0.txt
    keywords_3_25_0.txt
    ...
```

### 3.8 CLI interface

```
syntaqlite analyze-versions \
    --sqlite-source-dir ./sqlite-sources/ \
    --output-dir ./analysis-output/
```

Source acquisition is handled by a separate bash download script (`tools/dev/download-sqlite-versions`), not by the Rust tool. No download logic in Rust.

### 3.9 Handling extraction failures

Older versions may have slightly different source structure. The `c_extractor` might fail on some fragments. The tool should:

1. Log the failure with version and fragment name
2. Continue processing other fragments/versions
3. Include failures in the report: "get_token: extraction failed for 3.24.0"
4. This tells the developer where manual intervention is needed

### 3.10 Expected results

| Fragment       | Expected variants | Notes                                       |
| -------------- | ----------------- | ------------------------------------------- |
| `get_token`    | 3-6               | Major: `->` (3.38), digit separator, others |
| `aiClass`      | 1-2               | Might change with new CC\_\* values         |
| CC\_\* defines | 1-2               | New defines for new token types             |
| `ctypeMap`     | 1                 | Frozen                                      |
| `upperToLower` | 1                 | Frozen                                      |
| `idChar`       | 1                 | Frozen                                      |
| `charMap`      | 1                 | Frozen                                      |
| `isMacros`     | 1                 | Frozen                                      |
| Keywords       | ~5 transitions    | Additions in 3.25, 3.30, 3.35, 3.46         |

### 3.11 Implementation location

New module: `syntaqlite-codegen/src/version_analysis/` (feature-gated behind `version-analysis`)

Files:

- `mod.rs` — public types (`SqliteVersion`, `ExtractedFragments`, `VersionAnalysis`, etc.) + orchestration
- `extract.rs` — fragment extraction (reuses `CExtractor`)
- `keywords.rs` — keyword table parsing from `mkkeywordhash.c`
- `hash.rs` — normalization + SHA-256 hashing
- `diff.rs` — variant grouping + unified diffs

Depends on:

- `syntaqlite_codegen::c_source::c_extractor::CExtractor` (existing, same crate)
- `similar` crate (for diffs, add to Cargo.toml as optional)
- `sha2` crate (for hashing, add to Cargo.toml as optional)

No `serde`, `toml`, or `download.rs` in codegen. Serialization happens in CLI via its existing `serde_json`. Download handled by bash script (`tools/dev/download-sqlite-versions`).

Wire into CLI via new subcommand in `syntaqlite-cli/src/codegen_sqlite.rs` (behind `version-analysis` feature).

---

## 4. Phase 2: Version-Annotated Artifacts

Once the analysis report exists, a developer creates the version-annotated artifacts. Guided by the report, verified by oracle tests.

### 4.1 Artifacts to produce

Checked into `syntaqlite-codegen/sqlite/versioned/`:

- `get_token_v1.c` ... `get_token_vN.c` — complete tokenizer function per variant
- `ai_class.c` — single version (or v1/v2 if it varies)
- `cc_defines_v1.h` / `cc_defines_v2.h` — if they differ
- `ctype_map.c`, `upper_to_lower.c`, `id_char.h`, `char_map.h`, `is_macros.h` — single version each
- `keywords.toml` — all keywords with `since` field
- `version_map.toml` — maps version ranges to variant files

### 4.2 How codegen consumes these

Current: `third_party/sqlite/src/tokenize.c` -> `c_extractor` -> `c_transformer` -> `sqlite_tokenize.c`

New: `versioned/get_token_v*.c` -> codegen assembles dispatch function -> `c_transformer` -> `sqlite_tokenize.c`

Generated `sqlite_tokenize.c`:

```c
#include "version_dispatch.h"
static const unsigned char aiClass[] = { ... };

static i64 synq_get_token_v1(const unsigned char *z, int *tokenType) { ... }
static i64 synq_get_token_v2(const unsigned char *z, int *tokenType) { ... }
static i64 synq_get_token_v3(const unsigned char *z, int *tokenType) { ... }

i64 SynqSqliteGetToken(const SyntaqliteDialect *d, const unsigned char *z, int *tokenType) {
#ifdef SYNQ_SQLITE_VERSION
    #if SYNQ_SQLITE_VERSION >= 3049000
        return synq_get_token_v3(z, tokenType);
    #elif SYNQ_SQLITE_VERSION >= 3038000
        return synq_get_token_v2(z, tokenType);
    #else
        return synq_get_token_v1(z, tokenType);
    #endif
#else
    if (d->sqlite_version >= 3049000) return synq_get_token_v3(z, tokenType);
    if (d->sqlite_version >= 3038000) return synq_get_token_v2(z, tokenType);
    return synq_get_token_v1(z, tokenType);
#endif
}
```

Generated `sqlite_keyword.c` — superset hash table plus `aKWVersion[]`:

```c
static const int synq_sqlite_aKWVersion[148] = {
    0,        /* ABORT */
    ...
    3350000,  /* RETURNING */
    ...
};

int synq_sqlite3_keywordCode(const SyntaqliteDialect *d, const char *z, int n, int *pType) {
    ...
    for (...) {
        ...
        if (SYNQ_VER_LT(d, synq_sqlite_aKWVersion[i])) continue;
        if (synq_sqlite_aKWMask[i] && !SYNQ_HAS_FEATURE(d, synq_sqlite_aKWMask[i])) continue;
        *pType = synq_sqlite_aKWCode[i];
        break;
    }
}
```

### 4.3 Dialect struct changes

```c
typedef struct SyntaqliteDialect {
    const char* name;
    int32_t  sqlite_version;    // NEW: e.g. 3035000
    uint32_t feature_flags;     // NEW: bitmask, 0xFFFFFFFF = all
    // ... existing fields unchanged ...
} SyntaqliteDialect;
```

### 4.4 Function signature change

`get_token` and `keywordCode` gain `const SyntaqliteDialect *d` as first parameter:

```c
// Was:   (d)->get_token(z, t)
// Now:   (d)->get_token(d, z, t)
```

### 4.5 Rust API additions

```rust
impl Dialect<'_> {
    pub fn with_version(&self, version: u32) -> OwnedDialect { ... }
    pub fn without_feature(&self, feature: Feature) -> OwnedDialect { ... }
    pub fn version_issues(&self, stmt: &Statement, target: u32) -> Vec<VersionIssue> { ... }
}
```

---

## 5. Phase 3: Oracle Test Generation

### 5.1 The oracle program

A small C program compiled against each SQLite version's amalgamation:

```c
// oracle_gen.c — compile with: cc oracle_gen.c sqlite3.c -o oracle
int sqlite3GetToken(const unsigned char*, int*);

int main() {
    const char *inputs[] = {
        "SELECT * FROM t",
        "SELECT x->>'key' FROM t",
        "SELECT 1_000_000",
        "SELECT RETURNING FROM t",
        "SELECT WITHIN FROM t",
        NULL
    };
    // tokenize each, output JSON: {type_name, length} per token
}
```

Run once per version. Output checked into `testdata/`.

### 5.2 CI tests

```rust
#[test]
fn tokenizer_matches_oracle_3_35() {
    let oracle = load_oracle("testdata/oracle_tokens_3_35_0.json");
    let dialect = sqlite_dialect().with_version(3_035_000);
    for entry in &oracle {
        let actual = tokenize_all(&dialect, &entry.input);
        assert_eq!(actual, entry.expected_tokens);
    }
}
```

---

## 6. Workflow: Adding a New SQLite Version

```
1. Download new version source

2. Run: syntaqlite analyze-versions --sqlite-source-dir ./sqlite-sources/
   Report: "GetToken: identical to v3" or "GetToken: NEW VARIANT"

3a. If identical: update version_map.toml range, done. (~30 seconds)

3b. If new variant:
    Copy extracted variant file into versioned/
    Add entry to version_map.toml
    Generate oracle test data
    Run tests, verify, commit (~30 minutes)
```

---

## 7. Key Design Decisions

| Decision                                     | Choice                        | Rationale                                                                         |
| -------------------------------------------- | ----------------------------- | --------------------------------------------------------------------------------- |
| Whole function variants vs line-level ifdefs | Whole variants                | ~200 lines x ~5 variants. Trivially small. Much easier to read, audit, debug.     |
| Backwards compat tracking                    | Not needed                    | SQLite is purely additive. Every check is `>= threshold`.                         |
| Runtime cflags mechanism                     | Same as runtime version       | Both resolve to AST validation. `SYNQ_HAS_FEATURE` macro, same dual-mode pattern. |
| Keyword storage                              | Superset hash + version array | One table, one extra comparison per probe. No per-version regeneration.           |
| Grammar runtime switching                    | Full parse + AST validation   | Better UX: "RETURNING requires 3.35+" vs cryptic syntax error.                    |
| Automation level                             | Semi-automated + oracle tests | Tool extracts/diffs/reports. Human annotates. Tests verify.                       |
| First phase                                  | Download + analysis only      | Produces the data for all subsequent decisions. No premature architecture.        |

---

## 8. File Layout After Full Implementation

```
syntaqlite-codegen/
  sqlite/
    lemon.c                           # existing
    lempar.c                          # existing
    mkkeywordhash.c                   # existing
    versioned/                        # NEW
      get_token_v1.c ... get_token_vN.c
      ai_class.c
      cc_defines_v1.h  cc_defines_v2.h
      ctype_map.c  upper_to_lower.c
      id_char.h  char_map.h  is_macros.h
      keywords.toml
      version_map.toml
  src/
    tools/
      version_analysis.rs             # NEW — Phase 1 tool
      mkkeyword.rs                    # existing
    sqlite_runtime_codegen.rs         # MODIFIED later — reads from versioned/

syntaqlite-cli/
  tests/testdata/
    oracle_tokens_3_24_0.json         # NEW — oracle data
    oracle_tokens_3_35_0.json
    oracle_tokens_3_38_0.json
    oracle_keywords_3_35_0.json
    ...

syntaqlite-runtime/
  include/
    syntaqlite/dialect.h              # MODIFIED — add sqlite_version, feature_flags
    syntaqlite_ext/version_dispatch.h # NEW — SYNQ_VER_GE / SYNQ_HAS_FEATURE
```

---

## 9. Reference: Current Extraction Pipeline

For context, this is how extraction currently works (the code Phase 2 eventually replaces):

**Build time** (`syntaqlite-codegen/build.rs`):

- Sets `SYNTAQLITE_SQLITE_TOKENIZE_C` env var pointing to `third_party/src/sqlite/src/tokenize.c`

**Codegen time** (`syntaqlite-codegen/src/lib.rs`):

- `embedded_sqlite_tokenize_c()` -> `include_str!(env!("SYNTAQLITE_SQLITE_TOKENIZE_C"))`
- `embedded_sqlite_global_c()` -> `include_str!("/../third_party/src/sqlite/src/global.c")`
- `embedded_sqlite_sqliteint_h()` -> `include_str!("/../third_party/src/sqlite/src/sqliteInt.h")`
- These are passed to `extract_tokenizer()` which runs `c_extractor` on each, then `c_transformer` to rename symbols.

**Keyword generation** (`sqlite_runtime_codegen.rs::generate_keyword_hash()`):

- Spawns `mkkeyword` subprocess (compiled from `syntaqlite-codegen/sqlite/mkkeywordhash.c`)
- Accepts optional `--extra-file` for dialect extension keywords
- Output processed by `c_transformer` to rename symbols

**Source files consumed:**

- `third_party/src/sqlite/src/tokenize.c` — GetToken, aiClass, CC\_\*, IdChar, charMap
- `third_party/src/sqlite/src/global.c` — CtypeMap, UpperToLower
- `third_party/src/sqlite/src/sqliteInt.h` — Isspace, Isdigit, Isxdigit macros
- `syntaqlite-codegen/sqlite/mkkeywordhash.c` — keyword table + masks
- `third_party/src/sqlite/src/parse.y` — referenced in build.rs for grammar

**c_extractor API** (`syntaqlite-codegen/src/c_source/c_extractor.rs`):

- `CExtractor::new(content: &str)` — wraps source text
- `.extract_function(name)` -> `CFunction { text }`
- `.extract_static_array(name)` -> `CStaticArray { text }`
- `.extract_specific_defines(names)` -> `CDefines { text }`
- `.extract_defines_with_ifdef_context(names)` -> `CDefines { text }`
