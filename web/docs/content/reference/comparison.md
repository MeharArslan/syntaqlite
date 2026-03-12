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
| `syntaqlite` | 1.7 ± 0.1 | 1.6 | 4.0 | 1.16 ± 0.09 |
| `lemon-rs` | 1.5 ± 0.1 | 1.4 | 2.4 | 1.00 |
| `sql-parser-cst` | 75.7 ± 2.8 | 72.9 | 91.1 | 51.58 ± 2.96 |
| `sqlglot[c]` | 86.3 ± 8.2 | 81.8 | 124.0 | 58.80 ± 6.16 |
| `sqlparser-rs` | 1.8 ± 0.2 | 1.7 | 4.9 | 1.25 ± 0.13 |
| `node-sql-parser` | 77.5 ± 12.4 | 71.7 | 144.3 | 52.84 ± 8.80 |
| `sqlfluff` | 474.3 ± 19.0 | 448.2 | 511.8 | 323.24 ± 19.32 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.7 ± 0.5 | 2.4 | 12.0 | 1.00 |
| `lemon-rs` | 4.2 ± 0.1 | 4.0 | 5.0 | 1.58 ± 0.32 |
| `sql-parser-cst` | 142.8 ± 4.7 | 139.8 | 162.0 | 53.69 ± 10.79 |
| `sqlglot[c]` | 181.8 ± 1.7 | 179.6 | 184.5 | 68.33 ± 13.56 |
| `sqlparser-rs` | 11.0 ± 0.4 | 10.4 | 13.5 | 4.15 ± 0.84 |
| `node-sql-parser` | 153.9 ± 2.4 | 150.9 | 159.8 | 57.84 ± 11.50 |
| `sqlfluff` | 260.3 ± 13.2 | 253.2 | 298.6 | 97.86 ± 20.02 |


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
| `syntaqlite` | 1.9 ± 0.1 | 1.7 | 3.0 | 1.00 |
| `prettier-cst` | 423.2 ± 23.0 | 395.2 | 472.0 | 226.26 ± 19.82 |
| `sql-formatter` | 78.2 ± 4.7 | 74.9 | 95.8 | 41.78 ± 3.82 |
| `sqlglot[c]` | 89.1 ± 1.4 | 86.3 | 92.0 | 47.63 ± 3.36 |
| `sleek` | 8.7 ± 0.7 | 7.7 | 13.1 | 4.64 ± 0.51 |
| `sqruff` | 40.2 ± 0.9 | 38.9 | 42.9 | 21.48 ± 1.55 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 5.0 ± 0.2 | 4.7 | 6.4 | 1.00 |
| `prettier-cst` | 585.4 ± 22.8 | 555.5 | 614.2 | 118.21 ± 6.63 |
| `sql-formatter` | 202.4 ± 6.9 | 195.1 | 216.1 | 40.87 ± 2.16 |
| `sqlglot[c]` | 262.7 ± 1.6 | 260.2 | 265.8 | 53.04 ± 2.16 |
| `sleek` | 27.0 ± 0.6 | 25.9 | 29.6 | 5.44 ± 0.25 |
| `sqruff` | 3366.2 ± 137.9 | 3158.8 | 3490.6 | 679.67 ± 39.03 |


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

## Speed

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.0 ± 0.2 | 1.8 | 3.5 | 1.00 |
| `sqlite3` | 4.9 ± 0.5 | 4.1 | 7.4 | 2.40 ± 0.34 |
| `sqlite-runner-lsp` | 10062.5 ± 11.3 | 10041.4 | 10073.1 | 4972.36 ± 492.26 |
| `sql-lint` | 351.8 ± 9.2 | 343.0 | 374.0 | 173.85 ± 17.80 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 7.4 ± 0.6 | 6.9 | 14.3 | 1.00 |
| `sqlite3` | 10.0 ± 0.5 | 9.2 | 13.9 | 1.35 ± 0.13 |
| `sqlite-runner-lsp` | 10069.4 ± 11.8 | 10055.6 | 10088.1 | 1365.78 ± 107.71 |
| `sql-lint` | 379.7 ± 4.8 | 373.5 | 388.6 | 51.51 ± 4.11 |


# LSP

Feature testing for SQLite-aware language servers.

## Features

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

## Startup + Response Speed

Time to start server, send document, receive diagnostics, and exit:

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 32.2 ± 1.0 | 30.1 | 35.2 | 1.00 |
| `sqls` | 10055.0 ± 14.0 | 10040.8 | 10071.4 | 312.71 ± 9.26 |
| `sql-language-server` | 471.0 ± 3.1 | 465.4 | 474.5 | 14.65 ± 0.44 |

