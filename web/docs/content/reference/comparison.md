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
| `syntaqlite` | 1.8 ± 0.8 | 1.6 | 34.0 | 1.09 ± 0.83 |
| `lemon-rs` | 1.6 ± 1.0 | 1.4 | 40.7 | 1.00 |
| `sql-parser-cst` | 78.2 ± 8.0 | 73.7 | 122.7 | 48.12 ± 29.44 |
| `sqlglot[c]` | 91.2 ± 10.8 | 84.1 | 134.7 | 56.09 ± 34.47 |
| `sqlparser-rs` | 2.1 ± 3.0 | 1.7 | 123.5 | 1.30 ± 2.02 |
| `node-sql-parser` | 79.7 ± 5.9 | 73.1 | 94.0 | 49.04 ± 29.80 |
| `sqlfluff` | 504.9 ± 81.6 | 446.6 | 707.0 | 310.62 ± 193.98 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.6 ± 0.1 | 2.4 | 3.7 | 1.00 |
| `lemon-rs` | 4.7 ± 1.0 | 4.1 | 13.1 | 1.82 ± 0.40 |
| `sql-parser-cst` | 149.5 ± 14.0 | 140.6 | 192.8 | 57.83 ± 6.00 |
| `sqlglot[c]` | 227.2 ± 32.4 | 189.9 | 286.9 | 87.87 ± 13.11 |
| `sqlparser-rs` | 11.9 ± 1.2 | 10.7 | 18.7 | 4.60 ± 0.51 |
| `node-sql-parser` | 158.2 ± 14.2 | 150.0 | 211.8 | 61.21 ± 6.11 |
| `sqlfluff` | 12716.5 ± 814.3 | 11402.3 | 13606.4 | 4918.86 ± 382.34 |

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
| syntaqlite    |   36/40 |       4 |       - |
| prettier-cst  |   38/40 |       1 |       1 |
| sql-formatter |   38/40 |       1 |       1 |
| sqlglot[c]    |   31/40 |       4 |       5 |
| sleek         |   36/40 |       4 |       - |
| sqruff        |   32/40 |       3 |       5 |

## Speed

### bench.sql (1×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 9.3 ± 8.2 | 4.0 | 99.1 | 1.00 |
| `prettier-cst` | 1146.5 ± 471.1 | 639.2 | 1783.0 | 123.29 ± 120.34 |
| `sql-formatter` | 267.3 ± 54.6 | 166.1 | 366.2 | 28.74 ± 26.11 |
| `sqlglot[c]` | 134.7 ± 32.8 | 106.8 | 202.3 | 14.48 ± 13.30 |
| `sleek` | 13.2 ± 9.2 | 9.0 | 165.1 | 1.42 ± 1.60 |
| `sqruff` | 54.1 ± 3.4 | 49.8 | 64.1 | 5.82 ± 5.16 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 7.6 ± 1.8 | 6.1 | 32.1 | 1.00 |
| `prettier-cst` | 787.7 ± 15.9 | 770.3 | 808.5 | 103.90 ± 24.38 |
| `sql-formatter` | 330.6 ± 6.0 | 320.4 | 337.6 | 43.60 ± 10.23 |
| `sqlglot[c]` | 364.3 ± 10.9 | 354.6 | 389.9 | 48.05 ± 11.33 |
| `sleek` | 38.9 ± 1.5 | 35.6 | 42.6 | 5.12 ± 1.21 |
| `sqruff` | 4694.8 ± 371.6 | 4267.5 | 5071.1 | 619.24 ± 152.85 |

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
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpoh8hxfpk.sql:29:3
   |
29 |   monthly_stats(month, revenue, order_count) AS (
   |   ^~~~~~~~~~~~~
warning: unknown function 'ROUDN'
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpoh8hxfpk.sql:41:3
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
| `syntaqlite` | 6.6 ± 3.5 | 3.4 | 36.9 | 1.00 |
| `sqlite3` | 13.4 ± 7.2 | 7.2 | 76.0 | 2.02 ± 1.51 |
| `sqlite-runner-lsp` | 10089.9 ± 49.5 | 10049.1 | 10193.2 | 1519.61 ± 795.09 |
| `sql-lint` | 769.0 ± 48.7 | 724.3 | 875.7 | 115.82 ± 61.04 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 19.2 ± 2.3 | 14.9 | 30.7 | 1.00 |
| `sqlite3` | 29.2 ± 7.6 | 20.8 | 78.1 | 1.52 ± 0.44 |
| `sqlite-runner-lsp` | 10043.6 ± 4.7 | 10039.0 | 10049.9 | 523.07 ± 63.20 |
| `sql-lint` | 371.6 ± 4.3 | 364.6 | 376.7 | 19.35 ± 2.35 |

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
| `syntaqlite` | 33.2 ± 1.0 | 31.2 | 36.4 | 1.00 |
| `sqls` | 10071.9 ± 1.6 | 10070.2 | 10074.6 | 303.36 ± 9.41 |
| `sql-language-server` | 461.2 ± 6.9 | 455.9 | 473.1 | 13.89 ± 0.48 |
