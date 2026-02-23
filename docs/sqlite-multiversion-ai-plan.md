# Multi-Version SQLite Support

## 1. Problem

Syntaqlite currently extracts its tokenizer, keyword table, and character tables from a single vendored SQLite source tree (`third_party/src/sqlite/`, currently 3.51.2). This pins every consumer to one SQLite version's keyword recognition and token behavior, and compiles with all features enabled (no `SQLITE_OMIT_*` support).

Real consumers need version- and cflag-aware behavior:

- **Embedded tools** targeting a specific SQLite version (e.g., Android's bundled 3.32, iOS's 3.39) need the keyword set and token behavior to match that version, potentially with specific `SQLITE_OMIT_*` flags active.
- **Developer tools** (LSPs, playgrounds, linters) need to switch target version and cflags at runtime without recompilation, e.g., "show me how SQLite 3.35 with `SQLITE_OMIT_WINDOWFUNC` parses this query."

## 2. Approach

### 2.1 Key findings from the version analysis

Phase 1 analyzed 66 SQLite versions (3.12.2–3.51.2). See `sqlite-version-analysis.md` for full results. Key findings:

- **SQLite is purely additive** — no backwards-incompatible removals across the entire version range. Every version check is `>= threshold`.
- The tokenizer has 10 raw variants, but only two behaviorally significant non-keyword differences: `TK_PTR` (3.38) and `TK_QNUMBER` (3.46).
- Keywords have 7 addition points (3.24–3.47), totaling 148 keywords in the latest version.
- The grammar grew from 326 to 411 rules, all removals are refactorings.
- `TK_COMMENT` was split from `TK_SPACE` in 3.49. Syntaqlite depends on `TK_COMMENT` everywhere and always emits it regardless of target version.

### 2.2 Two axes: version and cflags

Version and `SQLITE_OMIT_*` cflags are independent axes that both affect the SQL surface:

- **Version** determines which keywords and token types exist (additive over time).
- **CFlags** determine which features are compiled in. They primarily affect keyword recognition and grammar rules.

Both axes compose: a keyword is recognized only if it exists in the target version AND its feature isn't omitted by cflags.

### 2.3 Strategy: one tokenizer, version + cflag gating

Instead of maintaining multiple tokenizer variants, we use a single tokenizer (the latest, 3.51.2) and gate version/cflag-dependent behavior via a config struct passed as a parameter.

**Dual-mode dispatch** — the same generated code serves both compile-time and runtime:

- **Compile-time** (`-DSYNQ_SQLITE_VERSION=3035000`): caller passes a static const config. The compiler constant-folds all comparisons and eliminates dead branches. Zero runtime cost.
- **Runtime**: caller constructs a config and passes it. All branches are live but cheap (integer comparisons).

### 2.4 What gets version/cflag-gated

**1. Keywords** — the keyword lookup function checks two conditions per keyword:

- `since <= config->sqlite_version` — does this keyword exist in this version?
- cflag check with polarity — is this keyword's feature enabled given the active cflags?

When a keyword isn't recognized, the tokenizer returns `TK_ID`. The parser then naturally cannot match syntax that depends on that keyword — e.g., if `RETURNING` isn't a keyword, the RETURNING clause syntax is unparseable. No AST validation needed for keyword-gated features.

**2. Token reclassification** — a small postlude in `GetToken` handles non-keyword tokenizer differences:

| Token                  | Since | Reclassification for older versions                                                                                             |
| ---------------------- | ----- | ------------------------------------------------------------------------------------------------------------------------------- |
| `TK_PTR` (`->`, `->>`) | 3.38  | Set length=1, return `TK_MINUS`. Next call naturally tokenizes `>` as `TK_GT`.                                                  |
| `TK_QNUMBER` (`1_000`) | 3.46  | Find first `_` in token, truncate length to that position, return `TK_INTEGER` or `TK_FLOAT`. Next call sees `_000` as `TK_ID`. |

**3. Subquery flag** — `SQLITE_OMIT_SUBQUERY` is the only cflag that affects grammar rules without being keyword-gated (subqueries use `(SELECT ...)` — all baseline tokens). Instead of a post-parse AST walk, the parser action code sets a flag (`pParse->sawSubquery = 1`) when a subquery production is reduced. After parsing, if `SQLITE_OMIT_SUBQUERY` is active and the flag is set, emit a diagnostic. O(1) check.

### 2.5 What does NOT get version/cflag-gated

- **`TK_COMMENT`** — syntaqlite always emits `TK_COMMENT` for comments regardless of target version. Upstream only split this from `TK_SPACE` in 3.49, but syntaqlite depends on the distinction. Deliberate enhancement that doesn't affect parsing behavior.
- **Character tables** (`aiClass`, `cc_defines`, `ctypeMap`, `upperToLower`) — always use the latest. These are internal to the tokenizer we're already using.
- **Grammar** — always parse with the full latest grammar. Version enforcement happens at the token level (keyword suppression). Cflag enforcement is keyword suppression + the subquery flag.
- **Bug fixes** (NUL handling, BOM handling, i64 return type) — kept unconditionally.

### 2.6 Audit of grammar-affecting `SQLITE_OMIT_*` flags

All `%ifdef`/`%ifndef` blocks in `parse.y` (3.51.2) were audited. Every flag except `SQLITE_OMIT_SUBQUERY` gates syntax that is entered through a keyword, so keyword suppression is sufficient:

| Flag                                   | Grammar effect                 | Keyword-gatable?                              |
| -------------------------------------- | ------------------------------ | --------------------------------------------- |
| `SQLITE_OMIT_EXPLAIN`                  | `EXPLAIN` statement            | Yes — `EXPLAIN` keyword                       |
| `SQLITE_OMIT_TEMPDB`                   | `TEMP` in CREATE               | Yes — `TEMP` keyword                          |
| `SQLITE_OMIT_COMPOUND_SELECT`          | `UNION`/`INTERSECT`/`EXCEPT`   | Yes — keywords                                |
| `SQLITE_OMIT_WINDOWFUNC`               | Window functions (~130 lines)  | Yes — `WINDOW`/`OVER`/`PARTITION`/etc.        |
| `SQLITE_OMIT_GENERATED_COLUMNS`        | `GENERATED ALWAYS`             | Yes — keywords                                |
| `SQLITE_OMIT_VIEW`                     | `CREATE VIEW`                  | Yes — `VIEW` keyword                          |
| `SQLITE_OMIT_CTE`                      | `WITH` clauses                 | Yes — `WITH` keyword (only used for CTEs)     |
| `SQLITE_OMIT_SUBQUERY`                 | Subqueries in FROM, IN, EXISTS | **No** — uses `LP select RP`, baseline tokens |
| `SQLITE_OMIT_CAST`                     | `CAST(x AS type)`              | Yes — `CAST` keyword                          |
| `SQLITE_OMIT_PRAGMA`                   | `PRAGMA` statements            | Yes — `PRAGMA` keyword                        |
| `SQLITE_OMIT_TRIGGER`                  | `CREATE TRIGGER` (~115 lines)  | Yes — `TRIGGER` keyword                       |
| `SQLITE_OMIT_ATTACH`                   | `ATTACH`/`DETACH`              | Yes — keywords                                |
| `SQLITE_OMIT_REINDEX`                  | `REINDEX`                      | Yes — keyword                                 |
| `SQLITE_OMIT_ANALYZE`                  | `ANALYZE`                      | Yes — keyword                                 |
| `SQLITE_OMIT_ALTERTABLE`               | `ALTER TABLE`                  | Yes — `ALTER` keyword                         |
| `SQLITE_OMIT_VIRTUALTABLE`             | `CREATE VIRTUAL TABLE`         | Yes — `VIRTUAL` keyword                       |
| `SQLITE_ENABLE_ORDERED_SET_AGGREGATES` | `WITHIN GROUP` syntax          | Yes — `WITHIN` keyword                        |

### 2.7 What we no longer need (vs the original plan)

- ~~Multiple tokenizer function variants~~ → one tokenizer, postlude checks
- ~~`ai_class` / `cc_defines` variant tracking~~ → always latest
- ~~`version_map.toml` / versioned variant files~~ → not needed
- ~~`SYNQ_VER_GE` / `SYNQ_HAS_FEATURE` dispatch macros~~ → plain `if` on config struct fields
- ~~AST validation pass~~ → keyword gating + subquery flag handles everything
- ~~Versioned variant files in `syntaqlite-codegen/sqlite/versioned/`~~ → not needed

---

## 3. Phase 1: Download and Analysis Tool (COMPLETED)

Implemented in `syntaqlite-codegen/src/version_analysis/` (feature-gated behind `version-analysis`). Results in `sqlite-version-analysis.md`.

### 3.1 What was built

- **Download script** (`tools/dev/download-sqlite-versions`) — downloads SQLite source files from GitHub mirror for 40+ versions (3.12.2–3.51.2). Idempotent, skips existing files.
- **Analysis tool** (`syntaqlite analyze-versions`) — extracts 8 code fragments + keywords + grammar from each version, hashes for dedup, groups into variants, computes diffs. Outputs JSON to stdout + variant files + grammar report.
- **Modules**: `version_analysis/{mod,extract,hash,diff,keywords,grammar}.rs`
- **Dependencies**: `sha2` (hashing), `similar` (diffs), both optional behind `version-analysis` feature.

### 3.2 Key results

See `sqlite-version-analysis.md` for full analysis. Summary:

| Fragment         | Variants    | Notes                                          |
| ---------------- | ----------- | ---------------------------------------------- |
| `get_token`      | 10          | 2 behavioral: TK_PTR (3.38), TK_QNUMBER (3.46) |
| `ai_class`       | 6           | Coupled to tokenizer changes                   |
| `cc_defines`     | 4           | Coupled to tokenizer changes                   |
| `ctype_map`      | 2           | Quote char flag in 3.13                        |
| `upper_to_lower` | 2           | UBSAN tables in 3.36                           |
| `char_map`       | 1           | Frozen                                         |
| `id_char`        | 1           | Frozen                                         |
| `is_macros`      | 1           | Frozen                                         |
| Keywords         | 7 additions | 3.24–3.47, 148 total                           |
| Grammar          | 24 changes  | 326→411 rules, semantically additive           |

---

## 4. Phase 2: Config Struct and Version/CFlag Gating

### 4.1 The config struct

The config struct lives in **`syntaqlite-runtime`** (not the dialect crate), because every dialect needs version + cflag configuration — this is shared infrastructure.

```c
// syntaqlite-runtime/include/syntaqlite/dialect_config.h

typedef struct SyntaqliteDialectConfig {
    int32_t  sqlite_version;   // Target version (e.g., 3035000). 0 = latest.
    uint32_t cflags;           // Bitmask of active cflags. See below for semantics.
} SyntaqliteDialectConfig;
```

The `cflags` field is a bitmask representing the state of `SQLITE_OMIT_*` and `SQLITE_ENABLE_*` flags. Each flag maps to a named constant defined in the runtime — these are fixed SQLite concepts shared across all dialects:

```c
// syntaqlite-runtime/include/syntaqlite/sqlite_cflags.h
#define SYNQ_SQLITE_OMIT_WINDOWFUNC                0x00000001
#define SYNQ_SQLITE_OMIT_RETURNING                 0x00000002
#define SYNQ_SQLITE_OMIT_CTE                       0x00000004
#define SYNQ_SQLITE_OMIT_SUBQUERY                  0x00000008
#define SYNQ_SQLITE_ENABLE_ORDERED_SET_AGGREGATES   0x00000010
// ... etc
```

**Polarity handling**: `SQLITE_OMIT_*` and `SQLITE_ENABLE_*` flags have opposite default states. The codegen handles this by encoding the polarity into the keyword mask table. Each keyword entry stores both a cflag bit and a polarity flag:

- `SQLITE_OMIT_*`: the keyword is active by **default**. Setting the bit in `cflags` **disables** it.
- `SQLITE_ENABLE_*`: the keyword is inactive by **default**. Setting the bit in `cflags` **enables** it.

The keyword lookup encodes both: `aKWCFlag[i]` stores the bit, `aKWCFlagPolarity[i]` stores whether setting the bit enables or disables the keyword. See section 4.4 for the check logic.

Users configure in terms of the `SQLITE_OMIT_*` / `SQLITE_ENABLE_*` names they know:

```c
SyntaqliteDialectConfig config = {
    .sqlite_version = 3035000,
    .cflags = SYNQ_SQLITE_OMIT_WINDOWFUNC
            | SYNQ_SQLITE_OMIT_CTE
            | SYNQ_SQLITE_ENABLE_ORDERED_SET_AGGREGATES,
};
```

A zero-initialized config means "latest version, default cflags (all OMIT off, all ENABLE off)" — identical to current behavior.

### 4.2 Function signature changes

`GetToken` and `keywordCode` gain a `const SyntaqliteDialectConfig *` parameter:

```c
// Was:
i64 SynqSqliteGetToken(const unsigned char *z, int *tokenType);

// Now:
i64 SynqSqliteGetToken(const SyntaqliteDialectConfig *config,
                        const unsigned char *z, int *tokenType);
```

### 4.3 `GetToken` postlude

At the end of `SynqSqliteGetToken`, before returning:

```c
  // Version-dependent token reclassification.
  if (config && config->sqlite_version != 0) {
    if (*tokenType == SYNTAQLITE_TK_PTR && config->sqlite_version < 3038000) {
      // -> and ->> operators added in 3.38.
      // Return just the '-' as TK_MINUS; next call picks up '>' naturally.
      *tokenType = SYNTAQLITE_TK_MINUS;
      return 1;
    }
    if (*tokenType == SYNTAQLITE_TK_QNUMBER && config->sqlite_version < 3046000) {
      // Digit separators added in 3.46.
      // Truncate to the first underscore.
      i64 j;
      int saw_dot = 0;
      for (j = 0; j < i; j++) {
        if (z[j] == '_') break;
        if (z[j] == '.') saw_dot = 1;
      }
      *tokenType = saw_dot ? SYNTAQLITE_TK_FLOAT : SYNTAQLITE_TK_INTEGER;
      return j;
    }
  }
```

When `config` is NULL or `sqlite_version` is 0, this is a single branch-not-taken. When compiled with `-DSYNQ_SQLITE_VERSION=0`, the compiler eliminates the entire block.

### 4.4 Keyword version + cflag gating

The keyword table (generated by `mkkeywordhash`) gains parallel arrays for version and cflag gating:

```c
// Introduction version for each keyword (0 = always present).
static const int32_t synq_sqlite_aKWSince[148] = {
    0,          /* ABORT */
    0,          /* ACTION */
    ...
    3035000,    /* RETURNING */
    3047000,    /* WITHIN */
};

// CFlag bit for each keyword (0 = no cflag dependency).
static const uint32_t synq_sqlite_aKWCFlag[148] = {
    0,                                          /* ABORT */
    ...
    SYNQ_SQLITE_OMIT_RETURNING,                 /* RETURNING */
    SYNQ_SQLITE_OMIT_WINDOWFUNC,                /* WINDOW */
    SYNQ_SQLITE_ENABLE_ORDERED_SET_AGGREGATES,  /* WITHIN */
    ...
};

// Polarity: 0 = OMIT (keyword active by default, bit disables it),
//           1 = ENABLE (keyword inactive by default, bit enables it).
static const uint8_t synq_sqlite_aKWCFlagPolarity[148] = {
    0,  /* ABORT */
    ...
    0,  /* RETURNING — OMIT flag */
    0,  /* WINDOW — OMIT flag */
    1,  /* WITHIN — ENABLE flag */
    ...
};
```

The keyword lookup function checks both version and cflags, respecting polarity:

```c
int SynqSqliteKeywordCode(const SyntaqliteDialectConfig *config,
                           const char *z, int n, int *pType) {
    ...
    // Version check: skip keywords newer than target version.
    if (config && config->sqlite_version != 0
        && synq_sqlite_aKWSince[i] > config->sqlite_version) {
      break;  // not a keyword, fall through to TK_ID
    }
    // CFlag check with polarity:
    //   OMIT (polarity=0): keyword disabled when bit IS set
    //   ENABLE (polarity=1): keyword disabled when bit IS NOT set
    if (config && synq_sqlite_aKWCFlag[i] != 0) {
      int bit_set = (synq_sqlite_aKWCFlag[i] & config->cflags) != 0;
      int is_enable = synq_sqlite_aKWCFlagPolarity[i];
      if (bit_set != is_enable) {
        break;  // cflag disables this keyword, fall through to TK_ID
      }
    }
    *pType = synq_sqlite_aKWCode[i];
    break;
}
```

The polarity logic: for OMIT flags (`polarity=0`), the keyword is disabled when the bit is set (`bit_set=1, is_enable=0, 1 != 0 → skip`). For ENABLE flags (`polarity=1`), the keyword is disabled when the bit is NOT set (`bit_set=0, is_enable=1, 0 != 1 → skip`).

Most keywords have `since = 0` and `cflag = 0`, so both checks are almost always false.

### 4.5 Keyword `since` data

From the phase 1 analysis:

```
since 0:        All keywords present in 3.12.2 (the baseline)
since 3024000:  DO, NOTHING
since 3025000:  CURRENT, FILTER, FOLLOWING, OVER, PARTITION,
                PRECEDING, RANGE, ROWS, UNBOUNDED, WINDOW
since 3028000:  EXCLUDE, GROUPS, OTHERS, TIES
since 3030000:  FIRST, LAST, NULLS
since 3031000:  ALWAYS, GENERATED
since 3035000:  MATERIALIZED, RETURNING
since 3047000:  WITHIN
```

### 4.6 Keyword-to-cflag mapping

Each `SQLITE_OMIT_*` flag suppresses specific keywords:

| CFlag                                  | Keywords suppressed                                                                                                   |
| -------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| `SQLITE_OMIT_WINDOWFUNC`               | WINDOW, OVER, PARTITION, CURRENT, FOLLOWING, PRECEDING, RANGE, ROWS, UNBOUNDED, FILTER, EXCLUDE, GROUPS, OTHERS, TIES |
| `SQLITE_OMIT_CTE`                      | WITH                                                                                                                  |
| `SQLITE_OMIT_COMPOUND_SELECT`          | UNION, INTERSECT, EXCEPT                                                                                              |
| `SQLITE_OMIT_GENERATED_COLUMNS`        | GENERATED, ALWAYS                                                                                                     |
| `SQLITE_OMIT_RETURNING`                | RETURNING                                                                                                             |
| `SQLITE_OMIT_CAST`                     | CAST                                                                                                                  |
| `SQLITE_OMIT_EXPLAIN`                  | EXPLAIN                                                                                                               |
| `SQLITE_OMIT_VIEW`                     | VIEW                                                                                                                  |
| `SQLITE_OMIT_TRIGGER`                  | TRIGGER                                                                                                               |
| `SQLITE_OMIT_ATTACH`                   | ATTACH, DETACH                                                                                                        |
| `SQLITE_OMIT_PRAGMA`                   | PRAGMA                                                                                                                |
| `SQLITE_OMIT_REINDEX`                  | REINDEX                                                                                                               |
| `SQLITE_OMIT_ANALYZE`                  | ANALYZE                                                                                                               |
| `SQLITE_OMIT_ALTERTABLE`               | ALTER                                                                                                                 |
| `SQLITE_OMIT_VIRTUALTABLE`             | VIRTUAL                                                                                                               |
| `SQLITE_OMIT_TEMPDB`                   | TEMP                                                                                                                  |
| `SQLITE_ENABLE_ORDERED_SET_AGGREGATES` | WITHIN (note: enable flag, not omit)                                                                                  |

This mapping is already encoded in `mkkeywordhash.c`'s mask table. The codegen pipeline extracts it.

### 4.7 `SQLITE_OMIT_SUBQUERY` — the special case

This is the only cflag that affects grammar rules without being keyword-gated. Subqueries use `(SELECT ...)` — all baseline tokens that can't be suppressed.

**Solution**: set a flag on the parse context when a subquery production fires.

The `%ifndef SQLITE_OMIT_SUBQUERY` blocks in `parse.y` guard three groups of rules (subqueries in FROM, IN, and expression position). Since the grammar is **not autogenerated** (it's upstream SQLite with manual modifications), we add the flag directly to the grammar action code by hand:

```c
// Added to subquery grammar actions in parse.y:
pParse->sawSubquery = 1;
```

After parsing completes, the caller checks:

```c
if ((config->cflags & SYNQ_SQLITE_OMIT_SUBQUERY) && pParse->sawSubquery) {
    // emit diagnostic: "subqueries not available with SQLITE_OMIT_SUBQUERY"
}
```

This is O(1) — no AST walk needed.

### 4.8 Dispatch macro update

The `SYNQ_GET_TOKEN` macro gains the config parameter:

```c
// syntaqlite/csrc/sqlite_dialect_dispatch.h (inline/amalgamation mode):
#define SYNQ_GET_TOKEN(d, cfg, z, t)  SynqSqliteGetToken(cfg, z, t)

// syntaqlite-runtime/csrc/dialect_dispatch.h (function pointer mode):
#define SYNQ_GET_TOKEN(d, cfg, z, t)  (d)->get_token(cfg, z, t)
```

The `get_token` function pointer signature in `SyntaqliteDialect` updates to:

```c
int64_t (*get_token)(const SyntaqliteDialectConfig *config,
                     const unsigned char *z, int *tokenType);
```

The config struct is defined in the runtime — all dialects share it.

### 4.9 Version/config lives on the parser

The `SyntaqliteDialect` struct is static and immutable. The config is per-parser session:

```c
// New API in parser.h:
int syntaqlite_parser_set_dialect_config(SyntaqliteParser *p,
                                         const SyntaqliteDialectConfig *config);
```

The parser stores the config pointer and passes it through to `SYNQ_GET_TOKEN` calls in `parser.c` and `tokenizer.c`. A NULL config means "latest, default cflags" — current behavior.

```c
SyntaqliteDialectConfig config = {
    .sqlite_version = 3035000,
    .cflags = SYNQ_SQLITE_OMIT_WINDOWFUNC,
};
syntaqlite_parser_set_dialect_config(parser, &config);
```

### 4.10 Compile-time mode

For compile-time version pinning, define `SYNQ_SQLITE_VERSION` and/or `SYNQ_SQLITE_CFLAGS`:

```c
// -DSYNQ_SQLITE_VERSION=3035000 -DSYNQ_SQLITE_CFLAGS=SYNQ_SQLITE_OMIT_WINDOWFUNC

// In dialect_dispatch.h:
#ifdef SYNQ_SQLITE_VERSION
static const SyntaqliteDialectConfig SYNQ_STATIC_CONFIG = {
    .sqlite_version = SYNQ_SQLITE_VERSION,
#ifdef SYNQ_SQLITE_CFLAGS
    .cflags = SYNQ_SQLITE_CFLAGS,
#else
    .cflags = 0,
#endif
};
#define SYNQ_GET_TOKEN(d, cfg, z, t)  SynqSqliteGetToken(&SYNQ_STATIC_CONFIG, z, t)
#endif
```

The compiler sees through the pointer to the static const and constant-folds all comparisons.

### 4.11 Rust-side changes

```rust
// In syntaqlite-runtime — mirrors the C struct:
pub struct DialectConfig {
    pub sqlite_version: i32,    // 0 = latest
    pub cflags: u32,            // 0 = default cflags
}

// Parser gains config setter:
impl Parser {
    pub fn set_dialect_config(&mut self, config: &DialectConfig) { ... }
}
```

### 4.12 Code ownership: everything in `syntaqlite/` stays generated

A key invariant: **`syntaqlite/csrc/` is 100% generated code** (`@generated` marker). All version/cflag logic — the `GetToken` postlude, the `aKWSince[]`/`aKWCFlag[]`/`aKWCFlagPolarity[]` arrays, the keyword version/cflag checks — is emitted by the codegen pipeline. No hand-written C files in the dialect crate. The config struct and cflag constants live in the runtime (hand-written, shared across all dialects).

The codegen pipeline already transforms extracted code (symbol renaming via `c_transformer`, fragment assembly). Injecting the postlude into `GetToken` and emitting version/cflag data alongside the keyword table is the same kind of transformation. The "hand-written" decisions (which tokens to reclassify, version thresholds, cflag mappings) live as logic in `syntaqlite-codegen/src/`, not as C source files.

This keeps the scope manageable: two `if` checks in the postlude, two data arrays for keywords. If the version-gating logic grows significantly in the future, the crate structure can be revisited — but for now, codegen injection is the right trade-off.

### 4.13 Codegen changes

Generated output changes (all emitted by the codegen pipeline):

| File                        | Change                                                                                    |
| --------------------------- | ----------------------------------------------------------------------------------------- |
| `sqlite_tokenize.c`         | `SynqSqliteGetToken` gains `const SyntaqliteDialectConfig *` param + postlude (generated) |
| `sqlite_keyword.c`          | `aKWSince[]` + `aKWCFlag[]` + `aKWCFlagPolarity[]` arrays + checks in lookup (generated)  |
| `sqlite_dialect_dispatch.h` | `SYNQ_GET_TOKEN` macro gains config param, compile-time mode (generated)                  |
| `dialect.c`                 | Function pointer assignment (signature update)                                            |

Codegen pipeline changes (where the logic lives):

| Module                         | Change                                                                                                    |
| ------------------------------ | --------------------------------------------------------------------------------------------------------- |
| `sqlite_runtime_codegen.rs`    | Tokenizer extraction: inject config param + postlude into `GetToken`                                      |
| `tools/mkkeyword.rs`           | Keyword generation: emit `aKWSince[]` + `aKWCFlag[]` + `aKWCFlagPolarity[]` arrays + version/cflag checks |
| `dialect_codegen/c_dialect.rs` | Dispatch macro generation: add config param                                                               |

Runtime changes (hand-written, dialect-agnostic):

| File                                                     | Change                                                       |
| -------------------------------------------------------- | ------------------------------------------------------------ |
| `syntaqlite-runtime/include/syntaqlite/dialect_config.h` | NEW: `SyntaqliteDialectConfig` struct definition             |
| `syntaqlite-runtime/include/syntaqlite/sqlite_cflags.h`  | NEW: `SYNQ_SQLITE_OMIT_*` / `SYNQ_SQLITE_ENABLE_*` constants |
| `syntaqlite-runtime/include/syntaqlite/dialect.h`        | `get_token` function pointer signature                       |
| `syntaqlite-runtime/include/syntaqlite/parser.h`         | `syntaqlite_parser_set_dialect_config()` API                 |
| `syntaqlite-runtime/csrc/parser.c`                       | Store config pointer, pass to `SYNQ_GET_TOKEN`               |
| `syntaqlite-runtime/csrc/tokenizer.c`                    | Pass config to `SYNQ_GET_TOKEN`                              |
| `syntaqlite-runtime/csrc/dialect_dispatch.h`             | `SYNQ_GET_TOKEN` macro gains config param                    |
| `syntaqlite-runtime/src/dialect/ffi.rs`                  | `get_token` field type update, `DialectConfig` struct        |
| `syntaqlite-runtime/src/parser/session.rs`               | Expose `set_dialect_config`                                  |

---

## 5. Phase 3: Oracle Tests

### 5.1 Purpose

Verify that syntaqlite's version/cflag-gated tokenizer and keyword behavior matches real SQLite at key version boundaries.

### 5.2 Test corpus

SQL inputs exercising version-dependent and cflag-dependent behavior:

```sql
-- Keywords: should be TK_ID before their introduction version
SELECT returning FROM t;
SELECT materialized FROM t;
SELECT within FROM t;
SELECT window FROM t;
SELECT filter FROM t;

-- TK_PTR: should be TK_MINUS + TK_GT before 3.38
SELECT x->'key' FROM t;
SELECT x->>'key' FROM t;

-- TK_QNUMBER: should be TK_INTEGER + TK_ID before 3.46
SELECT 1_000;
SELECT 1_000.5;
SELECT 0x1_FF;

-- CFlag-gated keywords
EXPLAIN SELECT 1;                   -- SQLITE_OMIT_EXPLAIN
SELECT * FROM (SELECT 1);           -- SQLITE_OMIT_SUBQUERY
SELECT 1 UNION SELECT 2;            -- SQLITE_OMIT_COMPOUND_SELECT
WITH cte AS (SELECT 1) SELECT * FROM cte;  -- SQLITE_OMIT_CTE
SELECT CAST(1 AS TEXT);             -- SQLITE_OMIT_CAST

-- Baseline: should tokenize identically across all versions
SELECT * FROM t WHERE x = 1;
CREATE TABLE t(a INTEGER, b TEXT);
```

### 5.3 Oracle generation

A small C program compiled against each real SQLite version's amalgamation. It tokenizes the test corpus and outputs JSON with `{token_type, length, text}` per token:

```c
// tools/dev/oracle_tokenizer.c
// Compile: cc oracle_tokenizer.c sqlite3.c -o oracle
int sqlite3GetToken(const unsigned char*, int*);
int main() { /* tokenize inputs, output JSON */ }
```

Run once per version. Output checked into `testdata/`.

### 5.4 CI tests

```rust
#[test]
fn version_gated_tokenizer_matches_oracle() {
    let test_versions = [
        (3_024_000, "testdata/oracle_tokens_3_24_0.json"),
        (3_035_000, "testdata/oracle_tokens_3_35_0.json"),
        (3_038_000, "testdata/oracle_tokens_3_38_0.json"),
        (3_046_000, "testdata/oracle_tokens_3_46_0.json"),
    ];
    for (version, oracle_path) in &test_versions {
        let oracle = load_oracle(oracle_path);
        let config = DialectConfig { sqlite_version: *version, cflags: 0 };
        let mut parser = Parser::new();
        parser.set_dialect_config(&config);
        for entry in &oracle {
            let tokens = tokenize_all(&parser, &entry.input);
            // TK_COMMENT vs TK_SPACE mismatch is expected for < 3.49
            // (syntaqlite always emits TK_COMMENT, real SQLite < 3.49 emits TK_SPACE)
            assert_tokens_match(&tokens, &entry.expected, *version);
        }
    }
}
```

### 5.5 CFlag tests

```rust
#[test]
fn omit_windowfunc_suppresses_keywords() {
    let config = DialectConfig {
        sqlite_version: 0,  // latest
        cflags: SYNQ_SQLITE_OMIT_WINDOWFUNC,
    };
    // "WINDOW" should tokenize as TK_ID, not TK_WINDOW
    let tokens = tokenize_with_config("SELECT window FROM t", &config);
    assert_eq!(tokens[1].token_type, TK_ID);
}

#[test]
fn omit_subquery_sets_flag() {
    let config = DialectConfig {
        sqlite_version: 0,
        cflags: SYNQ_SQLITE_OMIT_SUBQUERY,
    };
    // Should parse successfully (full grammar) but flag subquery usage
    let result = parse_with_config("SELECT * FROM (SELECT 1)", &config);
    assert!(result.saw_subquery);
    // The caller decides what to do with this — error, warning, etc.
}
```

### 5.6 Oracle generation tooling

```
tools/dev/generate-oracle-data    # downloads amalgamations, compiles, runs
tools/dev/oracle_tokenizer.c      # the C program
```

Only runs once (or when adding new test inputs). Oracle JSON files are checked in.

---

## 6. Implementation Order

1. **Config struct + signature changes** — define `SyntaqliteDialectConfig`, update `GetToken` and `keywordCode` to accept `const SyntaqliteDialectConfig *`. Wire through dispatch macros, parser, tokenizer. All callers pass NULL (latest). No behavioral change.

2. **`GetToken` postlude** — add the `TK_PTR` and `TK_QNUMBER` reclassification block. Dead code when config is NULL.

3. **Keyword `since` + cflag arrays** — add `aKWSince[]`, `aKWCFlag[]`, and `aKWCFlagPolarity[]` to the `mkkeywordhash` codegen pipeline. Add version + cflag checks to keyword lookup. Dead code when config is NULL.

4. **Subquery flag** — add `sawSubquery` field to parse context. Set it in the subquery grammar actions. Check after parsing when `SQLITE_OMIT_SUBQUERY` is active.

5. **Parser API** — add `syntaqlite_parser_set_config()`. Wire to Rust side.

6. **Compile-time mode** — add `SYNQ_SQLITE_VERSION` / `SYNQ_SQLITE_OMIT_FLAGS` support to dispatch headers.

7. **Oracle tests** — generate oracle data, write test harness, verify against key version boundaries and cflag combinations.

---

## 7. Workflow: Adding a New SQLite Version

```
1. Download new version source

2. Run: syntaqlite analyze-versions --sqlite-source-dir ./sqlite-sources/
   Compare keyword list and tokenizer fragments.

3. If new keywords: add entries to the `since` data in codegen, re-run codegen.

4. If tokenizer behavioral change (new token type): add reclassification
   case to the GetToken postlude.

5. Generate oracle test data for the new version.

6. Run tests, verify, commit.
```

---

## 8. Key Design Decisions

| Decision                     | Choice                              | Rationale                                                                                                   |
| ---------------------------- | ----------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Single tokenizer vs variants | Single + postlude                   | Only 2 behavioral differences (TK_PTR, TK_QNUMBER). Postlude is simpler than managing 10 function variants. |
| Config location              | Parser, not dialect                 | Dialect is static const. Version/cflags are per-session configuration.                                      |
| Config representation        | Struct pointer                      | Extensible, compiler can see through `const` pointer to static const for optimization.                      |
| CFlag naming                 | `SQLITE_OMIT_*` names               | Users reason in SQLite terms. Direct mapping from mkkeywordhash masks.                                      |
| Grammar cflag enforcement    | Keyword suppression + subquery flag | 16/17 cflags are keyword-gated. Only SQLITE_OMIT_SUBQUERY needs special handling (O(1) flag check).         |
| TK_COMMENT handling          | Always emit                         | Syntaqlite depends on comment/space distinction. Deliberate divergence from pre-3.49 SQLite.                |
| Backwards compat tracking    | Not needed                          | SQLite is purely additive. Every check is `>= threshold`.                                                   |

---

## 9. Cost Summary

| Component                 | Runtime cost (when versioning active) | Runtime cost (when not active) |
| ------------------------- | ------------------------------------- | ------------------------------ |
| Keyword version check     | 1 int comparison per keyword probe    | 0 (NULL config check)          |
| Keyword cflag check       | 1 bitmask check per keyword probe     | 0                              |
| GetToken postlude         | 2 `if` checks (TK_PTR, TK_QNUMBER)    | 0 (NULL config check)          |
| Subquery flag             | 1 assignment per subquery parsed      | 0                              |
| Post-parse subquery check | 1 bitmask + flag check                | 0                              |
| **Total**                 | **Negligible**                        | **Zero**                       |

---

## 10. Reference: Current Extraction Pipeline

For context, this is how extraction currently works:

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
