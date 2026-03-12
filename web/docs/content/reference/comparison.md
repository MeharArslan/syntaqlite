+++
title = "Competitive Comparison"
weight = 10
+++
# syntaqlite — Competitive Comparison

SQLite SQL tooling landscape. See [detailed results](@/reference/comparison-details.md) for per-statement breakdowns, corruption details, and diagnostic examples.


# Parser

40 test statements covering obscure SQLite syntax, validated against sqlite3 as ground truth.

## Accuracy

| Tool            | Correct                           | Rejects Valid | Accepts Invalid |
| --------------- | --------------------------------- | ------------: | --------------: |
| syntaqlite      | 40/40 (100%) ████████████████████ |             - |               - |
| lemon-rs        | 40/40 (100%) ████████████████████ |             - |               - |
| sql-parser-cst  | 39/40 (97%) ███████████████████   |             1 |               - |
| sqlglot[c]      | 35/40 (87%) █████████████████     |             5 |               - |
| sqlfluff        | 29/40 (72%) ██████████████        |            11 |               - |
| sqlparser-rs    | 26/40 (65%) █████████████         |            14 |               - |
| node-sql-parser | 15/40 (37%) ███████               |            25 |               - |

## Speed

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.7 ± 0.1 | 1.6 | 2.6 | 1.16 ± 0.11 |
| `lemon-rs` | 1.5 ± 0.1 | 1.4 | 3.4 | 1.00 |
| `sql-parser-cst` | 76.8 ± 2.2 | 72.7 | 83.1 | 51.08 ± 4.32 |
| `sqlglot[c]` | 86.1 ± 3.1 | 82.9 | 95.5 | 57.22 ± 5.00 |
| `sqlparser-rs` | 1.9 ± 0.2 | 1.7 | 4.9 | 1.27 ± 0.17 |
| `node-sql-parser` | 75.9 ± 3.6 | 72.5 | 91.0 | 50.44 ± 4.68 |
| `sqlfluff` | 465.7 ± 10.7 | 451.7 | 477.5 | 309.62 ± 25.58 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.8 ± 0.2 | 2.5 | 5.1 | 1.00 |
| `lemon-rs` | 8.6 ± 4.4 | 4.2 | 48.9 | 3.12 ± 1.63 |
| `sql-parser-cst` | 553.8 ± 132.6 | 424.4 | 720.2 | 200.59 ± 51.17 |
| `sqlglot[c]` | 433.9 ± 158.0 | 295.6 | 745.2 | 157.17 ± 58.89 |
| `sqlparser-rs` | 15.5 ± 3.3 | 12.3 | 34.1 | 5.61 ± 1.29 |
| `node-sql-parser` | 166.2 ± 23.6 | 151.2 | 236.0 | 60.22 ± 10.06 |
| `sqlfluff` | 271.3 ± 6.1 | 261.8 | 279.7 | 98.29 ± 8.94 |


# Formatter

Round-trip correctness (format then validate with sqlite3) and speed.

## Accuracy

| Tool          | Formats | SQLite OK | Corrupt |
| ------------- | ------: | --------: | ------: |
| syntaqlite    |   40/40 |     40/40 |       0 |
| prettier-cst  |   39/40 |     39/40 |       0 |
| sql-formatter |   39/40 |     39/40 |       0 |
| sqlglot[c]    |   35/40 |     31/40 |       4 |
| sleek         |   40/40 |     38/40 |       2 |
| sqruff        |   40/40 |     38/40 |       2 |

## Speed

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.9 ± 0.2 | 1.7 | 3.1 | 1.00 |
| `prettier-cst` | 500.7 ± 97.1 | 398.6 | 725.9 | 257.45 ± 54.30 |
| `sql-formatter` | 143.1 ± 64.0 | 86.0 | 341.3 | 73.59 ± 33.49 |
| `sqlglot[c]` | 101.7 ± 13.0 | 89.3 | 151.5 | 52.31 ± 7.97 |
| `sleek` | 9.2 ± 3.6 | 7.7 | 44.0 | 4.71 ± 1.90 |
| `sqruff` | 41.4 ± 3.8 | 38.7 | 62.9 | 21.29 ± 2.63 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 4.9 ± 0.2 | 4.7 | 5.8 | 1.00 |
| `prettier-cst` | 561.5 ± 7.9 | 552.1 | 572.2 | 115.00 ± 4.00 |
| `sql-formatter` | 213.9 ± 25.2 | 201.7 | 297.4 | 43.81 ± 5.34 |
| `sqlglot[c]` | 273.2 ± 11.1 | 262.9 | 299.9 | 55.95 ± 2.88 |
| `sleek` | 27.2 ± 0.5 | 26.2 | 28.6 | 5.58 ± 0.20 |
| `sqruff` | 3585.0 ± 455.3 | 3240.4 | 4385.6 | 734.25 ± 96.14 |


# Validator

Error detection accuracy and diagnostic quality.

## Accuracy

Schema: `users`, `orders`, `products`, `order_items`. Ground truth: sqlite3.

| Tool              | Approach          | Correct                    | Missed | FP |
| ----------------- | ----------------- | -------------------------- | -----: | -: |
| sqlite3           | runtime execution | 24/24 ████████████████████ |      - |  - |
| syntaqlite        | static semantic   | 23/24 ███████████████████  |      1 |  - |
| sql-lint          | structural checks | 12/24 ██████████           |     11 |  1 |
| sqlite-runner-lsp | runtime via LSP   | 9/24 ███████               |     15 |  - |

## Diagnostic Quality

Query with 2 errors: CTE declares 3 columns but SELECT produces 2, and typo `ROUDN` instead of `ROUND`.

| Tool              | Approach          | Errors Found | Finds All | Did-you-mean |
| ----------------- | ----------------- | :----------: | :-------: | :----------: |
| syntaqlite        | static semantic   |     2/2      |    Yes    |     Yes      |
| sqlite3           | runtime execution |     1/2      |    No     |      No      |
| sqlite-runner-lsp | runtime via LSP   |     0/2      |    No     |      No      |
| sql-lint          | structural checks |     0/2      |    No     |      No      |

**syntaqlite**:

```
error: table 'monthly_stats' has 2 values for 3 columns
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmp1e4h0drf.sql:29:3
   |
29 |   monthly_stats(month, revenue, order_count) AS (
   |   ^~~~~~~~~~~~~
warning: unknown function 'ROUDN'
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmp1e4h0drf.sql:41:3
   |
41 |   ROUDN(ms.revenue / ms.order_count, 2) AS avg_order
   |   ^~~~~
   = help: did you mean 'round'?
```

**sqlite3**:

```
Error: in prepare, table monthly_stats has 2 values for 3 columns
```

## Speed

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.1 ± 0.3 | 1.9 | 5.4 | 1.00 |
| `sqlite3` | 8.5 ± 6.5 | 4.6 | 55.0 | 4.03 ± 3.15 |
| `sqlite-runner-lsp` | 10063.6 ± 23.7 | 10042.4 | 10123.4 | 4771.48 ± 622.13 |
| `sql-lint` | 368.9 ± 11.5 | 355.5 | 388.3 | 174.90 ± 23.44 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 7.5 ± 0.3 | 7.1 | 9.6 | 1.00 |
| `sqlite3` | 10.3 ± 0.6 | 9.6 | 12.8 | 1.37 ± 0.09 |
| `sqlite-runner-lsp` | 10055.1 ± 9.6 | 10041.9 | 10068.7 | 1340.11 ± 51.33 |
| `sql-lint` | 392.6 ± 10.7 | 382.0 | 413.5 | 52.33 ± 2.46 |


# LSP

Feature testing for SQLite-aware language servers.

## Features

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

## Startup + Response Speed

Time to start server, send document, receive diagnostics, and exit:

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 47.7 ± 12.3 | 32.5 | 81.0 | 1.00 |
| `sqls` | 10064.4 ± 19.4 | 10044.2 | 10090.9 | 211.19 ± 54.44 |
| `sql-language-server` | 740.8 ± 239.4 | 504.6 | 1044.1 | 15.54 ± 6.43 |

