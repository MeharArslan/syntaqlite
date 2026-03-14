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

## Per-statement results

Legend: **PASS** = correctly parses valid SQL, **FAIL** = rejects valid SQL, **FP** = accepts invalid SQL

| Test                                   | sqlite3 | syntaqlite | lemon-rs | sql-parser-cst | sqlglot[c] | sqlfluff | sqlparser-rs | node-sql-parser |
| -------------------------------------- | :-----: | :--------: | :------: | :------------: | :--------: | :------: | :----------: | :-------------: |
| T01: Multi ON CONFLICT UPSERT + RETURN |   OK    |    PASS    |   PASS   |      PASS      |    FAIL    |   FAIL   |     FAIL     |      FAIL       |
| T02: Recursive CTE + MATERIALIZED / NO |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     FAIL     |      FAIL       |
| T03: CREATE TABLE STRICT + WITHOUT ROW |   OK    |    PASS    |   PASS   |      PASS      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T04: UPDATE FROM + INDEXED BY          |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     FAIL     |      FAIL       |
| T05: CREATE TRIGGER + RAISE + WHEN + F |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     FAIL     |      PASS       |
| T06: FILTER clause + IIF + NULLS LAST  |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T07: ATTACH DATABASE                   |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     PASS     |      PASS       |
| T08: INSERT OR REPLACE                 |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T09: CREATE VIRTUAL TABLE (FTS5)       |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     FAIL     |      FAIL       |
| T10: PRAGMA                            |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T11: EXPLAIN QUERY PLAN                |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     PASS     |      FAIL       |
| T12: ALTER TABLE DROP COLUMN           |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T13: ALTER TABLE RENAME COLUMN         |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T14: REINDEX                           |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     FAIL     |      FAIL       |
| T15: Window frame RANGE BETWEEN        |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T16: CREATE INDEX with WHERE (partial  |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T17: REPLACE statement                 |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T18: Nested window functions + EXCLUDE |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     FAIL     |      FAIL       |
| T19: GLOB and LIKE with ESCAPE         |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     FAIL     |      FAIL       |
| T20: INSERT with multiple VALUES + ON  |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T21: Complex subquery expressions      |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T22: ANALYZE                           |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     FAIL     |      FAIL       |
| T23: SAVEPOINT / RELEASE / ROLLBACK TO |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     PASS     |      FAIL       |
| T24: DROP TABLE IF EXISTS              |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T25: CREATE TABLE AS SELECT            |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T26: DETACH DATABASE                   |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   FAIL   |     FAIL     |      FAIL       |
| T27: UPSERT with complex expressions i |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T28: WITH (non-recursive) + DELETE ... |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     FAIL     |      FAIL       |
| T29: UPDATE ... RETURNING              |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T30: RIGHT JOIN + IS DISTINCT FROM     |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T31: FULL OUTER JOIN                   |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T32: JSON -> and ->> operators         |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T33: Numeric literals with underscores |   OK    |    PASS    |   PASS   |      FAIL      |    FAIL    |   FAIL   |     FAIL     |      FAIL       |
| T34: Multiple WINDOW definitions + nth |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T35: HAVING without GROUP BY (3.39+)   |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T36: IS NOT DISTINCT FROM in complex e |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T37: Blob literals + CAST chains       |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      PASS       |
| T38: GENERATED ALWAYS AS (VIRTUAL vs S |   OK    |    PASS    |   PASS   |      PASS      |    FAIL    |   PASS   |     PASS     |      PASS       |
| T39: Deeply nested CTE + compound SELE |   OK    |    PASS    |   PASS   |      PASS      |    PASS    |   PASS   |     PASS     |      FAIL       |
| T40: Window GROUPS frame + EXCLUDE TIE |   OK    |    PASS    |   PASS   |      PASS      |    FAIL    |   PASS   |     FAIL     |      FAIL       |

### Scoreboard

| Tool            | Correct                           | Rejects Valid | Accepts Invalid |
| --------------- | --------------------------------- | ------------: | --------------: |
| syntaqlite      | 40/40 (100%) ████████████████████ |             - |               - |
| lemon-rs        | 40/40 (100%) ████████████████████ |             - |               - |
| sql-parser-cst  | 39/40 (97%) ███████████████████   |             1 |               - |
| sqlglot[c]      | 35/40 (87%) █████████████████     |             5 |               - |
| sqlfluff        | 29/40 (72%) ██████████████        |            11 |               - |
| sqlparser-rs    | 26/40 (65%) █████████████         |            14 |               - |
| node-sql-parser | 15/40 (37%) ███████               |            25 |               - |

## Speed details

- `bench.sql`: 117 lines, 3,545 bytes
- `bench_30x.sql`: 3510 lines, 106,350 bytes

### bench.sql (1×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.7 ± 0.1 | 1.6 | 2.1 | 1.15 ± 0.06 |
| `lemon-rs` | 1.5 ± 0.1 | 1.4 | 2.2 | 1.00 |
| `sql-parser-cst` | 75.3 ± 5.6 | 72.0 | 108.8 | 51.53 ± 4.39 |
| `sqlglot[c]` | 83.9 ± 1.0 | 82.6 | 87.3 | 57.47 ± 2.50 |
| `sqlparser-rs` | 1.8 ± 0.1 | 1.7 | 5.2 | 1.23 ± 0.10 |
| `node-sql-parser` | 74.0 ± 5.5 | 71.4 | 106.8 | 50.63 ± 4.34 |
| `sqlfluff` | 445.0 ± 2.7 | 439.5 | 447.5 | 304.66 ± 12.90 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.6 ± 0.6 | 2.4 | 20.5 | 1.00 |
| `lemon-rs` | 4.2 ± 1.0 | 4.0 | 29.7 | 1.64 ± 0.54 |
| `sql-parser-cst` | 140.1 ± 1.2 | 138.2 | 142.6 | 54.28 ± 12.40 |
| `sqlglot[c]` | 180.5 ± 1.5 | 178.5 | 183.8 | 69.96 ± 15.99 |
| `sqlparser-rs` | 11.1 ± 2.4 | 10.4 | 33.7 | 4.30 ± 1.36 |
| `node-sql-parser` | 149.2 ± 1.3 | 147.3 | 152.3 | 57.82 ± 13.21 |
| `sqlfluff` | 6695.8 ± 304.1 | 6388.0 | 7052.6 | 2594.75 ± 604.22 |

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

For each formatter: does the formatted output produce identical `EXPLAIN`
bytecode to the original? This verifies semantic preservation, not just
syntactic validity. "CORRUPT" means the bytecode differs or `EXPLAIN` fails.

| Test                                   | syntaqlite | prettier-cst | sql-formatter | sqlglot[c] |  sleek  |  sqruff |
| -------------------------------------- | :--------: | :----------: | :-----------: | :--------: | :-----: | :-----: |
| T01: Multi ON CONFLICT UPSERT + RETURN |  CORRUPT   |      OK      |      OK       |    FAIL    |   OK    |   OK    |
| T02: Recursive CTE + MATERIALIZED / NO |  CORRUPT   |   CORRUPT    |    CORRUPT    |  CORRUPT   | CORRUPT | CORRUPT |
| T03: CREATE TABLE STRICT + WITHOUT ROW |     OK     |      OK      |      OK       |    FAIL    |   OK    |   OK    |
| T04: UPDATE FROM + INDEXED BY          |  CORRUPT   |      OK      |      OK       |     OK     |   OK    |   OK    |
| T05: CREATE TRIGGER + RAISE + WHEN + F |  CORRUPT   |   CORRUPT    |    CORRUPT    |  CORRUPT   | CORRUPT | CORRUPT |
| T06: FILTER clause + IIF + NULLS LAST  |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T07: ATTACH DATABASE                   |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T08: INSERT OR REPLACE                 |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T09: CREATE VIRTUAL TABLE (FTS5)       |     OK     |   CORRUPT    |    CORRUPT    |     OK     | CORRUPT | CORRUPT |
| T10: PRAGMA                            |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T11: EXPLAIN QUERY PLAN                |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T12: ALTER TABLE DROP COLUMN           |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T13: ALTER TABLE RENAME COLUMN         |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T14: REINDEX                           |     OK     |      OK      |      OK       |  CORRUPT   |   OK    |   OK    |
| T15: Window frame RANGE BETWEEN        |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T16: CREATE INDEX with WHERE (partial  |  CORRUPT   |   CORRUPT    |    CORRUPT    |  CORRUPT   | CORRUPT | CORRUPT |
| T17: REPLACE statement                 |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T18: Nested window functions + EXCLUDE |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T19: GLOB and LIKE with ESCAPE         |     OK     |      OK      |      OK       |     OK     | CORRUPT |   OK    |
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
| T32: JSON -> and ->> operators         |  CORRUPT   |      OK      |      OK       |     OK     |   OK    | CORRUPT |
| T33: Numeric literals with underscores |     OK     |     FAIL     |     FAIL      |    FAIL    | CORRUPT |   OK    |
| T34: Multiple WINDOW definitions + nth |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T35: HAVING without GROUP BY (3.39+)   |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T36: IS NOT DISTINCT FROM in complex e |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T37: Blob literals + CAST chains       |     OK     |      OK      |      OK       |     OK     | CORRUPT |   OK    |
| T38: GENERATED ALWAYS AS (VIRTUAL vs S |  CORRUPT   |   CORRUPT    |    CORRUPT    |    FAIL    | CORRUPT | CORRUPT |
| T39: Deeply nested CTE + compound SELE |     OK     |      OK      |      OK       |     OK     |   OK    |  FAIL   |
| T40: Window GROUPS frame + EXCLUDE TIE |     OK     |      OK      |      OK       |    FAIL    |   OK    |   OK    |

### Scoreboard

| Tool          | Correct | Corrupt | Refused |
| ------------- | ------: | ------: | ------: |
| syntaqlite    |   33/40 |       7 |       - |
| prettier-cst  |   34/40 |       5 |       1 |
| sql-formatter |   34/40 |       5 |       1 |
| sqlglot[c]    |   30/40 |       5 |       5 |
| sleek         |   32/40 |       8 |       - |
| sqruff        |   29/40 |       6 |       5 |

### Corruption details

| Tool          | Test                                                               | Error                                                        |
| ------------- | ------------------------------------------------------------------ | ------------------------------------------------------------ |
| syntaqlite    | T01: Multi ON CONFLICT UPSERT + RETURNING                          | EXPLAIN bytecode differs from original                       |
| syntaqlite    | T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZED               | EXPLAIN bytecode differs from original                       |
| prettier-cst  | T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZED               | EXPLAIN bytecode differs from original                       |
| sql-formatter | T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZED               | EXPLAIN bytecode differs from original                       |
| sqlglot[c]    | T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZED               | EXPLAIN failed on formatted SQL                              |
| sleek         | T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZED               | EXPLAIN bytecode differs from original                       |
| sqruff        | T02: Recursive CTE + MATERIALIZED / NOT MATERIALIZED               | EXPLAIN bytecode differs from original                       |
| syntaqlite    | T04: UPDATE FROM + INDEXED BY                                      | EXPLAIN bytecode differs from original                       |
| syntaqlite    | T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW                  | EXPLAIN bytecode differs from original                       |
| prettier-cst  | T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW                  | EXPLAIN bytecode differs from original                       |
| sql-formatter | T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW                  | EXPLAIN bytecode differs from original                       |
| sqlglot[c]    | T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW                  | EXPLAIN failed on formatted SQL                              |
| sleek         | T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW                  | EXPLAIN bytecode differs from original                       |
| sqruff        | T05: CREATE TRIGGER + RAISE + WHEN + FOR EACH ROW                  | EXPLAIN bytecode differs from original                       |
| prettier-cst  | T09: CREATE VIRTUAL TABLE (FTS5)                                   | EXPLAIN bytecode differs from original                       |
| sql-formatter | T09: CREATE VIRTUAL TABLE (FTS5)                                   | EXPLAIN bytecode differs from original                       |
| sleek         | T09: CREATE VIRTUAL TABLE (FTS5)                                   | EXPLAIN bytecode differs from original                       |
| sqruff        | T09: CREATE VIRTUAL TABLE (FTS5)                                   | EXPLAIN bytecode differs from original                       |
| sqlglot[c]    | T14: REINDEX                                                       | Error: in prepare, near "AS": syntax error
  REINDEX AS idx_ |
| syntaqlite    | T16: CREATE INDEX with WHERE (partial index)                       | EXPLAIN bytecode differs from original                       |
| prettier-cst  | T16: CREATE INDEX with WHERE (partial index)                       | EXPLAIN bytecode differs from original                       |
| sql-formatter | T16: CREATE INDEX with WHERE (partial index)                       | EXPLAIN bytecode differs from original                       |
| sqlglot[c]    | T16: CREATE INDEX with WHERE (partial index)                       | EXPLAIN bytecode differs from original                       |
| sleek         | T16: CREATE INDEX with WHERE (partial index)                       | EXPLAIN bytecode differs from original                       |
| sqruff        | T16: CREATE INDEX with WHERE (partial index)                       | EXPLAIN bytecode differs from original                       |
| sleek         | T19: GLOB and LIKE with ESCAPE                                     | EXPLAIN bytecode differs from original                       |
| sqlglot[c]    | T23: SAVEPOINT / RELEASE / ROLLBACK TO                             | Error: in prepare, near "AS": syntax error
  SAVEPOINT AS my |
| syntaqlite    | T32: JSON -> and ->> operators                                     | EXPLAIN bytecode differs from original                       |
| sqruff        | T32: JSON -> and ->> operators                                     | EXPLAIN failed on formatted SQL                              |
| sleek         | T33: Numeric literals with underscores                             | EXPLAIN failed on formatted SQL                              |
| sleek         | T37: Blob literals + CAST chains                                   | EXPLAIN failed on formatted SQL                              |
| syntaqlite    | T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + complex expressions | EXPLAIN bytecode differs from original                       |
| prettier-cst  | T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + complex expressions | EXPLAIN bytecode differs from original                       |
| sql-formatter | T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + complex expressions | EXPLAIN bytecode differs from original                       |
| sleek         | T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + complex expressions | EXPLAIN bytecode differs from original                       |
| sqruff        | T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + complex expressions | EXPLAIN failed on formatted SQL                              |

## Speed details

- `bench.sql`: 117 lines, 3,545 bytes
- `bench_30x.sql`: 3510 lines, 106,350 bytes

### bench.sql (1×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.8 ± 0.1 | 1.7 | 2.6 | 1.00 |
| `prettier-cst` | 399.8 ± 20.0 | 386.7 | 456.2 | 219.84 ± 13.96 |
| `sql-formatter` | 74.9 ± 1.1 | 72.3 | 77.9 | 41.19 ± 1.72 |
| `sqlglot[c]` | 87.7 ± 2.0 | 85.3 | 94.2 | 48.20 ± 2.18 |
| `sleek` | 8.7 ± 2.6 | 7.8 | 43.9 | 4.79 ± 1.44 |
| `sqruff` | 43.6 ± 16.6 | 38.8 | 141.6 | 23.99 ± 9.18 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 5.2 ± 0.7 | 4.8 | 15.7 | 1.00 |
| `prettier-cst` | 594.7 ± 74.5 | 541.4 | 721.4 | 115.37 ± 21.84 |
| `sql-formatter` | 210.6 ± 24.4 | 195.5 | 271.2 | 40.85 ± 7.48 |
| `sqlglot[c]` | 296.2 ± 51.9 | 264.2 | 414.7 | 57.47 ± 12.96 |
| `sleek` | 28.5 ± 3.6 | 26.3 | 60.3 | 5.53 ± 1.05 |
| `sqruff` | 4657.1 ± 815.3 | 3861.5 | 5994.5 | 903.47 ± 203.61 |


### Slow tools (single timed run)

| Tool          |  Time |
| ------------- | ----: |
| sqlfmt (1x)   | 206ms |
| sqlfmt (30x)  | 381ms |
| sqlfluff (1x) | 250ms |

---

# Validator details

## Diagnostic quality showcase

A realistic query with 2 subtle errors — how does each tool report them?

**Query** (CTE declares 3 columns but SELECT produces 2; typo `ROUDN`):

```sql
WITH
  monthly_stats(month, revenue, order_count) AS (
    SELECT
      STRFTIME('%Y-%m', o.created_at) AS month,
      SUM(o.total) AS revenue
    FROM orders o
    WHERE o.status = 'completed'
    GROUP BY STRFTIME('%Y-%m', o.created_at)
  )
SELECT
  ms.month,
  ms.revenue,
  ms.order_count,
  ROUDN(ms.revenue / ms.order_count, 2) AS avg_order
FROM monthly_stats ms
ORDER BY ms.month DESC
LIMIT 12;
```

### syntaqlite

Static semantic analysis — offline, no database needed. Finds **both** errors in one pass:

```
error: table 'monthly_stats' has 2 values for 3 columns
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpxhi61dqq.sql:29:3
   |
29 |   monthly_stats(month, revenue, order_count) AS (
   |   ^~~~~~~~~~~~~
warning: unknown function 'ROUDN'
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpxhi61dqq.sql:41:3
   |
41 |   ROUDN(ms.revenue / ms.order_count, 2) AS avg_order
   |   ^~~~~
   = help: did you mean 'round'?
```

### sqlite3

Runtime execution — stops at first error:

```
Error: in prepare, table monthly_stats has 2 values for 3 columns
```

### sqlite-runner-lsp

Runtime via LSP — wraps sqlite3, same single error:

```
(no diagnostics)
```

### sql-lint

Structural checks only:

```
/var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmp11hfv30k.sql:1 sql-lint was unable to lint the following query "WITH...
```


## Per-case error detection

Schema: `users`, `orders`, `products`, `order_items`. Ground truth: sqlite3.

| Test                      | Expect | syntaqlite | sqlite3 | sqlite-runner-lsp | sql-lint |
| ------------------------- | :----: | :--------: | :-----: | :---------------: | :------: |
| keyword typo (SELEC)      | error  |   FOUND    |  FOUND  |       MISS        |  FOUND   |
| missing close paren       | error  |   FOUND    |  FOUND  |       MISS        |  FOUND   |
| double comma              | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| unterminated string       | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| trailing comma in VALUES  | error  |   FOUND    |  FOUND  |       MISS        |  FOUND   |
| unknown table             | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| unknown table in JOIN     | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| unknown column            | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| unknown qualified column  | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| unknown column in SELECT  | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| SUBSTR: too few args      | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| REPLACE: too few args     | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| LENGTH: too many args     | error  |   FOUND    |  FOUND  |       MISS        |   MISS   |
| COALESCE: zero args       | error  |    MISS    |  FOUND  |       MISS        |   MISS   |
| CTE: 3 declared, 2 actual | error  |   FOUND    |  FOUND  |       MISS        |  FOUND   |
| valid: simple SELECT      | valid  |     OK     |   OK    |        OK         |    OK    |
| valid: JOIN + aggregate   | valid  |     OK     |   OK    |        OK         |    OK    |
| valid: SUBSTR with 3 args | valid  |     OK     |   OK    |        OK         |    OK    |
| valid: COALESCE variadic  | valid  |     OK     |   OK    |        OK         |    OK    |
| valid: CTE columns match  | valid  |     OK     |   OK    |        OK         |    FP    |
| valid: built-in functions | valid  |     OK     |   OK    |        OK         |    OK    |
| valid: INSERT             | valid  |     OK     |   OK    |        OK         |    OK    |
| valid: UPDATE             | valid  |     OK     |   OK    |        OK         |    OK    |
| valid: DELETE with WHERE  | valid  |     OK     |   OK    |        OK         |    OK    |

### Scoreboard

| Tool              | Approach          | Correct                    | Missed | FP |
| ----------------- | ----------------- | -------------------------- | -----: | -: |
| sqlite3           | runtime execution | 24/24 ████████████████████ |      - |  - |
| syntaqlite        | static semantic   | 23/24 ███████████████████  |      1 |  - |
| sql-lint          | structural checks | 12/24 ██████████           |     11 |  1 |
| sqlite-runner-lsp | runtime via LSP   | 9/24 ███████               |     15 |  - |

## Speed details

- `bench.sql`: 117 lines, 3,545 bytes (+ schema preamble)
- `bench_30x.sql`: 3510 lines, 106,350 bytes (+ schema preamble)

### bench.sql (1×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.1 ± 0.9 | 1.9 | 21.4 | 1.00 |
| `sqlite3` | 4.9 ± 0.2 | 4.6 | 6.2 | 2.30 ± 0.93 |
| `sqlite-runner-lsp` | 10050.2 ± 8.6 | 10041.9 | 10066.9 | 4708.90 ± 1896.03 |
| `sql-lint` | 335.4 ± 7.4 | 327.3 | 348.1 | 157.15 ± 63.37 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 8.6 ± 0.5 | 8.4 | 16.7 | 1.00 |
| `sqlite3` | 10.2 ± 2.0 | 9.6 | 37.0 | 1.18 ± 0.24 |
| `sqlite-runner-lsp` | 10071.4 ± 7.4 | 10065.4 | 10083.9 | 1165.82 ± 64.19 |
| `sql-lint` | 365.5 ± 1.6 | 362.4 | 367.5 | 42.31 ± 2.34 |

---

# LSP details

## Tested capabilities

Each server is started, sent a test file, and probed for completion, hover,
diagnostics, and formatting via the LSP protocol.

| Feature                |    syntaqlite   |      sqls     | sql-language-server |
| ---------------------- | :-------------: | :-----------: | :-----------------: |
| Completion             | Yes (129 items) | Yes (6 items) |   Yes (11 items)    |
| Hover                  |       No        |      Yes      |         No          |
| Go to definition       |       No        |      Yes      |         No          |
| Find references        |       No        |      No       |         No          |
| Diagnostics: syntax    |       Yes       |      No       |         Yes         |
| Diagnostics: semantic  |       Yes       |      No       |   No (style only)   |
| Formatting             |       Yes       |      Yes      |         No          |
| Rename                 |       No        |      Yes      |         Yes         |
| Signature help         |       Yes       |      Yes      |         No          |
| Requires DB connection |       No        |      Yes      |         No          |

## Diagnostic detail

What each server reports for `SELEC * FROM users;` (syntax error):

### syntaqlite

```
1:1 error syntax error near 'SELEC'
```

### sqls

```
(no diagnostics)
```

### sql-language-server

```
1:2 error Expected "$", "(", "--", "/*", "ALTER", "CREATE TABLE", "CREATE", "DELETE", "DROP TABLE", "DROP VIEW", "DROP", "INSERT", "REPLACE", "SELECT", "UPDATE", "WITH", "return", [ \t\n\r], or end of input but "S" found.
```


## Speed details

Time to start server, send document, receive diagnostics, and exit:

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 33.0 ± 0.9 | 30.4 | 35.0 | 1.00 |
| `sqls` | 10065.0 ± 7.2 | 10052.6 | 10070.3 | 304.91 ± 8.47 |
| `sql-language-server` | 469.7 ± 10.6 | 456.3 | 482.1 | 14.23 ± 0.51 |
