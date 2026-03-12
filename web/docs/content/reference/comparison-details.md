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
| T01: Multi ON CONFLICT UPSERT + RETURN |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T02: Recursive CTE + MATERIALIZED / NO |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T03: CREATE TABLE STRICT + WITHOUT ROW |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T04: UPDATE FROM + INDEXED BY          |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T05: CREATE TRIGGER + RAISE + WHEN + F |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T06: FILTER clause + IIF + NULLS LAST  |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T07: ATTACH DATABASE                   |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T08: INSERT OR REPLACE                 |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T09: CREATE VIRTUAL TABLE (FTS5)       |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T10: PRAGMA                            |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T11: EXPLAIN QUERY PLAN                |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T12: ALTER TABLE DROP COLUMN           |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T13: ALTER TABLE RENAME COLUMN         |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T14: REINDEX                           |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T15: Window frame RANGE BETWEEN        |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T16: CREATE INDEX with WHERE (partial  |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T17: REPLACE statement                 |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T18: Nested window functions + EXCLUDE |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T19: GLOB and LIKE with ESCAPE         |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T20: INSERT with multiple VALUES + ON  |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T21: Complex subquery expressions      |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T22: ANALYZE                           |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T23: SAVEPOINT / RELEASE / ROLLBACK TO |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T24: DROP TABLE IF EXISTS              |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T25: CREATE TABLE AS SELECT            |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T26: DETACH DATABASE                   |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T27: UPSERT with complex expressions i |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T28: WITH (non-recursive) + DELETE ... |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T29: UPDATE ... RETURNING              |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T30: RIGHT JOIN + IS DISTINCT FROM     |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T31: FULL OUTER JOIN                   |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T32: JSON -> and ->> operators         |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T33: Numeric literals with underscores |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T34: Multiple WINDOW definitions + nth |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T35: HAVING without GROUP BY (3.39+)   |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T36: IS NOT DISTINCT FROM in complex e |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T37: Blob literals + CAST chains       |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T38: GENERATED ALWAYS AS (VIRTUAL vs S |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T39: Deeply nested CTE + compound SELE |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |
| T40: Window GROUPS frame + EXCLUDE TIE |   OK    |    PASS    |   FAIL   |      FAIL      |    FAIL    |   PASS   |     FAIL     |      FAIL       |

### Scoreboard

| Tool            | Correct                           | Rejects Valid | Accepts Invalid |
| --------------- | --------------------------------- | ------------: | --------------: |
| syntaqlite      | 40/40 (100%) ████████████████████ |             - |               - |
| sqlfluff        | 40/40 (100%) ████████████████████ |             - |               - |
| lemon-rs        | 0/40 (0%)                         |            40 |               - |
| sql-parser-cst  | 0/40 (0%)                         |            40 |               - |
| sqlglot[c]      | 0/40 (0%)                         |            40 |               - |
| sqlparser-rs    | 0/40 (0%)                         |            40 |               - |
| node-sql-parser | 0/40 (0%)                         |            40 |               - |

## Parse Speed

- `bench.sql`: 117 lines, 3,545 bytes
- `bench_30x.sql`: 3510 lines, 106,350 bytes

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.8 ± 0.7 | 1.6 | 24.4 | 1.00 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.6 ± 0.3 | 2.4 | 7.3 | 1.00 |



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
| T01: Multi ON CONFLICT UPSERT + RETURN |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T02: Recursive CTE + MATERIALIZED / NO |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T03: CREATE TABLE STRICT + WITHOUT ROW |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T04: UPDATE FROM + INDEXED BY          |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T05: CREATE TRIGGER + RAISE + WHEN + F |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T06: FILTER clause + IIF + NULLS LAST  |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T07: ATTACH DATABASE                   |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T08: INSERT OR REPLACE                 |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T09: CREATE VIRTUAL TABLE (FTS5)       |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T10: PRAGMA                            |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T11: EXPLAIN QUERY PLAN                |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T12: ALTER TABLE DROP COLUMN           |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T13: ALTER TABLE RENAME COLUMN         |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T14: REINDEX                           |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T15: Window frame RANGE BETWEEN        |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T16: CREATE INDEX with WHERE (partial  |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T17: REPLACE statement                 |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T18: Nested window functions + EXCLUDE |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T19: GLOB and LIKE with ESCAPE         |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T20: INSERT with multiple VALUES + ON  |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T21: Complex subquery expressions      |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T22: ANALYZE                           |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T23: SAVEPOINT / RELEASE / ROLLBACK TO |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T24: DROP TABLE IF EXISTS              |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T25: CREATE TABLE AS SELECT            |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T26: DETACH DATABASE                   |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T27: UPSERT with complex expressions i |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T28: WITH (non-recursive) + DELETE ... |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T29: UPDATE ... RETURNING              |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T30: RIGHT JOIN + IS DISTINCT FROM     |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T31: FULL OUTER JOIN                   |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T32: JSON -> and ->> operators         |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    | CORRUPT |
| T33: Numeric literals with underscores |     OK     |     FAIL     |     FAIL      |    FAIL    | CORRUPT |   OK    |
| T34: Multiple WINDOW definitions + nth |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T35: HAVING without GROUP BY (3.39+)   |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T36: IS NOT DISTINCT FROM in complex e |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T37: Blob literals + CAST chains       |     OK     |     FAIL     |     FAIL      |    FAIL    | CORRUPT |   OK    |
| T38: GENERATED ALWAYS AS (VIRTUAL vs S |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    | CORRUPT |
| T39: Deeply nested CTE + compound SELE |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |
| T40: Window GROUPS frame + EXCLUDE TIE |     OK     |     FAIL     |     FAIL      |    FAIL    |   OK    |   OK    |

### Scoreboard

| Tool          | Formats | SQLite OK | Corrupt |
| ------------- | ------: | --------: | ------: |
| syntaqlite    |   40/40 |     40/40 |       0 |
| prettier-cst  |    0/40 |      0/40 |       0 |
| sql-formatter |    0/40 |      0/40 |       0 |
| sqlglot[c]    |    0/40 |      0/40 |       0 |
| sleek         |   40/40 |     38/40 |       2 |
| sqruff        |   40/40 |     38/40 |       2 |

### Corruption Details

| Tool   | Test                                                               | Error                                                        |
| ------ | ------------------------------------------------------------------ | ------------------------------------------------------------ |
| sqruff | T32: JSON -> and ->> operators                                     | Error: in prepare, near ">": syntax error
  EXPLAIN SELECT   |
| sleek  | T33: Numeric literals with underscores                             | Error: in prepare, near "AS": syntax error
  EXPLAIN SELECT  |
| sleek  | T37: Blob literals + CAST chains                                   | Error: in prepare, near "AS": syntax error
  EXPLAIN SELECT  |
| sqruff | T38: GENERATED ALWAYS AS (VIRTUAL vs STORED) + complex expressions | Error: in prepare, near "|": syntax error
  abel TEXT GENERA |

## Format Speed

- `bench.sql`: 117 lines, 3,545 bytes
- `bench_30x.sql`: 3510 lines, 106,350 bytes

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.8 ± 0.1 | 1.7 | 3.2 | 1.00 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 4.9 ± 0.6 | 4.7 | 14.4 | 1.00 |


### Slow Tools (single timed run)

| Tool          | Time |
| ------------- | ---: |
| sqlfmt (1x)   | 10ms |
| sqlfmt (30x)  |  8ms |
| sqlfluff (1x) |  7ms |


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
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmp761p3gzh.sql:29:3
   |
29 |   monthly_stats(month, revenue, order_count) AS (
   |   ^~~~~~~~~~~~~
warning: unknown function 'ROUDN'
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmp761p3gzh.sql:41:3
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
/var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmp558yozg0.sql:1 sql-lint was unable to lint the following query "WITH...
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
| `syntaqlite` | 1.9 ± 0.2 | 1.8 | 4.3 | 1.00 |
| `sqlite3` | 5.3 ± 1.9 | 4.2 | 27.1 | 2.73 ± 1.04 |
| `sqlite-runner-lsp` | 10053.7 ± 11.6 | 10041.2 | 10072.9 | 5175.61 ± 551.14 |
| `sql-lint` | 489.6 ± 25.1 | 459.2 | 534.1 | 252.06 ± 29.80 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 6.8 ± 0.2 | 6.5 | 9.1 | 1.00 |
| `sqlite3` | 9.5 ± 0.5 | 8.9 | 13.6 | 1.40 ± 0.08 |
| `sqlite-runner-lsp` | 10072.3 ± 3.0 | 10069.9 | 10077.4 | 1487.39 ± 46.78 |
| `sql-lint` | 752.2 ± 152.0 | 569.1 | 898.5 | 111.08 ± 22.71 |



# LSP Comparison

Feature testing for SQLite-aware language servers.

## Tested Capabilities

Each server is started, sent a test file, and probed for completion, hover,
diagnostics, and formatting. Results are from actual LSP responses.

| Feature                |    syntaqlite   |      sqls     | sql-language-server  |
| ---------------------- | :-------------: | :-----------: | :------------------: |
| Completion             | Yes (150 items) | Yes (1 items) | Advertised (0 items) |
| Hover                  |       No        |      Yes      |          No          |
| Go to definition       |       No        |      Yes      |          No          |
| Find references        |       No        |      No       |          No          |
| Diagnostics: syntax    |       Yes       |      No       |         Yes          |
| Diagnostics: semantic  |       Yes       |      No       |   No (style only)    |
| Formatting             |       Yes       |      Yes      |          No          |
| Rename                 |       No        |      Yes      |         Yes          |
| Signature help         |       No        |      Yes      |          No          |
| Requires DB connection |       No        |      Yes      |          No          |

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
| `syntaqlite` | 39.2 ± 5.8 | 34.4 | 63.9 | 1.00 |
| `sqls` | 10059.2 ± 7.9 | 10053.2 | 10071.7 | 256.41 ± 37.86 |
| `sql-language-server` | 659.1 ± 25.3 | 640.3 | 702.7 | 16.80 ± 2.56 |


