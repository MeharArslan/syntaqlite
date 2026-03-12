"""
Markdown rendering for syntaqlite competitive comparison.

Reads cached JSON results and produces markdown for docs.
"""

import re

# Used by render_detail to reference the same formatter/validator tool orders
FORMATTER_NAMES = ["syntaqlite", "prettier-cst", "sql-formatter", "sqlglot[c]", "sleek", "sqruff"]


def _md_table(headers, rows, align=None):
    """Render a markdown table as a list of lines."""
    cols = len(headers)
    if align is None:
        align = ['l'] * cols
    widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            plain = re.sub(r'\033\[[0-9;]*m', '', str(cell))
            widths[i] = max(widths[i], len(plain))
    hdr = '| ' + ' | '.join(
        h.ljust(widths[i]) if align[i] == 'l' else h.rjust(widths[i]) if align[i] == 'r' else h.center(widths[i])
        for i, h in enumerate(headers)
    ) + ' |'
    sep_parts = []
    for i in range(cols):
        if align[i] == 'r':
            sep_parts.append('-' * (widths[i] - 1) + ':')
        elif align[i] == 'c':
            sep_parts.append(':' + '-' * (widths[i] - 2) + ':')
        else:
            sep_parts.append('-' * widths[i])
    sep = '| ' + ' | '.join(sep_parts) + ' |'
    lines = [hdr, sep]
    for row in rows:
        cells = []
        for i, cell in enumerate(row):
            s = str(cell)
            plain = re.sub(r'\033\[[0-9;]*m', '', s)
            pad = widths[i] - len(plain)
            if align[i] == 'r':
                cells.append(' ' * pad + s)
            elif align[i] == 'c':
                lp = pad // 2
                rp = pad - lp
                cells.append(' ' * lp + s + ' ' * rp)
            else:
                cells.append(s + ' ' * pad)
        lines.append('| ' + ' | '.join(cells) + ' |')
    return lines


def _emit_table(p, headers, rows, align=None):
    for line in _md_table(headers, rows, align):
        p(line)


def _emit_speed(p, speed, label_1x="bench.sql (1x)", label_30x="bench_30x.sql (30x)"):
    bench_1x = speed.get("bench_1x", {})
    bench_30x = speed.get("bench_30x", {})
    if bench_1x.get("hyperfine_md"):
        p(f"### {label_1x}\n")
        p(bench_1x["hyperfine_md"])
        p("")
    if bench_30x.get("hyperfine_md"):
        p(f"\n### {label_30x}\n")
        p(bench_30x["hyperfine_md"])
        p("")


def _repro_section(p, meta):
    """Emit the 'Reproducing These Results' section."""
    ts = meta.get("timestamp", "unknown")
    ver = meta.get("syntaqlite_version", "unknown")
    plat = meta.get("platform", "unknown")
    # Just date portion
    date = ts[:10] if len(ts) >= 10 else ts

    p(f"\n> Generated on `{plat}` with syntaqlite `{ver}` on {date}.")
    p("> To reproduce: `tools/run-comparison --setup && tools/run-comparison --all`.")
    p("> See [detailed results](@/reference/comparison-details.md) for per-statement breakdowns.\n")


# ─── Summary ──────────────────────────────────────────────────────

def render_summary(results):
    """Generate the summary comparison page from JSON results."""
    lines = []
    p = lines.append

    p("# syntaqlite — Competitive Comparison\n")
    p("SQLite SQL tooling landscape.\n")

    # Add repro info from first available meta
    for cat in ["parser", "formatter", "validator", "lsp"]:
        if cat in results and "meta" in results[cat]:
            _repro_section(p, results[cat]["meta"])
            break

    # ── Parser ──
    if "parser" in results:
        r = results["parser"]
        tallies = r["tallies"]
        total = r["total"]

        p("\n# Parser\n")
        p(f"{total} test statements covering obscure SQLite syntax, validated against sqlite3 "
          "as ground truth.\n")

        p("## Accuracy\n")
        ranked = sorted(tallies.items(), key=lambda x: (-x[1]["correct"], x[1]["reject_valid"]))
        sb_rows = []
        for name, t in ranked:
            pct = t["correct"] * 100 // total
            bar = "█" * (t["correct"] * 20 // total)
            rv = str(t['reject_valid']) if t['reject_valid'] > 0 else "-"
            ai = str(t['accept_invalid']) if t['accept_invalid'] > 0 else "-"
            sb_rows.append([name, f"{t['correct']}/{total} ({pct}%) {bar}", rv, ai])
        _emit_table(p, ["Tool", "Correct", "Rejects Valid", "Accepts Invalid"], sb_rows, ['l', 'l', 'r', 'r'])

        p("\n## Speed\n")
        _emit_speed(p, r["speed"])

    # ── Formatter ──
    if "formatter" in results:
        r = results["formatter"]
        tallies = r["tallies"]
        total = r["total"]
        n_valid = r["n_valid"]
        fmt_names = r.get("formatter_names", FORMATTER_NAMES)

        p("\n# Formatter\n")
        p("Round-trip correctness (format then validate with sqlite3) and speed.\n")

        p("## Accuracy\n")
        sb_rows = []
        for tn in fmt_names:
            t = tallies[tn]
            sb_rows.append([tn, f"{t['format_ok']}/{total}", f"{t['sqlite_ok']}/{n_valid}", str(t['corrupted'])])
        _emit_table(p, ["Tool", "Formats", "SQLite OK", "Corrupt"], sb_rows, ['l', 'r', 'r', 'r'])

        p("\n## Speed\n")
        _emit_speed(p, r["speed"])

    # ── Validator ──
    if "validator" in results:
        r = results["validator"]
        tallies = r["tallies"]
        total = r["total"]
        demo = r["demo_diagnostics"]
        tool_meta = r.get("tool_meta", [])

        p("\n# Validator\n")
        p("Error detection accuracy and diagnostic quality.\n")

        p("## Accuracy\n")
        p(f"Schema: `users`, `orders`, `products`, `order_items`. Ground truth: sqlite3.\n")
        ranked = sorted(tallies.items(), key=lambda x: (-x[1]["correct"], x[1]["fn"]))
        sb_rows = []
        for name, t in ranked:
            approach = next((a for n, a in tool_meta if n == name), "")
            pct = t["correct"] * 100 // total
            bar = "█" * (t["correct"] * 20 // total)
            fn_str = str(t["fn"]) if t["fn"] > 0 else "-"
            fp_str = str(t["fp"]) if t["fp"] > 0 else "-"
            sb_rows.append([name, approach, f"{t['correct']}/{total} {bar}", fn_str, fp_str])
        _emit_table(p, ["Tool", "Approach", "Correct", "Missed", "FP"], sb_rows, ['l', 'l', 'l', 'r', 'r'])

        # Diagnostic quality table
        p("\n## Diagnostic Quality\n")
        p("Query with 2 errors: CTE declares 3 columns but SELECT produces 2, "
          "and typo `ROUDN` instead of `ROUND`.\n")
        dq_rows = []
        for name in ["syntaqlite", "sqlite3", "sqlite-runner-lsp", "sql-lint"]:
            d = demo.get(name, {})
            found = d.get("errors_found", 0)
            finds_all = "Yes" if d.get("finds_all") else "No"
            dym = "Yes" if d.get("did_you_mean") else "No"
            dq_rows.append([name, d.get("approach", ""), f"{found}/2", finds_all, dym])
        _emit_table(p, ["Tool", "Approach", "Errors Found", "Finds All", "Did-you-mean"],
                    dq_rows, ['l', 'l', 'c', 'c', 'c'])

        # Show actual diagnostic output examples
        for name in ["syntaqlite", "sqlite3"]:
            d = demo.get(name, {})
            if d.get("output"):
                p(f"\n**{name}**:\n")
                p("```")
                p(d["output"])
                p("```")

        p("\n## Speed\n")
        _emit_speed(p, r["speed"])

    # ── LSP ──
    if "lsp" in results:
        r = results["lsp"]

        p("\n# LSP\n")
        p("Feature testing for SQLite-aware language servers.\n")

        p("## Features\n")
        p("Each server is started, sent a test file, and probed for completion, hover,\n"
          "diagnostics, and formatting. Results are from actual LSP responses.\n")
        feat_headers = ["Feature"] + r["tool_names"]
        feat_align = ['l'] + ['c'] * len(r["tool_names"])
        _emit_table(p, feat_headers, r["feature_rows"], feat_align)

        p("\n## Startup + Response Speed\n")
        p("Time to start server, send document, receive diagnostics, and exit:\n")
        bench_md = r.get("speed", {}).get("hyperfine_md")
        if bench_md:
            p(bench_md)
            p("")

    return "\n".join(lines) + "\n"


# ─── Detail ──────────────────────────────────────────────────────

def render_detail(results):
    """Generate the detailed comparison page from JSON results."""
    lines = []
    p = lines.append

    p("# syntaqlite — Competitive Comparison\n")
    p("SQLite SQL tooling landscape.\n")

    # ── Parser Detail ──
    if "parser" in results:
        r = results["parser"]
        total = r["total"]
        tool_names = r.get("tool_names", [])

        p("\n# Parser Comparison\n")
        p("Per-statement SQLite SQL parsing accuracy, validated against sqlite3 as ground truth.\n")

        # Ground truth table
        p("## Ground Truth\n")
        p("Validating all test statements against sqlite3:\n")
        gt_rows = []
        for gt in r["ground_truth"]:
            status = "OK" if gt["sqlite3_ok"] else f"SKIP ({gt['error']})"
            gt_rows.append([gt["name"], status])
        _emit_table(p, ["Statement", "sqlite3"], gt_rows)
        n_valid = sum(1 for gt in r["ground_truth"] if gt["sqlite3_ok"])
        p(f"\n**{n_valid}/{total}** statements validated by sqlite3.\n")

        # Per-statement accuracy
        p("## Parser Accuracy\n")
        p("Legend: **PASS** = correctly parses valid SQL, **FAIL** = rejects valid SQL, **FP** = accepts invalid SQL\n")
        headers = ["Test", "sqlite3"] + tool_names
        align = ['l', 'c'] + ['c'] * len(tool_names)
        rows = []
        for stmt in r["per_statement"]:
            row = [stmt["name"][:38], stmt["sqlite3"]]
            for tn in tool_names:
                row.append(stmt["tools"].get(tn, "?"))
            rows.append(row)
        _emit_table(p, headers, rows, align)

        # Scoreboard
        p("\n### Scoreboard\n")
        tallies = r["tallies"]
        ranked = sorted(tallies.items(), key=lambda x: (-x[1]["correct"], x[1]["reject_valid"]))
        sb_rows = []
        for name, t in ranked:
            pct = t["correct"] * 100 // total
            bar = "█" * (t["correct"] * 20 // total)
            rv = str(t['reject_valid']) if t['reject_valid'] > 0 else "-"
            ai = str(t['accept_invalid']) if t['accept_invalid'] > 0 else "-"
            sb_rows.append([name, f"{t['correct']}/{total} ({pct}%) {bar}", rv, ai])
        _emit_table(p, ["Tool", "Correct", "Rejects Valid", "Accepts Invalid"], sb_rows, ['l', 'l', 'r', 'r'])

        # Speed
        p("\n## Parse Speed\n")
        s = r["speed"]
        if s.get("bench_1x", {}).get("description"):
            p(f"- `bench.sql`: {s['bench_1x']['description']}")
        if s.get("bench_30x", {}).get("description"):
            p(f"- `bench_30x.sql`: {s['bench_30x']['description']}\n")
        _emit_speed(p, s)

    # ── Formatter Detail ──
    if "formatter" in results:
        r = results["formatter"]
        total = r["total"]
        n_valid = r["n_valid"]
        fmt_names = r.get("formatter_names", FORMATTER_NAMES)

        p("\n# Formatter Comparison\n")
        p("Round-trip correctness (format then validate with sqlite3) and speed.\n")

        # Ground truth
        p("## Ground Truth\n")
        gt_rows = []
        for gt in r["ground_truth"]:
            status = "OK" if gt["sqlite3_ok"] else f"SKIP ({gt['error']})"
            gt_rows.append([gt["name"], status])
        _emit_table(p, ["Statement", "sqlite3"], gt_rows)
        p(f"\n**{n_valid}/{total}** statements validated by sqlite3.\n")

        # Round-trip validation per statement
        p("## Round-Trip Validation\n")
        p("For each formatter: does the formatted output still pass real SQLite?\n")
        headers = ["Test"] + fmt_names
        align = ['l'] + ['c'] * len(fmt_names)
        rows = []
        for stmt in r["per_statement"]:
            row = [stmt["name"][:38]]
            for tn in fmt_names:
                row.append(stmt["tools"].get(tn, "?"))
            rows.append(row)
        _emit_table(p, headers, rows, align)

        # Scoreboard
        p("\n### Scoreboard\n")
        tallies = r["tallies"]
        sb_rows = []
        for tn in fmt_names:
            t = tallies[tn]
            sb_rows.append([tn, f"{t['format_ok']}/{total}", f"{t['sqlite_ok']}/{n_valid}", str(t['corrupted'])])
        _emit_table(p, ["Tool", "Formats", "SQLite OK", "Corrupt"], sb_rows, ['l', 'r', 'r', 'r'])

        # Corruption details
        if r.get("corruption_details"):
            p("\n### Corruption Details\n")
            cd_rows = [[d["tool"], d["test"], d["error"]] for d in r["corruption_details"]]
            _emit_table(p, ["Tool", "Test", "Error"], cd_rows)

        # Speed
        p("\n## Format Speed\n")
        s = r["speed"]
        if s.get("bench_1x", {}).get("description"):
            p(f"- `bench.sql`: {s['bench_1x']['description']}")
        if s.get("bench_30x", {}).get("description"):
            p(f"- `bench_30x.sql`: {s['bench_30x']['description']}\n")
        _emit_speed(p, s)

        # Slow tools
        if r.get("slow_tools"):
            p("\n### Slow Tools (single timed run)\n")
            slow_rows = [[st["name"], f"{st['time_ms']}ms"] for st in r["slow_tools"]]
            _emit_table(p, ["Tool", "Time"], slow_rows, ['l', 'r'])
        p("")

    # ── Validator Detail ──
    if "validator" in results:
        r = results["validator"]
        total = r["total"]
        demo = r["demo_diagnostics"]
        tool_meta = r.get("tool_meta", [])

        p("\n# Validator Comparison\n")
        p("Error detection accuracy and diagnostic quality.\n")

        # Diagnostic quality showcase
        p("## Diagnostic Quality\n")
        p("A realistic query with subtle errors — how does each tool report them?\n")
        p(f"**Query** (2 errors: CTE declares 3 columns but SELECT produces 2; typo `ROUDN`):\n")
        if r.get("demo_query"):
            p("```sql")
            p(r["demo_query"])
            p("```\n")

        for name in ["syntaqlite", "sqlite3", "sqlite-runner-lsp", "sql-lint"]:
            d = demo.get(name, {})
            p(f"### {name}\n")
            p(f"{d.get('description', '')}\n")
            p("```")
            p(d.get("output", "(no output)"))
            p("```\n")

        # Error detection accuracy
        p("## Error Detection Accuracy\n")
        p(f"Schema: `users`, `orders`, `products`, `order_items`. Ground truth: sqlite3.\n")
        tool_names_val = [n for n, _ in tool_meta]
        headers = ["Test", "Expect"] + tool_names_val
        align = ['l', 'c'] + ['c'] * len(tool_names_val)
        rows = []
        for case in r["per_case"]:
            row = [case["description"][:38], case["expected"]]
            for tn in tool_names_val:
                row.append(case["tools"].get(tn, "?"))
            rows.append(row)
        _emit_table(p, headers, rows, align)

        # Scoreboard
        p("\n### Scoreboard\n")
        tallies = r["tallies"]
        ranked = sorted(tallies.items(), key=lambda x: (-x[1]["correct"], x[1]["fn"]))
        sb_rows = []
        for name, t in ranked:
            approach = next((a for n, a in tool_meta if n == name), "")
            pct = t["correct"] * 100 // total
            bar = "█" * (t["correct"] * 20 // total)
            fn_str = str(t["fn"]) if t["fn"] > 0 else "-"
            fp_str = str(t["fp"]) if t["fp"] > 0 else "-"
            sb_rows.append([name, approach, f"{t['correct']}/{total} {bar}", fn_str, fp_str])
        _emit_table(p, ["Tool", "Approach", "Correct", "Missed", "FP"], sb_rows, ['l', 'l', 'l', 'r', 'r'])

        # Speed
        p("\n## Validation Speed\n")
        s = r["speed"]
        if s.get("bench_1x", {}).get("description"):
            p(f"- `bench.sql`: {s['bench_1x']['description']}")
        if s.get("bench_30x", {}).get("description"):
            p(f"- `bench_30x.sql`: {s['bench_30x']['description']}\n")
        _emit_speed(p, s)

    # ── LSP Detail ──
    if "lsp" in results:
        r = results["lsp"]
        tool_names_lsp = r["tool_names"]

        p("\n# LSP Comparison\n")
        p("Feature testing for SQLite-aware language servers.\n")

        p("## Tested Capabilities\n")
        p("Each server is started, sent a test file, and probed for completion, hover,\n"
          "diagnostics, and formatting. Results are from actual LSP responses.\n")
        feat_headers = ["Feature"] + tool_names_lsp
        feat_align = ['l'] + ['c'] * len(tool_names_lsp)
        _emit_table(p, feat_headers, r["feature_rows"], feat_align)

        # Diagnostic detail
        if r.get("diagnostic_detail"):
            p("\n## Diagnostic Detail\n")
            p("What each server reports for `SELEC * FROM users;` (syntax error):\n")
            for name in tool_names_lsp:
                detail = r["diagnostic_detail"].get(name, "(no data)")
                p(f"### {name}\n")
                p("```")
                p(detail)
                p("```\n")

        # Speed
        p("## LSP Startup + Response Speed\n")
        p("Time to start server, send document, receive diagnostics, and exit:\n")
        bench_md = r.get("speed", {}).get("hyperfine_md")
        if bench_md:
            p(bench_md)
        p("")

    return "\n".join(lines) + "\n"


# ─── Write docs ──────────────────────────────────────────────────

SUMMARY_FRONTMATTER = """\
+++
title = "Competitive Comparison"
weight = 10
+++
"""

DETAIL_FRONTMATTER = """\
+++
title = "Comparison Details"
weight = 11
+++
"""


def write_docs(results, docs_dir):
    """Write both summary and detail markdown files with Zola frontmatter."""
    import os
    os.makedirs(docs_dir, exist_ok=True)

    summary = render_summary(results)
    summary_path = os.path.join(docs_dir, "comparison.md")
    with open(summary_path, "w") as f:
        f.write(SUMMARY_FRONTMATTER)
        f.write(summary)
    print(f"Written: {summary_path}")

    detail = render_detail(results)
    detail_path = os.path.join(docs_dir, "comparison-details.md")
    with open(detail_path, "w") as f:
        f.write(DETAIL_FRONTMATTER)
        f.write(detail)
    print(f"Written: {detail_path}")
