+++
title = "Competitive Comparison"
weight = 10
+++
# syntaqlite — Competitive Comparison

SQLite SQL tooling landscape.


> Generated on `arm64-darwin` with syntaqlite `unknown` on 2026-03-12.
> To reproduce: `tools/run-comparison --setup && tools/run-comparison --all`.
> See [detailed results](@/reference/comparison-details.md) for per-statement breakdowns.


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

**sqlite3**:

```
Error: in prepare, table monthly_stats has 2 values for 3 columns
```

## Speed

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
| `syntaqlite` | 35.2 ± 1.8 | 32.1 | 40.8 | 1.00 |
| `sqls` | 10053.2 ± 10.9 | 10042.8 | 10071.4 | 285.55 ± 14.66 |
| `sql-language-server` | 489.7 ± 9.9 | 479.5 | 506.1 | 13.91 ± 0.77 |

