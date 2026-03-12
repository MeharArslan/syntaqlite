+++
title = "Comparison Details"
weight = 11
+++
# syntaqlite — Competitive Comparison

SQLite SQL tooling landscape.


# Parser Comparison

Per-statement SQLite SQL parsing accuracy, validated against sqlite3 as ground truth.

## Ground Truth

Validating all test statements against sqlite3:

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

## Parser Accuracy

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

## Parse Speed

- `bench.sql`: 117 lines, 3,545 bytes
- `bench_30x.sql`: 3510 lines, 106,350 bytes

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.8 ± 0.4 | 1.6 | 14.2 | 1.18 ± 0.30 |
| `lemon-rs` | 1.5 ± 0.2 | 1.4 | 3.3 | 1.00 |
| `sql-parser-cst` | 76.8 ± 1.1 | 74.7 | 79.5 | 50.48 ± 5.22 |
| `sqlglot[c]` | 85.8 ± 1.4 | 83.7 | 89.6 | 56.43 ± 5.85 |
| `sqlparser-rs` | 1.8 ± 0.1 | 1.7 | 3.1 | 1.20 ± 0.14 |
| `node-sql-parser` | 74.7 ± 1.2 | 73.3 | 78.5 | 49.10 ± 5.08 |
| `sqlfluff` | 458.5 ± 5.9 | 446.1 | 469.2 | 301.39 ± 31.08 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.6 ± 0.1 | 2.4 | 3.8 | 1.00 |
| `lemon-rs` | 4.2 ± 0.1 | 4.1 | 5.0 | 1.64 ± 0.08 |
| `sql-parser-cst` | 143.7 ± 2.5 | 138.8 | 149.4 | 56.13 ± 2.57 |
| `sqlglot[c]` | 186.3 ± 4.4 | 181.4 | 196.6 | 72.81 ± 3.54 |
| `sqlparser-rs` | 13.1 ± 2.8 | 10.8 | 29.2 | 5.12 ± 1.10 |
| `node-sql-parser` | 313.3 ± 59.3 | 217.5 | 421.5 | 122.44 ± 23.73 |
| `sqlfluff` | 390.3 ± 115.8 | 287.1 | 569.5 | 152.50 ± 45.72 |


# Formatter Comparison

Round-trip correctness (format then validate with sqlite3) and speed.

## Ground Truth

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

## Round-Trip Validation

For each formatter: does the formatted output still pass real SQLite?

| Test                                   | syntaqlite | prettier-cst | sql-formatter | sqlglot[c] |  sleek  |  sqruff |
| -------------------------------------- | :--------: | :----------: | :-----------: | :--------: | :-----: | :-----: |
| T01: Multi ON CONFLICT UPSERT + RETURN |     OK     |      OK      |      OK       |    FAIL    |   OK    |   OK    |
| T02: Recursive CTE + MATERIALIZED / NO |     OK     |      OK      |      OK       |  CORRUPT   |   OK    |   OK    |
| T03: CREATE TABLE STRICT + WITHOUT ROW |     OK     |      OK      |      OK       |    FAIL    |   OK    |   OK    |
| T04: UPDATE FROM + INDEXED BY          |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T05: CREATE TRIGGER + RAISE + WHEN + F |     OK     |      OK      |      OK       |  CORRUPT   |   OK    |   OK    |
| T06: FILTER clause + IIF + NULLS LAST  |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T07: ATTACH DATABASE                   |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T08: INSERT OR REPLACE                 |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T09: CREATE VIRTUAL TABLE (FTS5)       |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T10: PRAGMA                            |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T11: EXPLAIN QUERY PLAN                |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T12: ALTER TABLE DROP COLUMN           |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T13: ALTER TABLE RENAME COLUMN         |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T14: REINDEX                           |     OK     |      OK      |      OK       |  CORRUPT   |   OK    |   OK    |
| T15: Window frame RANGE BETWEEN        |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T16: CREATE INDEX with WHERE (partial  |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T17: REPLACE statement                 |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T18: Nested window functions + EXCLUDE |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
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
| T34: Multiple WINDOW definitions + nth |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T35: HAVING without GROUP BY (3.39+)   |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T36: IS NOT DISTINCT FROM in complex e |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T37: Blob literals + CAST chains       |     OK     |      OK      |      OK       |     OK     | CORRUPT |   OK    |
| T38: GENERATED ALWAYS AS (VIRTUAL vs S |     OK     |      OK      |      OK       |    FAIL    |   OK    | CORRUPT |
| T39: Deeply nested CTE + compound SELE |     OK     |      OK      |      OK       |     OK     |   OK    |   OK    |
| T40: Window GROUPS frame + EXCLUDE TIE |     OK     |      OK      |      OK       |    FAIL    |   OK    |   OK    |

### Scoreboard

| Tool          | Formats | SQLite OK | Corrupt |
| ------------- | ------: | --------: | ------: |
| syntaqlite    |   40/40 |     40/40 |       0 |
| prettier-cst  |   39/40 |     39/40 |       0 |
| sql-formatter |   39/40 |     39/40 |       0 |
| sqlglot[c]    |   35/40 |     31/40 |       4 |
| sleek         |   40/40 |     38/40 |       2 |
| sqruff        |   40/40 |     38/40 |       2 |

### Corruption Details

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

## Format Speed

- `bench.sql`: 117 lines, 3,545 bytes
- `bench_30x.sql`: 3510 lines, 106,350 bytes

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.9 ± 0.2 | 1.7 | 4.3 | 1.00 |
| `prettier-cst` | 408.8 ± 5.5 | 400.3 | 417.2 | 210.36 ± 20.36 |
| `sql-formatter` | 78.0 ± 1.3 | 75.6 | 81.5 | 40.15 ± 3.91 |
| `sqlglot[c]` | 91.5 ± 2.6 | 88.1 | 100.3 | 47.08 ± 4.70 |
| `sleek` | 8.8 ± 0.7 | 7.8 | 13.1 | 4.54 ± 0.56 |
| `sqruff` | 40.9 ± 1.1 | 39.5 | 44.8 | 21.05 ± 2.09 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 5.4 ± 1.4 | 4.7 | 18.7 | 1.00 |
| `prettier-cst` | 559.5 ± 9.2 | 551.6 | 574.9 | 104.40 ± 27.36 |
| `sql-formatter` | 201.9 ± 1.1 | 199.2 | 203.9 | 37.68 ± 9.86 |
| `sqlglot[c]` | 272.2 ± 5.8 | 265.9 | 283.5 | 50.79 ± 13.33 |
| `sleek` | 27.9 ± 0.9 | 26.6 | 34.4 | 5.21 ± 1.37 |
| `sqruff` | 3641.3 ± 147.1 | 3503.8 | 3880.5 | 679.47 ± 179.84 |


### Slow Tools (single timed run)

| Tool          |  Time |
| ------------- | ----: |
| sqlfmt (1x)   | 264ms |
| sqlfmt (30x)  | 307ms |
| sqlfluff (1x) | 233ms |


# Validator Comparison

Error detection accuracy and diagnostic quality.

## Diagnostic Quality

A realistic query with subtle errors — how does each tool report them?

**Query** (2 errors: CTE declares 3 columns but SELECT produces 2; typo `ROUDN`):

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
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpopk0z37n.sql:29:3
   |
29 |   monthly_stats(month, revenue, order_count) AS (
   |   ^~~~~~~~~~~~~
warning: unknown function 'ROUDN'
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpopk0z37n.sql:41:3
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
/var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmplm8laoca.sql:1 sql-lint was unable to lint the following query "WITH...
```

## Error Detection Accuracy

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

## Validation Speed

- `bench.sql`: 117 lines, 3,545 bytes (+ schema preamble)
- `bench_30x.sql`: 3510 lines, 106,350 bytes (+ schema preamble)

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.1 ± 0.2 | 1.9 | 3.7 | 1.00 |
| `sqlite3` | 5.3 ± 1.0 | 4.4 | 22.7 | 2.50 ± 0.55 |
| `sqlite-runner-lsp` | 10058.5 ± 21.5 | 10042.0 | 10114.0 | 4749.93 ± 454.68 |
| `sql-lint` | 361.4 ± 11.7 | 350.5 | 381.3 | 170.66 ± 17.25 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 7.5 ± 1.2 | 7.0 | 29.6 | 1.00 |
| `sqlite3` | 10.2 ± 0.5 | 9.4 | 13.0 | 1.37 ± 0.22 |
| `sqlite-runner-lsp` | 10060.1 ± 4.3 | 10055.7 | 10066.0 | 1344.95 ± 208.44 |
| `sql-lint` | 383.3 ± 9.1 | 366.5 | 392.8 | 51.24 ± 8.03 |


# LSP Comparison

Feature testing for SQLite-aware language servers.

## Tested Capabilities

Each server is started, sent a test file, and probed for completion, hover,
diagnostics, and formatting. Results are from actual LSP responses.

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
| Signature help         |       No        |      Yes      |         No          |
| Requires DB connection |       No        |      Yes      |         No          |

## Diagnostic Detail

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

## LSP Startup + Response Speed

Time to start server, send document, receive diagnostics, and exit:

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 35.2 ± 1.8 | 32.1 | 40.8 | 1.00 |
| `sqls` | 10053.2 ± 10.9 | 10042.8 | 10071.4 | 285.55 ± 14.66 |
| `sql-language-server` | 489.7 ± 9.9 | 479.5 | 506.1 | 13.91 ± 0.77 |

