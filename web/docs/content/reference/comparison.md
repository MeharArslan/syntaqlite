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
| `syntaqlite` | 1.6 ± 0.1 | 1.6 | 2.3 | 1.17 ± 0.07 |
| `lemon-rs` | 1.4 ± 0.1 | 1.3 | 1.8 | 1.00 |
| `sql-parser-cst` | 75.0 ± 1.2 | 73.1 | 77.7 | 53.83 ± 2.28 |
| `sqlglot[c]` | 84.1 ± 1.0 | 82.2 | 86.7 | 60.34 ± 2.49 |
| `sqlparser-rs` | 1.7 ± 0.1 | 1.6 | 2.9 | 1.25 ± 0.08 |
| `node-sql-parser` | 73.3 ± 1.1 | 71.8 | 76.2 | 52.64 ± 2.23 |
| `sqlfluff` | 447.9 ± 2.1 | 444.2 | 450.9 | 321.50 ± 12.80 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 2.5 ± 0.1 | 2.4 | 3.3 | 1.00 |
| `lemon-rs` | 4.1 ± 0.1 | 4.0 | 4.8 | 1.65 ± 0.08 |
| `sql-parser-cst` | 141.2 ± 1.4 | 138.9 | 143.7 | 56.11 ± 2.25 |
| `sqlglot[c]` | 181.0 ± 2.0 | 178.3 | 186.4 | 71.93 ± 2.91 |
| `sqlparser-rs` | 10.7 ± 0.3 | 10.2 | 12.3 | 4.25 ± 0.21 |
| `node-sql-parser` | 149.0 ± 1.3 | 146.9 | 152.5 | 59.23 ± 2.37 |
| `sqlfluff` | 6373.2 ± 59.0 | 6304.7 | 6464.7 | 2532.62 ± 101.43 |

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
| `syntaqlite` | 1.8 ± 0.1 | 1.7 | 2.5 | 1.00 |
| `prettier-cst` | 417.9 ± 11.3 | 406.0 | 444.3 | 235.77 ± 12.38 |
| `sql-formatter` | 80.2 ± 10.1 | 72.7 | 112.5 | 45.25 ± 6.04 |
| `sqlglot[c]` | 96.2 ± 16.1 | 85.8 | 156.9 | 54.27 ± 9.41 |
| `sleek` | 8.7 ± 1.0 | 7.8 | 16.6 | 4.91 ± 0.58 |
| `sqruff` | 39.6 ± 0.7 | 38.7 | 41.6 | 22.36 ± 1.08 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 4.9 ± 0.1 | 4.7 | 5.7 | 1.00 |
| `prettier-cst` | 564.0 ± 3.8 | 558.8 | 569.3 | 114.45 ± 3.05 |
| `sql-formatter` | 198.6 ± 1.9 | 195.5 | 201.8 | 40.30 ± 1.11 |
| `sqlglot[c]` | 264.2 ± 1.6 | 261.2 | 266.1 | 53.62 ± 1.42 |
| `sleek` | 27.3 ± 1.4 | 26.3 | 38.2 | 5.55 ± 0.31 |
| `sqruff` | 3047.1 ± 21.7 | 3022.0 | 3074.5 | 618.32 ± 16.56 |

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
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpqpviyg04.sql:30:3
   |
30 |   monthly_stats(month, revenue, order_count) AS (
   |   ^~~~~~~~~~~~~
warning: unknown function 'ROUDN'
  --> /var/folders/rx/t6_rqmqx0f15l7kgp7yjhcbc0000gn/T/tmpqpviyg04.sql:42:3
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
| `syntaqlite` | 2.0 ± 0.1 | 1.8 | 3.7 | 1.00 |
| `sqlite3` | 5.0 ± 0.5 | 4.4 | 7.7 | 2.45 ± 0.28 |
| `sqlite-runner-lsp` | 10071.4 ± 7.3 | 10054.6 | 10083.1 | 4972.01 ± 331.20 |
| `sql-lint` | 352.0 ± 3.3 | 347.3 | 356.8 | 173.75 ± 11.69 |

### bench_30x.sql (30×)

| Command | Mean [ms] | Min [ms] | Max [ms] | Relative |
|:---|---:|---:|---:|---:|
| `syntaqlite` | 6.0 ± 0.2 | 5.7 | 6.7 | 1.00 |
| `sqlite3` | 10.2 ± 0.5 | 9.5 | 13.7 | 1.69 ± 0.09 |
| `sqlite-runner-lsp` | 10066.0 ± 8.5 | 10051.0 | 10071.3 | 1677.58 ± 43.06 |
| `sql-lint` | 378.9 ± 3.4 | 374.5 | 383.0 | 63.15 ± 1.71 |

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
| Go to definition       |       Yes       |      Yes      |         No          |
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
| `syntaqlite` | 32.3 ± 0.9 | 30.5 | 35.5 | 1.00 |
| `sqls` | 10072.2 ± 3.2 | 10066.8 | 10074.8 | 311.41 ± 8.53 |
| `sql-language-server` | 462.9 ± 3.5 | 456.3 | 466.2 | 14.31 ± 0.41 |
