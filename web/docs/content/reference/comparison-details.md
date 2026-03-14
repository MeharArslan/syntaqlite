+++
title = "Comparison Details"
weight = 11
+++
# Comparison — detailed results

Per-statement breakdowns, reproduction instructions, and methodology for the
[competitive comparison](@/reference/comparison.md).

Generated on `arm64-darwin` with syntaqlite `0.1.0` on 2026-03-14.

---

# How to reproduce

```bash
# 1. Install all competitor tools (npm, cargo, brew, go, uv)
tools/run-comparison --setup

# 2. Run all comparisons and generate markdown
tools/run-comparison --all

# 3. Or run a single category
tools/run-comparison parser
tools/run-comparison formatter
tools/run-comparison validator
tools/run-comparison lsp
```

**Requirements:** macOS or Linux, with `npm`, `cargo`, `brew` (macOS),
`go`, `uv`, and `sqlite3` on PATH. The setup script installs:

| Tool | Source | Version pinning |
|------|--------|-----------------|
| syntaqlite | Built from this repo (`cargo build --release`) | Current HEAD |
| lemon-rs | `tests/comparison/parser/` (Cargo workspace) | Pinned in Cargo.lock |
| sqlparser-rs | `tests/comparison/parser/` (Cargo workspace) | Pinned in Cargo.lock |
| sql-parser-cst | `tests/comparison/package.json` (npm) | Pinned in package-lock |
| node-sql-parser | `tests/comparison/package.json` (npm) | Pinned in package-lock |
| sqlglot | `tests/comparison/pyproject.toml` (uv) | Pinned in uv.lock |
| sqlfluff | `tests/comparison/pyproject.toml` (uv) | Pinned in uv.lock |
| sql-formatter | npm global | Latest at install time |
| prettier + sql-parser-cst | npm + prettier plugin | Pinned in package-lock |
| sleek | `cargo install sleek` | Latest at install time |
| sqruff | `brew install sqruff` | Latest at install time |
| sql-lint | `tests/comparison/package.json` (npm) | Pinned in package-lock |
| sqls | `go install` | Latest at install time |
| sql-language-server | npm global | Latest at install time |
| hyperfine | `brew install hyperfine` | Latest at install time |

**Speed benchmarks** use [hyperfine](https://github.com/sharkdp/hyperfine) with
`--warmup 3` to fill filesystem caches. Each tool is invoked as a subprocess —
this measures end-to-end wall time including process startup, which is realistic
for CLI usage and editor integrations.

**Test statements** are in `tests/comparison/test_statements.sql` (40 statements)
and `tests/comparison/bench_statements.sql` (25 statements for speed benchmarks).
The accuracy suite uses deliberately tricky SQLite-specific syntax; the speed
suite uses representative real-world queries.

---

# Parser details

## Ground truth

Every test statement validated against `sqlite3` (via `EXPLAIN`):

{{PARSER_GROUND_TRUTH_TABLE}}

**{{PARSER_N_VALID}}/{{PARSER_TOTAL}}** statements validated by sqlite3.

## Per-statement results

Legend: **PASS** = correctly parses valid SQL, **FAIL** = rejects valid SQL, **FP** = accepts invalid SQL

{{PARSER_PER_STMT_TABLE}}

### Scoreboard

{{PARSER_SCOREBOARD}}

## Speed details

- `bench.sql`: {{PARSER_SPEED_1X_DESC}}
- `bench_30x.sql`: {{PARSER_SPEED_30X_DESC}}

### bench.sql (1×)

{{PARSER_SPEED_1X}}

### bench_30x.sql (30×)

{{PARSER_SPEED_30X}}

---

# Formatter details

## Ground truth

| Statement                                          | sqlite3 |
| -------------------------------------------------- | ------- |
| T01: Multi ON CONFLICT UPSERT + RETURNING          | OK      |
| T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZ | OK      |
| T03: CREATE TABLE STRICT + WITHOUT ROWID + generat | OK      |
| T04: UPDATE FROM + INDEXED BY                      | OK      |
| T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW  | OK      |
| T06: FILTER clause + IIF + NULLS LAST              | OK      |
| T07: ATTACH DATABASE                               | OK      |
| T08: INSERT OR REPLACE                             | OK      |
| T09: CREATE VIRTUAL TABLE (FTS5)                   | OK      |
| T10: PRAGMA                                        | OK      |
| T11: EXPLAIN QUERY PLAN                            | OK      |
| T12: ALTER TABLE DROP COLUMN                       | OK      |
| T13: ALTER TABLE RENAME COLUMN                     | OK      |
| T14: REINDEX                                       | OK      |
| T15: Window frame RANGE BETWEEN                    | OK      |
| T16: CREATE INDEX with WHERE (partial index)       | OK      |
| T17: REPLACE statement                             | OK      |
| T18: Nested window functions + EXCLUDE             | OK      |
| T19: GLOB and LIKE with ESCAPE                     | OK      |
| T20: INSERT with multiple VALUES + ON CONFLICT DO  | OK      |
| T21: Complex subquery expressions                  | OK      |
| T22: ANALYZE                                       | OK      |
| T23: SAVEPOINT / RELEASE / ROLLBACK TO             | OK      |
| T24: DROP TABLE IF EXISTS                          | OK      |
| T25: CREATE TABLE AS SELECT                        | OK      |
| T26: DETACH DATABASE                               | OK      |
| T27: UPSERT with complex expressions in DO UPDATE  | OK      |
| T28: WITH (non-recursive) + DELETE ... RETURNING   | OK      |
| T29: UPDATE ... RETURNING                          | OK      |
| T30: RIGHT JOIN + IS DISTINCT FROM                 | OK      |
| T31: FULL OUTER JOIN                               | OK      |
| T32: JSON -> and ->> operators                     | OK      |
| T33: Numeric literals with underscores             | OK      |
| T34: Multiple WINDOW definitions + nth_value + nti | OK      |
| T35: HAVING without GROUP BY (3.39+)               | OK      |
| T36: IS NOT DISTINCT FROM in complex expression    | OK      |
| T37: Blob literals + CAST chains                   | OK      |
| T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + com | OK      |
| T39: Deeply nested CTE + compound SELECT (UNION /  | OK      |
| T40: Window GROUPS frame + EXCLUDE TIES            | OK      |

**40/40** statements validated by sqlite3.

## Round-trip validation

For each formatter: does the formatted output still pass `sqlite3`?

| Test                                   | syntaqlite | prettier-cst | sql-formatter | sqlglot[c] |  sleek  |  sqruff |
| -------------------------------------- | :--------: | :----------: | :-----------: | :--------: | :-----: | :-----: |
| T01: Multi ON CONFLICT UPSERT + RETURN |     OK     |      OK      |      OK       |    FAIL    |   OK    |   OK    |
| T02: Recursive CTE + MATERIALIZED / NO |     OK     |      OK      |      OK       |  CORRUPT   |   OK    |   OK    |
| T03: CREATE TABLE STRICT + WITHOUT ROW |     OK     |      OK      |      OK       |    FAIL    |   OK    |   OK    |
| T04: UPDATE FROM + INDEXED BY          |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T05: CREATE TRIGGER + RAISE + WHEN + F |     OK     |      OK      |      OK       |  CORRUPT   |   OK    |   OK    |
| T06: FILTER clause + IIF + NULLS LAST  |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T07: ATTACH DATABASE                   |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T08: INSERT OR REPLACE                 |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T09: CREATE VIRTUAL TABLE (FTS5)       |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T10: PRAGMA                            |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T11: EXPLAIN QUERY PLAN                |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T12: ALTER TABLE DROP COLUMN           |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T13: ALTER TABLE RENAME COLUMN         |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T14: REINDEX                           |     OK     |      OK      |      OK       |  CORRUPT   |   OK    |   OK    |
| T15: Window frame RANGE BETWEEN        |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T16: CREATE INDEX with WHERE (partial  |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T17: REPLACE statement                 |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T18: Nested window functions + EXCLUDE |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T19: GLOB and LIKE with ESCAPE         |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T20: INSERT with multiple VALUES + ON  |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T21: Complex subquery expressions      |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T22: ANALYZE                           |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T23: SAVEPOINT / RELEASE / ROLLBACK TO |     OK     |      OK      |      OK       |  CORRUPT   |   OK    |   OK    |
| T24: DROP TABLE IF EXISTS              |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T25: CREATE TABLE AS SELECT            |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T26: DETACH DATABASE                   |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T27: UPSERT with complex expressions i |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T28: WITH (non-recursive) + DELETE ... |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T29: UPDATE ... RETURNING              |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T30: RIGHT JOIN + IS DISTINCT FROM     |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T31: FULL OUTER JOIN                   |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T32: JSON -> and ->> operators         |     OK     |      OK      |      OK       |     OK     |   OK    | CORRUPT |
| T33: Numeric literals with underscores |     OK     |     FAIL     |     FAIL      |    FAIL    | CORRUPT |   OK    |
| T34: Multiple WINDOW definitions + nth |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T35: HAVING without GROUP BY (3.39+)   |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T36: IS NOT DISTINCT FROM in complex e |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T37: Blob literals + CAST chains       |     OK     |      OK      |      OK       |     OK     | CORRUPT |   OK    |
| T38: GENERATED ALWAYS AS (VIRTUAL vs S |     OK     |      OK      |      OK       |    FAIL    |   OK    | CORRUPT |
| T39: Deeply nested CTE + compound SELE |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T40: Window GROUPS frame + EXCLUDE TIE |     OK     |      OK      |      OK       |    FAIL    |   OK    |   OK    |

### Scoreboard

| Tool          | Correct | Corrupt | Refused |
| ------------- | ------: | ------: | ------: |
| syntaqlite    |   40/40 |       - |       - |
| prettier-cst  |   39/40 |       - |       1 |
| sql-formatter |   39/40 |       - |       1 |
| sqlglot[c]    |   31/40 |       4 |       5 |
| sleek         |   38/40 |       2 |       - |
| sqruff        |   33/40 |       2 |       5 |

### Corruption details

| Tool       | Test                                                               | Error                                                        |
| ---------- | ------------------------------------------------------------------ | ------------------------------------------------------------ |
| sqlglot[c] | T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZED               | Error: in prepare, no such column: x
  LIZED (   VALUES      |
| sqlglot[c] | T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW                  | Error: in prepare, near "SELECT": syntax error
  yees', OLD. |
| sqlglot[c] | T14: REINDEX                                                       | Error: in prepare, near "AS": syntax error
  REINDEX AS idx_ |
| sqlglot[c] | T23: SAVEPOINT / RELEASE / ROLLBACK TO                             | Error: in prepare, near "AS": syntax error
  SAVEPOINT AS my |
| sqruff     | T32: JSON -> and ->> operators                                     | Error: in prepare, near ">": syntax error
  EXPLAIN SELECT   |
| sleek      | T33: Numeric literals with underscores                             | Error: in prepare, near "AS": syntax error
  EXPLAIN SELECT  |
| sleek      | T37: Blob literals + CAST chains                                   | Error: in prepare, near "AS": syntax error
  EXPLAIN SELECT  |
| sqruff     | T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + complex expressions | Error: in prepare, near "|": syntax error
  abel TEXT GENERA |

## Speed details

- `bench.sql`: 117 lines, 3,545 bytes
- `bench_30x.sql`: 3510 lines, 106,350 bytes

### bench.sql (1×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.8 ± 0.1 | 1.7 | 3.3 | 1.00 |
| `prettier-cst` | 396.5 ± 8.5 | 388.9 | 416.1 | 217.05 ± 16.18 |
| `sql-formatter` | 74.9 ± 1.2 | 72.5 | 78.1 | 41.01 ± 3.00 |
| `sqlglot[c]` | 87.2 ± 1.5 | 85.2 | 92.6 | 47.75 ± 3.50 |
| `sleek` | 8.3 ± 1.2 | 7.7 | 27.7 | 4.56 ± 0.75 |
| `sqruff` | 39.8 ± 4.0 | 38.5 | 73.8 | 21.77 ± 2.67 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 4.9 ± 0.1 | 4.7 | 5.5 | 1.00 |
| `prettier-cst` | 542.3 ± 4.3 | 535.1 | 545.3 | 111.47 ± 2.33 |
| `sql-formatter` | 200.3 ± 8.2 | 195.8 | 229.6 | 41.17 ± 1.87 |
| `sqlglot[c]` | 264.5 ± 1.5 | 262.6 | 267.2 | 54.37 ± 1.10 |
| `sleek` | 26.9 ± 0.4 | 26.1 | 27.9 | 5.52 ± 0.14 |
| `sqruff` | 3286.7 ± 53.0 | 3226.0 | 3349.7 | 675.63 ± 17.01 |


### Slow tools (single timed run)

| Tool          |  Time |
| ------------- | ----: |
| sqlfmt (1x)   | 204ms |
| sqlfmt (30x)  | 304ms |
| sqlfluff (1x) | 224ms |

---

# Validator details

## Diagnostic quality showcase

A realistic query with 2 subtle errors — how does each tool report them?

**Query** (CTE declares 3 columns but SELECT produces 2; typo `ROUDN`):

```sql
{{VALIDATOR_DEMO_QUERY}}
```

{{VALIDATOR_TOOL_OUTPUTS}}

## Per-case error detection

Schema: `users`, `orders`, `products`, `order_items`. Ground truth: sqlite3.

{{VALIDATOR_PER_CASE_TABLE}}

### Scoreboard

{{VALIDATOR_SCOREBOARD}}

## Speed details

- `bench.sql`: {{VALIDATOR_SPEED_1X_DESC}}
- `bench_30x.sql`: {{VALIDATOR_SPEED_30X_DESC}}

### bench.sql (1×)

{{VALIDATOR_SPEED_1X}}

### bench_30x.sql (30×)

{{VALIDATOR_SPEED_30X}}

---

# LSP details

## Tested capabilities

Each server is started, sent a test file, and probed for completion, hover,
diagnostics, and formatting via the LSP protocol.

{{LSP_FEATURES_TABLE}}

## Diagnostic detail

What each server reports for `SELEC * FROM users;` (syntax error):

{{LSP_DIAGNOSTIC_DETAIL}}

## Speed details

Time to start server, send document, receive diagnostics, and exit:

{{LSP_SPEED}}
