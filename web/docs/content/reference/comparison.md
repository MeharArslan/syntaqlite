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

# Formatting

**What we test:** Round-trip semantic preservation. Each of the same 40
statements is formatted, then we run `EXPLAIN` on both the original and
formatted SQL and compare the bytecode sqlite3 produces. Identical bytecode
means sqlite3 will execute the exact same operations — the formatter preserved
semantics, not just validity. Tools that crash or refuse to format score
"refused". Tools whose output produces different bytecode score "corrupt".

**Why bytecode, not just acceptance?** A formatter could subtly alter your SQL
(reorder expressions, change operator grouping) in a way sqlite3 still accepts
but that produces different results. Bytecode comparison catches these silent
semantic changes. For statements where `EXPLAIN` isn't applicable (e.g.
`PRAGMA`, `ATTACH`), we fall back to acceptance-only.

## Accuracy

| Tool          | Correct | Corrupt | Refused |
| ------------- | ------: | ------: | ------: |
| syntaqlite    |   33/40 |       7 |       - |
| prettier-cst  |   34/40 |       5 |       1 |
| sql-formatter |   34/40 |       5 |       1 |
| sqlglot[c]    |   30/40 |       5 |       5 |
| sleek         |   32/40 |       8 |       - |
| sqruff        |   29/40 |       6 |       5 |

## Speed

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

**sqlite3** stops at the first error (runtime execution):

```
Error: in prepare, table monthly_stats has 2 values for 3 columns
```

## Speed

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
| `syntaqlite` | 33.0 ± 0.9 | 30.4 | 35.0 | 1.00 |
| `sqls` | 10065.0 ± 7.2 | 10052.6 | 10070.3 | 304.91 ± 8.47 |
| `sql-language-server` | 469.7 ± 10.6 | 456.3 | 482.1 | 14.23 ± 0.51 |
