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
| `syntaqlite` | 1.7 ± 0.1 | 1.6 | 2.4 | 1.14 ± 0.09 |
| `lemon-rs` | 1.5 ± 0.1 | 1.3 | 3.2 | 1.00 |
| `sql-parser-cst` | 76.5 ± 2.7 | 73.8 | 88.2 | 52.01 ± 4.08 |
| `sqlglot[c]` | 85.1 ± 1.1 | 83.4 | 89.3 | 57.80 ± 4.13 |
| `sqlparser-rs` | 1.8 ± 0.1 | 1.7 | 3.2 | 1.22 ± 0.12 |
| `node-sql-parser` | 74.0 ± 1.5 | 71.4 | 79.2 | 50.29 ± 3.67 |
| `sqlfluff` | 463.8 ± 15.0 | 446.3 | 484.6 | 315.17 ± 24.37 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.6 ± 0.3 | 2.4 | 9.0 | 1.00 |
| `lemon-rs` | 4.2 ± 0.3 | 4.0 | 8.7 | 1.62 ± 0.23 |
| `sql-parser-cst` | 143.4 ± 2.4 | 140.4 | 148.6 | 54.87 ± 7.04 |
| `sqlglot[c]` | 198.0 ± 17.3 | 179.6 | 236.5 | 75.80 ± 11.69 |
| `sqlparser-rs` | 11.7 ± 2.0 | 10.5 | 41.9 | 4.48 ± 0.96 |
| `node-sql-parser` | 150.0 ± 2.4 | 146.6 | 156.5 | 57.41 ± 7.36 |
| `sqlfluff` | 6408.9 ± 55.5 | 6343.9 | 6482.8 | 2452.81 ± 312.56 |

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
| syntaqlite    |   40/40 |       - |       - |
| prettier-cst  |   39/40 |       - |       1 |
| sql-formatter |   39/40 |       - |       1 |
| sqlglot[c]    |   31/40 |       4 |       5 |
| sleek         |   37/40 |       3 |       - |
| sqruff        |   33/40 |       2 |       5 |

## Speed

### bench.sql (1×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 1.8 ± 0.1 | 1.7 | 2.2 | 1.00 |
| `prettier-cst` | 404.8 ± 3.7 | 399.8 | 410.1 | 228.12 ± 7.92 |
| `sql-formatter` | 74.5 ± 0.9 | 73.1 | 76.7 | 41.96 ± 1.49 |
| `sqlglot[c]` | 86.2 ± 1.3 | 84.8 | 89.1 | 48.56 ± 1.78 |
| `sleek` | 8.2 ± 0.3 | 7.8 | 10.2 | 4.60 ± 0.21 |
| `sqruff` | 39.3 ± 0.7 | 38.4 | 41.9 | 22.14 ± 0.83 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 4.9 ± 0.1 | 4.7 | 6.4 | 1.00 |
| `prettier-cst` | 551.8 ± 6.8 | 541.6 | 558.2 | 112.34 ± 3.55 |
| `sql-formatter` | 195.5 ± 0.9 | 193.5 | 197.0 | 39.79 ± 1.17 |
| `sqlglot[c]` | 261.0 ± 1.6 | 258.6 | 264.4 | 53.14 ± 1.59 |
| `sleek` | 26.7 ± 0.4 | 26.1 | 28.3 | 5.44 ± 0.18 |
| `sqruff` | 3211.8 ± 144.9 | 3068.8 | 3456.6 | 653.88 ± 35.13 |

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
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmp2gompion.sql:30:3
   |
30 |   monthly_stats(month, revenue, order_count) AS (
   |   ^~~~~~~~~~~~~
warning: unknown function 'ROUDN'
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmp2gompion.sql:42:3
   |
42 |   ROUDN(ms.revenue / ms.order_count, 2) AS avg_order
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
| `syntaqlite` | 2.1 ± 0.1 | 1.9 | 3.0 | 1.00 |
| `sqlite3` | 4.9 ± 0.4 | 4.5 | 10.1 | 2.39 ± 0.23 |
| `sqlite-runner-lsp` | 10063.4 ± 8.4 | 10051.4 | 10073.4 | 4907.99 ± 292.51 |
| `sql-lint` | 360.7 ± 10.9 | 341.7 | 375.0 | 175.93 ± 11.76 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 6.2 ± 0.2 | 5.8 | 7.3 | 1.00 |
| `sqlite3` | 10.4 ± 0.5 | 9.5 | 13.6 | 1.69 ± 0.10 |
| `sqlite-runner-lsp` | 10064.0 ± 7.4 | 10051.3 | 10069.8 | 1636.39 ± 56.73 |
| `sql-lint` | 373.6 ± 5.4 | 367.3 | 381.7 | 60.75 ± 2.28 |

---

# LSP (Language Server)

**What we test:** We start each language server, open a test document, and probe
for completions, hover, diagnostics, and formatting. Results are from actual LSP
protocol responses — no self-reported feature lists.

**Key difference:** `sqls` requires a live database connection for its features.
syntaqlite and sql-language-server work offline.

## Features

| Feature                |    syntaqlite   |         sqls         | sql-language-server |
| ---------------------- | :-------------: | :------------------: | :-----------------: |
| Completion             | Yes (136 items) | Advertised (0 items) |   Yes (11 items)    |
| Hover                  |       Yes       |          No          |         No          |
| Go to definition       |       Yes       |         Yes          |         No          |
| Find references        |       Yes       |          No          |         No          |
| Diagnostics: syntax    |       Yes       |          No          |         Yes         |
| Diagnostics: semantic  |       Yes       |          No          |         Yes         |
| Formatting             |       Yes       |         Yes          |         No          |
| Rename                 |       Yes       |         Yes          |         Yes         |
| Signature help         |       Yes       |         Yes          |         No          |
| Requires DB connection |       No        |         Yes          |         No          |

## Startup + response speed

Time from server start → document open → diagnostics received → exit:

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 32.2 ± 1.4 | 30.0 | 40.9 | 1.00 |
| `sqls` | 10058.0 ± 13.0 | 10039.5 | 10069.9 | 312.52 ± 13.81 |
| `sql-language-server` | 473.1 ± 6.0 | 464.8 | 483.3 | 14.70 ± 0.68 |
