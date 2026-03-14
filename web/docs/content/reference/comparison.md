+++
title = "Competitive Comparison"
weight = 10
+++
# How does syntaqlite compare?

There are many SQL tools out there. This page tests them head-to-head on
**SQLite-specific SQL** — the kind of syntax that trips up generic SQL parsers —
and shows raw, reproducible results.

> Generated on `arm64-darwin` with syntaqlite `0.1.0` on 2026-03-14.
> To reproduce: `tools/run-comparison --setup && tools/run-comparison --all`.
> Per-statement breakdowns and reproduction details are in the
> [detailed results](@/reference/comparison-details.md).

---

# Parsing

**What we test:** 40 statements covering advanced SQLite syntax — `UPSERT`,
`RETURNING`, `STRICT` tables, window frames with `EXCLUDE`, numeric underscores,
`IS NOT DISTINCT FROM`, recursive CTEs, and more. Each statement is first
validated against `sqlite3` itself (the ground truth), then run through every
parser. A tool scores "correct" only if it agrees with sqlite3.

**Why syntaqlite scores well:** It embeds SQLite's own Lemon-generated parser.
If sqlite3 accepts a statement, syntaqlite accepts it too — by construction,
not by reimplementation. lemon-rs (a Rust port of the same grammar) scores
similarly for the same reason.

Tools built on hand-written or generic SQL grammars tend to lag behind SQLite's
full syntax, particularly on features added after 3.35 (RETURNING, MATERIALIZED,
IS DISTINCT FROM, numeric underscores).

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

We benchmark two file sizes: a 40-statement file (startup-dominated) and that
file repeated 30× (throughput-dominated).

### bench.sql (1×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.7 ± 0.8 | 1.6 | 31.1 | 1.17 ± 0.54 |
| `lemon-rs` | 1.5 ± 0.1 | 1.4 | 2.0 | 1.00 |
| `sql-parser-cst` | 75.1 ± 1.3 | 73.3 | 79.5 | 51.59 ± 2.70 |
| `sqlglot[c]` | 84.9 ± 1.1 | 83.2 | 87.7 | 58.34 ± 2.97 |
| `sqlparser-rs` | 1.8 ± 0.6 | 1.7 | 17.4 | 1.27 ± 0.39 |
| `node-sql-parser` | 73.5 ± 1.1 | 71.2 | 75.7 | 50.52 ± 2.60 |
| `sqlfluff` | 449.0 ± 2.5 | 446.1 | 454.2 | 308.48 ± 15.26 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.6 ± 0.8 | 2.4 | 27.1 | 1.00 |
| `lemon-rs` | 4.2 ± 0.4 | 4.0 | 13.2 | 1.62 ± 0.53 |
| `sql-parser-cst` | 141.4 ± 1.5 | 138.9 | 144.0 | 54.31 ± 17.10 |
| `sqlglot[c]` | 182.7 ± 1.1 | 180.8 | 185.7 | 70.17 ± 22.09 |
| `sqlparser-rs` | 11.2 ± 2.1 | 10.5 | 45.8 | 4.30 ± 1.58 |
| `node-sql-parser` | 150.2 ± 1.0 | 148.5 | 152.2 | 57.70 ± 18.16 |
| `sqlfluff` | 254.0 ± 2.7 | 249.2 | 258.1 | 97.57 ± 30.72 |

---

# Formatting

**What we test:** Round-trip correctness. Each of the same 40 statements is
formatted, then the formatted output is validated against `sqlite3`. A tool
scores "correct" only if sqlite3 still accepts the formatted SQL. Tools that
crash or refuse to format a statement score "fail". Tools that produce output
sqlite3 rejects score "corrupt" — the most dangerous failure mode.

**Why this matters:** A formatter that silently changes the meaning of your SQL
is worse than one that refuses to format it. The "corrupt" column is the most
important.

## Accuracy

| Tool          | Correct | Corrupt | Refused |
| ------------- | ------: | ------: | ------: |
| syntaqlite    |   40/40 |       - |       - |
| prettier-cst  |   39/40 |       - |       1 |
| sql-formatter |   39/40 |       - |       1 |
| sqlglot[c]    |   31/40 |       4 |       5 |
| sleek         |   38/40 |       2 |       - |
| sqruff        |   33/40 |       2 |       5 |

## Speed

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

---

# Validation

**What we test:** Can the tool catch real SQL errors — unknown tables, bad
column references, wrong function arity, CTE column mismatches — without
running the query? We define 24 test cases (15 with intentional errors, 9
valid) against a known schema, and check each tool's verdict against sqlite3.

**Why syntaqlite is different:** Most SQL "linters" do structural/style checks
(trailing commas, missing parens). syntaqlite does static *semantic* analysis —
it resolves table/column references, checks function signatures, and validates
CTE column counts, all without a database connection.

## Accuracy

Schema: `users`, `orders`, `products`, `order_items`. Ground truth: sqlite3.

| Tool              | Approach          | Correct                    | Missed | FP |
| ----------------- | ----------------- | -------------------------- | -----: | -: |
| sqlite3           | runtime execution | 24/24 ████████████████████ |      - |  - |
| syntaqlite        | static semantic   | 23/24 ███████████████████  |      1 |  - |
| sql-lint          | structural checks | 12/24 ██████████           |     11 |  1 |
| sqlite-runner-lsp | runtime via LSP   | 9/24 ███████               |     15 |  - |

## Diagnostic quality

A real query with 2 subtle errors — CTE declares 3 columns but SELECT produces
2, and a typo `ROUDN` instead of `ROUND`:

| Tool              | Approach          | Errors Found | Finds All | Did-you-mean |
| ----------------- | ----------------- | :----------: | :-------: | :----------: |
| syntaqlite        | static semantic   |     2/2      |    Yes    |     Yes      |
| sqlite3           | runtime execution |     1/2      |    No     |      No      |
| sqlite-runner-lsp | runtime via LSP   |     0/2      |    No     |      No      |
| sql-lint          | structural checks |     0/2      |    No     |      No      |

**syntaqlite** finds both errors in one pass, with source locations and a
did-you-mean suggestion:

```
error: table 'monthly_stats' has 2 values for 3 columns
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpvs13_h7h.sql:29:3
   |
29 |   monthly_stats(month, revenue, order_count) AS (
   |   ^~~~~~~~~~~~~
warning: unknown function 'ROUDN'
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpvs13_h7h.sql:41:3
   |
41 |   ROUDN(ms.revenue / ms.order_count, 2) AS avg_order
   |   ^~~~~
   = help: did you mean 'round'?
```

**sqlite3** stops at the first error (runtime execution):

```
Error: in prepare, table monthly_stats has 2 values for 3 columns
```

## Speed

### bench.sql (1×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.1 ± 0.3 | 1.8 | 6.2 | 1.00 |
| `sqlite3` | 5.0 ± 0.5 | 4.4 | 7.9 | 2.40 ± 0.37 |
| `sqlite-runner-lsp` | 10071.4 ± 3.9 | 10063.7 | 10077.7 | 4833.40 ± 597.15 |
| `sql-lint` | 348.6 ± 2.8 | 343.4 | 351.7 | 167.30 ± 20.71 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 8.7 ± 0.2 | 8.3 | 9.6 | 1.00 |
| `sqlite3` | 10.3 ± 2.7 | 9.5 | 54.9 | 1.19 ± 0.31 |
| `sqlite-runner-lsp` | 10074.5 ± 1.6 | 10072.6 | 10076.6 | 1160.54 ± 26.26 |
| `sql-lint` | 374.4 ± 5.6 | 366.0 | 384.0 | 43.13 ± 1.17 |

---

# LSP (Language Server)

**What we test:** We start each language server, open a test document, and probe
for completions, hover, diagnostics, and formatting. Results are from actual LSP
protocol responses — no self-reported feature lists.

**Key difference:** `sqls` requires a live database connection for its features.
syntaqlite and sql-language-server work offline.

## Features

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

## Startup + response speed

Time from server start → document open → diagnostics received → exit:

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 32.6 ± 1.1 | 30.6 | 35.6 | 1.00 |
| `sqls` | 10074.0 ± 3.8 | 10068.3 | 10078.0 | 309.00 ± 10.50 |
| `sql-language-server` | 457.2 ± 4.7 | 449.4 | 462.1 | 14.02 ± 0.50 |
