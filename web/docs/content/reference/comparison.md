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
| sqlfluff        | 40/40 (100%) ████████████████████ |             - |               - |
| lemon-rs        | 0/40 (0%)                         |            40 |               - |
| sql-parser-cst  | 0/40 (0%)                         |            40 |               - |
| sqlglot[c]      | 0/40 (0%)                         |            40 |               - |
| sqlparser-rs    | 0/40 (0%)                         |            40 |               - |
| node-sql-parser | 0/40 (0%)                         |            40 |               - |

## Speed

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.8 ± 0.7 | 1.6 | 24.4 | 1.00 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.6 ± 0.3 | 2.4 | 7.3 | 1.00 |


# Formatter

Round-trip correctness (format then validate with sqlite3) and speed.

## Accuracy

| Tool          | Formats | SQLite OK | Corrupt |
| ------------- | ------: | --------: | ------: |
| syntaqlite    |   40/40 |     40/40 |       0 |
| prettier-cst  |    0/40 |      0/40 |       0 |
| sql-formatter |    0/40 |      0/40 |       0 |
| sqlglot[c]    |    0/40 |      0/40 |       0 |
| sleek         |   40/40 |     38/40 |       2 |
| sqruff        |   40/40 |     38/40 |       2 |

## Speed

### bench.sql (1x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.8 ± 0.1 | 1.7 | 3.2 | 1.00 |


### bench_30x.sql (30x)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 4.9 ± 0.6 | 4.7 | 14.4 | 1.00 |


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
| `syntaqlite` | 39.2 ± 5.8 | 34.4 | 63.9 | 1.00 |
| `sqls` | 10059.2 ± 7.9 | 10053.2 | 10071.7 | 256.41 ± 37.86 |
| `sql-language-server` | 659.1 ± 25.3 | 640.3 | 702.7 | 16.80 ± 2.56 |

