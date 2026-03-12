#!/usr/bin/env python3
"""
Structured comparison of SQLite SQL tools across categories:
parser, formatter, validator, LSP.

Each test statement gets its own temp file so tools are tested individually.
"""

import subprocess
import tempfile
import os
import sys
import time
import re
import shutil
import difflib
from concurrent.futures import ThreadPoolExecutor, as_completed

SCRIPT_DIR = os.path.dirname(os.path.abspath(__file__))
REPO = os.path.normpath(os.path.join(SCRIPT_DIR, "..", "..", ".."))
DIR = os.path.join(REPO, "tests", "comparison")  # test data directory
SYNQ = os.path.join(REPO, "target", "release", "syntaqlite")
LEMON_RS = os.path.join(DIR, "parser", "target", "release", "lemon-rs-parse")
SQLPARSER_RS = os.path.join(DIR, "parser", "target", "release", "sqlparser-parse")
UV = "uv"
BASE_DB = os.path.join(DIR, "formatter", "_test_schema.db")

# Schema covering all tables/indexes referenced by test_statements.sql.
# Used as ground truth: copy per-statement, run EXPLAIN to validate parse+semantics.
TEST_SCHEMA_SQL = """\
CREATE TABLE users(id INTEGER PRIMARY KEY, email TEXT, name TEXT, active INT, deleted_at TEXT, nickname TEXT, username TEXT, status TEXT, last_login TEXT);
CREATE TABLE orders(id INTEGER PRIMARY KEY, customer_id INT, status TEXT, total REAL, updated_at TEXT, created_at TEXT);
CREATE TABLE customers(id INTEGER PRIMARY KEY, active INT);
CREATE TABLE order_items(id INTEGER PRIMARY KEY, order_id INT, product_id INT, qty INT, price REAL);
CREATE TABLE employees(id INTEGER PRIMARY KEY, salary REAL, department TEXT, dept_id INT, active INT);
CREATE TABLE departments(id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE inventory(sku TEXT, warehouse TEXT, qty INT, price REAL, PRIMARY KEY(sku, warehouse));
CREATE UNIQUE INDEX idx_inventory_sku ON inventory(sku);
CREATE TABLE sensors(id TEXT PRIMARY KEY);
CREATE TABLE audit_log(id INTEGER PRIMARY KEY, tbl TEXT, row_id INT, old_val TEXT);
CREATE TABLE kv(key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE data(id INTEGER PRIMARY KEY, value REAL);
CREATE TABLE files(id INTEGER PRIMARY KEY, path TEXT, name TEXT);
CREATE TABLE counters(name TEXT PRIMARY KEY, count INT);
CREATE TABLE products(id INTEGER PRIMARY KEY, name TEXT, price REAL, category_id INT, category TEXT, status TEXT, nickname TEXT, username TEXT);
CREATE TABLE categories(id INTEGER PRIMARY KEY, name TEXT);
CREATE TABLE sessions(id INTEGER PRIMARY KEY, user_id INT, last_active TEXT, expires_at TEXT);
CREATE TABLE archived_users(id INTEGER PRIMARY KEY, name TEXT, email TEXT);
CREATE TABLE settings(key TEXT PRIMARY KEY, value TEXT);
CREATE TABLE metrics(name TEXT, ts TEXT, value REAL, updated_count INT DEFAULT 0, PRIMARY KEY(name, ts));
CREATE TABLE transactions(id INTEGER PRIMARY KEY, category TEXT, amount REAL);
CREATE TABLE measurements(id INTEGER PRIMARY KEY, sensor_id TEXT REFERENCES sensors(id), raw_value REAL, unit TEXT DEFAULT 'celsius', calibrated REAL, recorded_at TEXT);
CREATE INDEX idx_orders_status ON orders(status);
CREATE INDEX idx_measurements_sensor ON measurements(sensor_id);
CREATE TABLE docs(id INTEGER PRIMARY KEY, title TEXT, body TEXT);
CREATE TABLE docs_json(id INTEGER PRIMARY KEY, data TEXT);
CREATE TABLE computed(a INTEGER NOT NULL, b INTEGER NOT NULL);
"""

# Statements that can't be wrapped in EXPLAIN (they are themselves meta/control stmts).
# For these, run directly on a disposable DB copy.
NO_EXPLAIN_PREFIXES = ('ATTACH', 'DETACH', 'PRAGMA', 'EXPLAIN', 'ANALYZE',
                       'SAVEPOINT', 'RELEASE', 'REINDEX', 'VACUUM')

G = "\033[0;32m"
R = "\033[0;31m"
Y = "\033[0;33m"
C = "\033[0;36m"
N = "\033[0m"
B = "\033[1m"
DIM = "\033[2m"


def md_table(headers, rows, align=None):
    """Print a markdown table. align is a list of 'l', 'r', or 'c' per column."""
    cols = len(headers)
    if align is None:
        align = ['l'] * cols
    # Compute column widths
    widths = [len(h) for h in headers]
    for row in rows:
        for i, cell in enumerate(row):
            # Strip ANSI codes for width calculation
            plain = re.sub(r'\033\[[0-9;]*m', '', str(cell))
            widths[i] = max(widths[i], len(plain))
    # Print header
    hdr = '| ' + ' | '.join(h.ljust(widths[i]) if align[i] == 'l' else h.rjust(widths[i]) if align[i] == 'r' else h.center(widths[i]) for i, h in enumerate(headers)) + ' |'
    # Separator
    sep_parts = []
    for i in range(cols):
        if align[i] == 'r':
            sep_parts.append('-' * (widths[i] - 1) + ':')
        elif align[i] == 'c':
            sep_parts.append(':' + '-' * (widths[i] - 2) + ':')
        else:
            sep_parts.append('-' * widths[i])
    sep = '| ' + ' | '.join(sep_parts) + ' |'
    print(hdr)
    print(sep)
    # Rows
    for row in rows:
        cells = []
        for i, cell in enumerate(row):
            s = str(cell)
            plain = re.sub(r'\033\[[0-9;]*m', '', s)
            # Pad based on plain text width, but use the original (possibly colored) string
            pad = widths[i] - len(plain)
            if align[i] == 'r':
                cells.append(' ' * pad + s)
            elif align[i] == 'c':
                lp = pad // 2
                rp = pad - lp
                cells.append(' ' * lp + s + ' ' * rp)
            else:
                cells.append(s + ' ' * pad)
        print('| ' + ' | '.join(cells) + ' |')


def run(cmd, input_text=None, timeout=30, cwd=None):
    try:
        p = subprocess.run(
            cmd, shell=True, capture_output=True, text=True,
            timeout=timeout, cwd=cwd or DIR, input=input_text,
        )
        return p.returncode == 0, p.stdout, p.stderr
    except subprocess.TimeoutExpired:
        return False, "", "TIMEOUT"
    except Exception as e:
        return False, "", str(e)


def _tmpfile(sql):
    f = tempfile.NamedTemporaryFile(suffix=".sql", mode="w", delete=False)
    f.write(sql)
    f.flush()
    f.close()
    return f.name


def create_test_db():
    """Create (or recreate) the base test schema DB used for SQLite ground-truth validation."""
    os.makedirs(os.path.dirname(BASE_DB), exist_ok=True)
    if os.path.exists(BASE_DB):
        os.unlink(BASE_DB)
    p = subprocess.run(
        ["sqlite3", BASE_DB], input=TEST_SCHEMA_SQL,
        capture_output=True, text=True,
    )
    if p.returncode != 0:
        print(f"  {R}ERROR creating test schema DB: {p.stderr.strip()}{N}")
        sys.exit(1)


def validate_sql_with_sqlite(sql):
    """Validate a SQL statement against real SQLite.

    Copies the base schema DB, wraps the statement in EXPLAIN (when possible),
    and runs it through sqlite3. Returns (ok, error_message).
    """
    tmp_db = tempfile.mktemp(suffix=".db")
    shutil.copy2(BASE_DB, tmp_db)
    try:
        sql_stripped = sql.strip().rstrip(';').strip()
        upper = sql_stripped.upper()

        # DETACH needs a prior ATTACH; run as two-statement sequence
        if upper.startswith('DETACH'):
            test_sql = "ATTACH ':memory:' AS scratch;\n" + sql
        # Some stmts can't be EXPLAINed — run directly
        elif any(upper.startswith(pfx) for pfx in NO_EXPLAIN_PREFIXES):
            test_sql = sql
        else:
            test_sql = "EXPLAIN " + sql

        p = subprocess.run(
            ["sqlite3", tmp_db, test_sql],
            capture_output=True, timeout=10,
        )
        if p.returncode == 0:
            return True, ""
        # stderr is bytes — decode leniently for error messages
        return False, p.stderr.decode("utf-8", errors="replace").strip()
    except subprocess.TimeoutExpired:
        return False, "TIMEOUT"
    finally:
        if os.path.exists(tmp_db):
            os.unlink(tmp_db)


def get_formatted_sql(tool_name, input_path):
    """Run a formatter tool on a SQL file and return (ok, formatted_sql).

    Returns the formatted SQL text, or None if the tool failed.
    """
    if tool_name == "syntaqlite":
        ok, out, _ = run(f"'{SYNQ}' fmt '{input_path}'")
        return (True, out) if ok else (False, None)
    elif tool_name == "prettier-cst":
        with open(input_path) as f:
            ok, out, _ = run("npx prettier --stdin-filepath t.sql", input_text=f.read(), cwd=DIR)
        return (True, out) if ok else (False, None)
    elif tool_name == "sql-formatter":
        ok, out, _ = run(f"sql-formatter -l sqlite '{input_path}'")
        return (True, out) if ok else (False, None)
    elif tool_name == "sqlglot[c]":
        ok, out, _ = run(
            f"{UV} run python -c \"import sqlglot;[print(e.sql(dialect='sqlite',pretty=True)) "
            f"for e in sqlglot.parse(open('{input_path}').read(),dialect='sqlite') if e]\"",
            timeout=10,
        )
        return (True, out) if ok else (False, None)
    elif tool_name == "sleek":
        tmp = input_path + ".sleek_tmp.sql"
        shutil.copy2(input_path, tmp)
        run(f"sleek '{tmp}'")
        try:
            with open(tmp) as f:
                formatted = f.read()
            return (True, formatted)
        except Exception:
            return (False, None)
        finally:
            if os.path.exists(tmp):
                os.unlink(tmp)
    elif tool_name == "sqruff":
        tmp = input_path + ".sqruff_tmp.sql"
        shutil.copy2(input_path, tmp)
        run(f"sqruff fix '{tmp}' --dialect sqlite")
        try:
            with open(tmp) as f:
                formatted = f.read()
            return (True, formatted)
        except Exception:
            return (False, None)
        finally:
            if os.path.exists(tmp):
                os.unlink(tmp)
    return (False, None)


FORMATTER_NAMES = ["syntaqlite", "prettier-cst", "sql-formatter", "sqlglot[c]", "sleek", "sqruff"]


def load_test_statements():
    """Load test statements from test_statements.sql, split by blank-line-separated blocks."""
    with open(os.path.join(DIR, "test_statements.sql")) as f:
        content = f.read()
    blocks = re.split(r'\n\n+', content.strip())
    tests = []
    for block in blocks:
        lines = block.strip().split('\n')
        name_lines = [l for l in lines if l.startswith('--')]
        sql_lines = [l for l in lines if not l.startswith('--')]
        name = name_lines[0].lstrip('- ').strip() if name_lines else "unnamed"
        sql = '\n'.join(sql_lines).strip()
        if sql:
            tests.append((name, sql))
    return tests


def write_per_stmt_files(tests, out_dir):
    """Write each test statement to its own file. Returns list of (name, path)."""
    os.makedirs(out_dir, exist_ok=True)
    results = []
    for i, (name, sql) in enumerate(tests):
        fname = f"t{i+1:02d}.sql"
        path = os.path.join(out_dir, fname)
        with open(path, "w") as f:
            f.write(sql + "\n")
        results.append((name, path))
    return results


# ─── PARSER TOOLS ─────────────────────────────────────────────────────

def parse_syntaqlite(path):
    ok, _, _ = run(f"'{SYNQ}' fmt '{path}'")
    return ok

def parse_lemon_rs(path):
    ok, _, _ = run(f"'{LEMON_RS}' '{path}'")
    return ok

def parse_sqlparser_rs(path):
    ok, _, _ = run(f"'{SQLPARSER_RS}' '{path}'")
    return ok

def parse_sqlglot(path):
    ok, _, _ = run(
        f"{UV} run python -c \""
        f"import sqlglot; [e.sql() for e in sqlglot.parse(open('{path}').read(), dialect='sqlite') if e]"
        f"\"",
        timeout=10,
    )
    return ok

def parse_sql_parser_cst(path):
    script = (
        f'const {{parse}}=require("sql-parser-cst");'
        f'const fs=require("fs");'
        f'try{{parse(fs.readFileSync("{path}","utf8"),{{dialect:"sqlite"}});process.exit(0)}}'
        f'catch(e){{console.error(e.message);process.exit(1)}}'
    )
    ok, _, _ = run(f"node -e '{script}'", timeout=10)
    return ok

def parse_sqlfluff(path):
    ok, out, err = run(f"{UV} run sqlfluff parse '{path}' --dialect sqlite", timeout=30)
    return "unparsable" not in (out + err).lower()

def parse_sqruff(path):
    ok, out, err = run(f"sqruff lint '{path}' --dialect sqlite --parsing-errors", timeout=10)
    combined = (out + err).lower()
    # sqruff returns non-zero for lint warnings, which is fine.
    # Only count as parse failure if there are actual parse/unparsable errors.
    return "unparsable" not in combined and "panic" not in combined

def parse_node_sql_parser(path):
    script = (
        f'const {{Parser}}=require("node-sql-parser");'
        f'const fs=require("fs");'
        f'const p=new Parser();'
        f'try{{p.astify(fs.readFileSync("{path}","utf8"),{{database:"SQLite"}});process.exit(0)}}'
        f'catch(e){{console.error(e.message);process.exit(1)}}'
    )
    ok, _, _ = run(f"node -e '{script}'", timeout=10)
    return ok


PARSER_TOOLS = [
    ("syntaqlite", parse_syntaqlite),
    ("lemon-rs", parse_lemon_rs),
    ("sql-parser-cst", parse_sql_parser_cst),
    ("sqlglot[c]", parse_sqlglot),
    ("sqlfluff", parse_sqlfluff),
    ("sqlparser-rs", parse_sqlparser_rs),
    ("node-sql-parser", parse_node_sql_parser),
]


# ─── PARSER CATEGORY ─────────────────────────────────────────────────

def run_parser_comparison():
    print(f"\n# Parser Comparison\n")
    print(f"Per-statement SQLite SQL parsing accuracy, validated against sqlite3 as ground truth.\n")

    # Ensure base schema DB exists for ground-truth validation
    create_test_db()

    tests = load_test_statements()
    stmt_dir = os.path.join(DIR, "parser", "stmts")
    stmt_files = write_per_stmt_files(tests, stmt_dir)
    total = len(stmt_files)

    # ── Ground truth (parallel) ──
    print(f"## Ground Truth\n")
    print(f"Validating all test statements against sqlite3:\n")
    ground_truth = {}
    with ThreadPoolExecutor(max_workers=os.cpu_count() or 8) as pool:
        gt_futures = {}
        for name, path in stmt_files:
            with open(path) as f:
                sql = f.read()
            gt_futures[pool.submit(validate_sql_with_sqlite, sql)] = path
        for fut in as_completed(gt_futures):
            path = gt_futures[fut]
            ok, err = fut.result()
            ground_truth[path] = (ok, err)
    gt_rows = []
    for name, path in stmt_files:
        ok, err = ground_truth[path]
        status = "OK" if ok else f"SKIP ({err[:40]})"
        gt_rows.append([name[:50], status])
        ground_truth[path] = ok  # simplify for later use
    md_table(["Statement", "sqlite3"], gt_rows)

    n_valid = sum(ground_truth.values())
    print(f"\n**{n_valid}/{total}** statements validated by sqlite3.\n")

    # ── Accuracy ──
    print(f"## Parser Accuracy\n")
    print(f"Legend: **PASS** = correctly parses valid SQL, **FAIL** = rejects valid SQL, **FP** = accepts invalid SQL\n")

    tallies = {name: {"correct": 0, "reject_valid": 0, "accept_invalid": 0} for name, _ in PARSER_TOOLS}
    headers = ["Test", "sqlite3"] + [n for n, _ in PARSER_TOOLS]
    align = ['l', 'c'] + ['c'] * len(PARSER_TOOLS)

    # Run all tool×statement combinations in parallel
    tool_results = {}  # (stmt_idx, tool_name) -> bool
    with ThreadPoolExecutor(max_workers=os.cpu_count() or 8) as pool:
        futures = {}
        for si, (name, path) in enumerate(stmt_files):
            for tool_name, tool_fn in PARSER_TOOLS:
                fut = pool.submit(tool_fn, path)
                futures[fut] = (si, tool_name)
        for fut in as_completed(futures):
            si, tool_name = futures[fut]
            tool_results[(si, tool_name)] = fut.result()

    rows = []
    for si, (name, path) in enumerate(stmt_files):
        sqlite_ok = ground_truth[path]
        row = [name[:38], "OK" if sqlite_ok else "ERR"]
        for tool_name, _ in PARSER_TOOLS:
            tool_ok = tool_results[(si, tool_name)]
            if sqlite_ok and tool_ok:
                row.append(f"{G}PASS{N}")
                tallies[tool_name]["correct"] += 1
            elif sqlite_ok and not tool_ok:
                row.append(f"{R}FAIL{N}")
                tallies[tool_name]["reject_valid"] += 1
            elif not sqlite_ok and tool_ok:
                row.append(f"{Y}FP{N}")
                tallies[tool_name]["accept_invalid"] += 1
            else:
                row.append(f"{G}PASS{N}")
                tallies[tool_name]["correct"] += 1
        rows.append(row)
    md_table(headers, rows, align)

    # ── Scoreboard ──
    print(f"\n### Scoreboard\n")
    ranked = sorted(tallies.items(), key=lambda x: (-x[1]["correct"], x[1]["reject_valid"]))
    sb_headers = ["Tool", "Correct", "Rejects Valid", "Accepts Invalid"]
    sb_rows = []
    for name, t in ranked:
        pct = t["correct"] * 100 // total
        bar = "█" * (t["correct"] * 20 // total)
        color = G if t["correct"] == total else Y if pct >= 75 else R
        rv = str(t['reject_valid']) if t['reject_valid'] > 0 else "-"
        ai = str(t['accept_invalid']) if t['accept_invalid'] > 0 else "-"
        sb_rows.append([f"{color}{name}{N}", f"{t['correct']}/{total} ({pct}%) {bar}", rv, ai])
    md_table(sb_headers, sb_rows, ['l', 'l', 'r', 'r'])
    print()

    # ── Speed benchmarks ──
    print(f"## Parse Speed\n")

    parse_dir = os.path.join(DIR, "parser")
    bench_src = os.path.join(DIR, "bench_statements.sql")
    with open(bench_src) as f:
        bench_sql = f.read()

    small_path = os.path.join(parse_dir, "bench.sql")
    with open(small_path, "w") as f:
        f.write(bench_sql)

    large_path = os.path.join(parse_dir, "bench_30x.sql")
    with open(large_path, "w") as f:
        for _ in range(30):
            f.write(bench_sql)

    small_lines = len(bench_sql.splitlines())
    large_lines = small_lines * 30
    print(f"- `bench.sql`: {small_lines} lines, {len(bench_sql):,} bytes")
    print(f"- `bench_30x.sql`: {large_lines} lines, {len(bench_sql)*30:,} bytes\n")

    # Helper scripts for tools that need wrapping
    with open(os.path.join(parse_dir, "_parse_cst.js"), "w") as f:
        f.write(
            'const {parse}=require("sql-parser-cst");const fs=require("fs");\n'
            'parse(fs.readFileSync(process.argv[2],"utf8"),{dialect:"sqlite"});\n'
        )

    with open(os.path.join(parse_dir, "_parse_node.js"), "w") as f:
        f.write(
            'const {Parser}=require("node-sql-parser");const fs=require("fs");\n'
            'const p=new Parser();\n'
            'p.astify(fs.readFileSync(process.argv[2],"utf8"),{database:"SQLite"});\n'
        )

    with open(os.path.join(parse_dir, "_parse_sqlglot.py"), "w") as f:
        f.write(
            "import sqlglot,sys\n"
            "list(sqlglot.parse(open(sys.argv[1]).read(),dialect='sqlite'))\n"
        )

    for label, sqlfile in [
        ("bench.sql (1x)", small_path),
        ("bench_30x.sql (30x)", large_path),
    ]:
        print(f"### {label}\n")
        warmup = "3" if "1x" in label else "2"
        runs = "10" if "1x" in label else "5"
        cmd = (
            f"hyperfine --warmup {warmup} --min-runs {runs} --shell=none "
            f"--export-markdown '{parse_dir}/bench_{label.split()[0]}.md' "
            f"-n syntaqlite '{SYNQ} parse {sqlfile}' "
            f"-n lemon-rs '{LEMON_RS} {sqlfile}' "
            f"-n sql-parser-cst 'node {parse_dir}/_parse_cst.js {sqlfile}' "
            f"-n 'sqlglot[c]' '{UV} run --directory {DIR} python {parse_dir}/_parse_sqlglot.py {sqlfile}' "
            f"-n sqlparser-rs '{SQLPARSER_RS} {sqlfile}' "
            f"-n node-sql-parser 'node {parse_dir}/_parse_node.js {sqlfile}' "
            f"-n sqlfluff '{UV} run --directory {DIR} sqlfluff parse {sqlfile} --dialect sqlite' "
        )
        subprocess.run(cmd, shell=True, cwd=DIR)
        bench_md = os.path.join(parse_dir, f"bench_{label.split()[0]}.md")
        if os.path.exists(bench_md):
            print(open(bench_md).read())
        print()

    return tallies


# ─── FORMATTER CATEGORY ──────────────────────────────────────────────

def run_formatter_comparison():
    print(f"\n# Formatter Comparison\n")
    print(f"Round-trip correctness (format then validate with sqlite3) and speed.\n")

    # Ensure base schema DB exists
    create_test_db()

    tests = load_test_statements()
    stmt_dir = os.path.join(DIR, "formatter", "stmts")
    stmt_files = write_per_stmt_files(tests, stmt_dir)
    total = len(stmt_files)

    # ── Ground truth ──
    print(f"## Ground Truth\n")
    orig_pass = {}
    gt_rows = []
    for name, path in stmt_files:
        with open(path) as f:
            sql = f.read()
        ok, err = validate_sql_with_sqlite(sql)
        orig_pass[path] = ok
        status = "OK" if ok else f"SKIP ({err[:40]})"
        gt_rows.append([name[:50], status])
    md_table(["Statement", "sqlite3"], gt_rows)

    n_valid = sum(orig_pass.values())
    print(f"\n**{n_valid}/{total}** statements validated by sqlite3.\n")

    # ── Round-trip validation ──
    print(f"## Round-Trip Validation\n")
    print(f"For each formatter: does the formatted output still pass real SQLite?\n")

    tallies = {tn: {"format_ok": 0, "sqlite_ok": 0, "corrupted": 0} for tn in FORMATTER_NAMES}
    corruption_details = []
    headers = ["Test"] + FORMATTER_NAMES
    align = ['l'] + ['c'] * len(FORMATTER_NAMES)

    # Run all formatter×statement combinations in parallel
    fmt_results = {}  # (si, tool_name) -> (fmt_ok, formatted_sql)
    with ThreadPoolExecutor(max_workers=os.cpu_count() or 8) as pool:
        futures = {}
        for si, (name, path) in enumerate(stmt_files):
            for tn in FORMATTER_NAMES:
                fut = pool.submit(get_formatted_sql, tn, path)
                futures[fut] = (si, tn)
        for fut in as_completed(futures):
            si, tn = futures[fut]
            fmt_results[(si, tn)] = fut.result()

    # Now validate formatted output (also in parallel)
    validate_jobs = {}  # (si, tool_name) -> formatted_sql
    for si, (name, path) in enumerate(stmt_files):
        for tn in FORMATTER_NAMES:
            fmt_ok, formatted_sql = fmt_results[(si, tn)]
            if fmt_ok and formatted_sql and formatted_sql.strip() and orig_pass[path]:
                validate_jobs[(si, tn)] = formatted_sql

    validate_results = {}  # (si, tool_name) -> (sqlite_ok, sqlite_err)
    with ThreadPoolExecutor(max_workers=os.cpu_count() or 8) as pool:
        futures = {}
        for key, sql in validate_jobs.items():
            fut = pool.submit(validate_sql_with_sqlite, sql)
            futures[fut] = key
        for fut in as_completed(futures):
            validate_results[futures[fut]] = fut.result()

    rows = []
    for si, (name, path) in enumerate(stmt_files):
        row = [name[:38]]
        for tn in FORMATTER_NAMES:
            fmt_ok, formatted_sql = fmt_results[(si, tn)]
            if not fmt_ok or not formatted_sql or not formatted_sql.strip():
                row.append(f"{R}FAIL{N}")
                continue
            tallies[tn]["format_ok"] += 1
            if not orig_pass[path]:
                row.append("skip")
                continue
            sqlite_ok, sqlite_err = validate_results.get((si, tn), (False, "not run"))
            if sqlite_ok:
                tallies[tn]["sqlite_ok"] += 1
                row.append(f"{G}OK{N}")
            else:
                tallies[tn]["corrupted"] += 1
                corruption_details.append((tn, name, sqlite_err[:60]))
                row.append(f"{R}CORRUPT{N}")
        rows.append(row)
    md_table(headers, rows, align)

    # Scoreboard
    print(f"\n### Scoreboard\n")
    sb_rows = []
    for tn in FORMATTER_NAMES:
        t = tallies[tn]
        color = G if t["corrupted"] == 0 and t["format_ok"] == n_valid else Y if t["corrupted"] == 0 else R
        sb_rows.append([f"{color}{tn}{N}", f"{t['format_ok']}/{total}", f"{t['sqlite_ok']}/{n_valid}", str(t['corrupted'])])
    md_table(["Tool", "Formats", "SQLite OK", "Corrupt"], sb_rows, ['l', 'r', 'r', 'r'])

    if corruption_details:
        print(f"\n### Corruption Details\n")
        cd_rows = [[f"{R}{tn}{N}", tname, err] for tn, tname, err in corruption_details]
        md_table(["Tool", "Test", "Error"], cd_rows)
    print()

    # ── Speed benchmarks ──
    print(f"## Format Speed\n")

    bench_src = os.path.join(DIR, "bench_statements.sql")
    with open(bench_src) as f:
        bench_sql = f.read()

    fmt_bench_dir = os.path.join(DIR, "formatter")
    small_path = os.path.join(fmt_bench_dir, "bench.sql")
    with open(small_path, "w") as f:
        f.write(bench_sql)

    large_path = os.path.join(fmt_bench_dir, "bench_30x.sql")
    with open(large_path, "w") as f:
        for _ in range(30):
            f.write(bench_sql)

    small_lines = len(bench_sql.splitlines())
    large_lines = small_lines * 30
    print(f"- `bench.sql`: {small_lines} lines, {len(bench_sql):,} bytes")
    print(f"- `bench_30x.sql`: {large_lines} lines, {len(bench_sql)*30:,} bytes\n")

    # Write helper scripts
    fmt_dir = fmt_bench_dir
    helpers = {
        "_wrap_sleek.sh": 'cp "$1" "$1.bak"; sleek "$1" > /dev/null 2>&1; cp "$1.bak" "$1"; rm -f "$1.bak"',
        "_wrap_sqruff.sh": 'cp "$1" "$1.bak"; sqruff fix "$1" --dialect sqlite > /dev/null 2>&1; cp "$1.bak" "$1"; rm -f "$1.bak"',
    }
    for fname, cmd in helpers.items():
        p = os.path.join(fmt_dir, fname)
        with open(p, "w") as f:
            f.write(f"#!/bin/bash\n{cmd}\n")
        os.chmod(p, 0o755)

    with open(os.path.join(fmt_dir, "_fmt_sqlglot.py"), "w") as f:
        f.write(
            "import sqlglot,sys\n"
            "sql=open(sys.argv[1]).read()\n"
            "for e in sqlglot.parse(sql,dialect='sqlite'):\n"
            "  if e is not None: print(e.sql(dialect='sqlite',pretty=True))\n"
        )

    with open(os.path.join(fmt_dir, "_wrap_prettier.sh"), "w") as f:
        f.write("#!/bin/bash\nnpx prettier --stdin-filepath test.sql < \"$1\"\n")
    os.chmod(os.path.join(fmt_dir, "_wrap_prettier.sh"), 0o755)

    for label, sqlfile in [
        ("bench.sql (1x)", small_path),
        ("bench_30x.sql (30x)", large_path),
    ]:
        print(f"### {label}\n")
        warmup = "3" if "1x" in label else "2"
        runs = "10" if "1x" in label else "5"
        cmd = (
            f"hyperfine --warmup {warmup} --min-runs {runs} --shell=none "
            f"--export-markdown '{fmt_bench_dir}/bench_{label.split()[0]}.md' "
            f"-n syntaqlite '{SYNQ} fmt {sqlfile}' "
            f"-n prettier-cst '{fmt_dir}/_wrap_prettier.sh {sqlfile}' "
            f"-n sql-formatter 'sql-formatter -l sqlite {sqlfile}' "
            f"-n 'sqlglot[c]' '{UV} run --directory {DIR} python {fmt_dir}/_fmt_sqlglot.py {sqlfile}' "
            f"-n sleek '{fmt_dir}/_wrap_sleek.sh {sqlfile}' "
            f"-n sqruff '{fmt_dir}/_wrap_sqruff.sh {sqlfile}' "
        )
        subprocess.run(cmd, shell=True, cwd=DIR)
        bench_md = os.path.join(fmt_bench_dir, f"bench_{label.split()[0]}.md")
        if os.path.exists(bench_md):
            print(open(bench_md).read())
        print()

    # Slow tools
    print(f"### Slow Tools (single timed run)\n")
    slow_rows = []
    for name, fmt_cmd in [
        ("sqlfmt (1x)", f"cd '{DIR}' && cp '{small_path}' /tmp/_sqlfmt.sql && {UV} run sqlfmt /tmp/_sqlfmt.sql > /dev/null 2>&1; rm -f /tmp/_sqlfmt.sql"),
        ("sqlfmt (30x)", f"cd '{DIR}' && cp '{large_path}' /tmp/_sqlfmt.sql && {UV} run sqlfmt /tmp/_sqlfmt.sql > /dev/null 2>&1; rm -f /tmp/_sqlfmt.sql"),
        ("sqlfluff (1x)", f"cd '{DIR}' && cp '{small_path}' /tmp/_sqlfluff.sql && {UV} run sqlfluff format /tmp/_sqlfluff.sql --dialect sqlite -q > /dev/null 2>&1; rm -f /tmp/_sqlfluff.sql"),
    ]:
        start = time.monotonic()
        subprocess.run(fmt_cmd, shell=True, capture_output=True, timeout=120)
        elapsed = time.monotonic() - start
        slow_rows.append([name, f"{elapsed*1000:.0f}ms"])
    md_table(["Tool", "Time"], slow_rows, ['l', 'r'])
    print()


# ─── VALIDATOR CATEGORY ──────────────────────────────────────────────

# Schema for validator tests — shared between syntaqlite (DDL preamble) and sqlite3 (test DB).
# Uses the same comprehensive schema as TEST_SCHEMA_SQL to cover both test and bench statements.
VALIDATOR_SCHEMA_SQL = TEST_SCHEMA_SQL

# Validator tools: each returns True if an error/warning was detected.
def validate_syntaqlite(sql, schema_preamble):
    """Static semantic analysis — prepend schema DDL so it knows the tables."""
    t = _tmpfile(schema_preamble + sql + "\n")
    ok, out, err = run(f"'{SYNQ}' validate '{t}'")
    os.unlink(t)
    combined = out + err
    return "warning:" in combined or "error:" in combined


def validate_sqlite3_runtime(sql):
    """Runtime execution against real sqlite3 with schema."""
    ok, _ = validate_sql_with_sqlite(sql)
    return not ok


def validate_sql_lint(sql):
    """Structural checks (missing WHERE, unmatched parens)."""
    t = _tmpfile(sql + "\n")
    ok, out, err = run(f"npx sql-lint '{t}'")
    os.unlink(t)
    combined = out + err
    return "sql-lint" in combined.lower() or "unable to lint" in combined.lower()


LSP_VALIDATE = os.path.join(DIR, "validator", "_lsp_validate.js")
SQLITE_RUNNER_LSP = os.path.join(DIR, "_sqlite_runner_lsp", "server", "server.js")


def validate_sqlite_runner_lsp(sql, schema_preamble=""):
    """Runtime execution via LSP (wraps sqlite3 under the hood)."""
    t = _tmpfile(schema_preamble + sql + "\n")
    ok, out, err = run(f"node '{LSP_VALIDATE}' node '{SQLITE_RUNNER_LSP}' --stdio -- '{t}'")
    os.unlink(t)
    combined = out + err
    return "error" in combined.lower() and "no diagnostics" not in combined.lower()


VALIDATOR_TOOLS = [
    ("syntaqlite", "static semantic", validate_syntaqlite),
    ("sqlite3", "runtime execution", validate_sqlite3_runtime),
    ("sqlite-runner-lsp", "runtime via LSP", validate_sqlite_runner_lsp),
    ("sql-lint", "structural checks", validate_sql_lint),
]


def run_validator_comparison():
    print(f"\n# Validator Comparison\n")
    print(f"Error detection accuracy and diagnostic quality.\n")

    # Ensure base schema DB exists for sqlite3 runtime validation
    create_test_db()
    schema_preamble = VALIDATOR_SCHEMA_SQL.strip() + "\n"

    # ── Diagnostic quality — side-by-side on a realistic query ──
    print(f"## Diagnostic Quality\n")
    print(f"A realistic query with subtle errors — how does each tool report them?\n")

    demo_query = (
        "WITH\n"
        "  monthly_stats(month, revenue, order_count) AS (\n"
        "    SELECT\n"
        "      STRFTIME('%Y-%m', o.created_at) AS month,\n"
        "      SUM(o.total) AS revenue\n"
        "    FROM orders o\n"
        "    WHERE o.status = 'completed'\n"
        "    GROUP BY STRFTIME('%Y-%m', o.created_at)\n"
        "  )\n"
        "SELECT\n"
        "  ms.month,\n"
        "  ms.revenue,\n"
        "  ms.order_count,\n"
        "  ROUDN(ms.revenue / ms.order_count, 2) AS avg_order\n"
        "FROM monthly_stats ms\n"
        "ORDER BY ms.month DESC\n"
        "LIMIT 12;\n"
    )
    print(f"**Query** (2 errors: CTE declares 3 columns but SELECT produces 2; typo `ROUDN`):\n")
    print("```sql")
    print(demo_query.strip())
    print("```\n")

    # syntaqlite
    t = _tmpfile(schema_preamble + demo_query)
    _, out, err = run(f"'{SYNQ}' validate '{t}'")
    os.unlink(t)
    print(f"### syntaqlite\n")
    print(f"Static semantic analysis — offline, no database needed. Finds **both** errors in one pass:\n")
    print("```")
    for line in (out + err).strip().split('\n'):
        print(line)
    print("```\n")

    # sqlite3
    _, sqlite_err = validate_sql_with_sqlite(demo_query.rstrip().rstrip(';'))
    print(f"### sqlite3\n")
    print(f"Runtime execution — stops at first error:\n")
    print("```")
    if sqlite_err:
        print(sqlite_err.strip())
    print("```\n")

    # sqlite-runner-lsp
    t = _tmpfile(schema_preamble + demo_query)
    _, out, err = run(f"node '{LSP_VALIDATE}' node '{SQLITE_RUNNER_LSP}' --stdio -- '{t}'")
    os.unlink(t)
    print(f"### sqlite-runner-lsp\n")
    print(f"Runtime via LSP — wraps sqlite3, same single error:\n")
    combined = (out + err).strip()
    print("```")
    if combined and "no diagnostics" not in combined:
        print(combined)
    else:
        print("(no diagnostics)")
    print("```\n")

    # sql-lint
    t = _tmpfile(demo_query)
    _, out, err = run(f"npx sql-lint '{t}'")
    os.unlink(t)
    print(f"### sql-lint\n")
    print(f"Structural checks only:\n")
    combined = (out + err).strip()
    print("```")
    if combined and ("sql-lint" in combined.lower() or "error" in combined.lower()):
        short = combined.split('\n')[0]
        if len(short) > 120:
            short = short[:120] + "..."
        print(short)
    else:
        print("(no diagnostics)")
    print("```\n")

    # ── Correctness — error detection on edge cases ──
    print(f"## Error Detection Accuracy\n")
    print(f"Schema: `users`, `orders`, `products`, `order_items`. Ground truth: sqlite3.\n")

    # (sql, description, expected)
    #   expected: "error" = should be flagged, "valid" = should pass clean
    # Only cases where sqlite3 actually reports an error, plus valid statements.
    cases = [
        # Syntax errors
        ("SELEC * FROM users;", "keyword typo (SELEC)", "error"),
        ("SELECT * FROM users WHERE id IN (1, 2, 3;", "missing close paren", "error"),
        ("SELECT id,, name FROM users;", "double comma", "error"),
        ("SELECT * FROM users WHERE name = 'hello;", "unterminated string", "error"),
        ("INSERT INTO users VALUES (1, ;", "trailing comma in VALUES", "error"),
        # Semantic: unknown table
        ("SELECT * FROM nonexistent;", "unknown table", "error"),
        ("SELECT u.name FROM users u JOIN fake f ON f.id = u.id;", "unknown table in JOIN", "error"),
        # Semantic: unknown column
        ("SELECT bogus FROM users;", "unknown column", "error"),
        ("SELECT u.nonexistent FROM users u;", "unknown qualified column", "error"),
        ("SELECT name, fake_col FROM users WHERE active = 1;", "unknown column in SELECT", "error"),
        # Semantic: wrong function arity
        ("SELECT SUBSTR(name) FROM users;", "SUBSTR: too few args", "error"),
        ("SELECT REPLACE(name, 'a') FROM users;", "REPLACE: too few args", "error"),
        ("SELECT LENGTH(name, email) FROM users;", "LENGTH: too many args", "error"),
        ("SELECT COALESCE() FROM users;", "COALESCE: zero args", "error"),
        # Semantic: CTE column count
        ("WITH cte(a, b, c) AS (SELECT 1, 2) SELECT * FROM cte;", "CTE: 3 declared, 2 actual", "error"),
        # Valid statements (must NOT be flagged)
        ("SELECT id, name FROM users WHERE active = 1;", "valid: simple SELECT", "valid"),
        ("SELECT u.name, COUNT(o.id) FROM users u JOIN orders o ON o.customer_id = u.id GROUP BY u.name;", "valid: JOIN + aggregate", "valid"),
        ("SELECT SUBSTR(name, 1, 3) FROM users;", "valid: SUBSTR with 3 args", "valid"),
        ("SELECT COALESCE(email, name, 'anon') FROM users;", "valid: COALESCE variadic", "valid"),
        ("WITH cte(a, b) AS (SELECT 1, 2) SELECT * FROM cte;", "valid: CTE columns match", "valid"),
        ("SELECT LENGTH(name), UPPER(email), LOWER(name) FROM users;", "valid: built-in functions", "valid"),
        ("INSERT INTO users (name, email) VALUES ('x', 'y');", "valid: INSERT", "valid"),
        ("UPDATE orders SET status = 'shipped' WHERE id = 1;", "valid: UPDATE", "valid"),
        ("DELETE FROM users WHERE id = 1;", "valid: DELETE with WHERE", "valid"),
    ]

    tallies = {name: {"correct": 0, "fp": 0, "fn": 0} for name, _, _ in VALIDATOR_TOOLS}
    total = len(cases)
    headers = ["Test", "Expect"] + [n for n, _, _ in VALIDATOR_TOOLS]
    align = ['l', 'c'] + ['c'] * len(VALIDATOR_TOOLS)

    # Run all tool×case combinations in parallel
    def _run_validator(tool_name, tool_fn, sql):
        if tool_name in ("syntaqlite", "sqlite-runner-lsp"):
            return tool_fn(sql, schema_preamble)
        return tool_fn(sql)

    val_results = {}  # (ci, tool_name) -> bool
    with ThreadPoolExecutor(max_workers=os.cpu_count() or 8) as pool:
        futures = {}
        for ci, (sql, desc, expected) in enumerate(cases):
            for tool_name, _, tool_fn in VALIDATOR_TOOLS:
                fut = pool.submit(_run_validator, tool_name, tool_fn, sql)
                futures[fut] = (ci, tool_name)
        for fut in as_completed(futures):
            ci, tool_name = futures[fut]
            val_results[(ci, tool_name)] = fut.result()

    rows = []
    for ci, (sql, desc, expected) in enumerate(cases):
        row = [desc[:38], expected]
        for tool_name, _, _ in VALIDATOR_TOOLS:
            detected = val_results[(ci, tool_name)]
            if expected == "error":
                if detected:
                    row.append(f"{G}FOUND{N}")
                    tallies[tool_name]["correct"] += 1
                else:
                    row.append(f"{R}MISS{N}")
                    tallies[tool_name]["fn"] += 1
            else:
                if detected:
                    row.append(f"{R}FP{N}")
                    tallies[tool_name]["fp"] += 1
                else:
                    row.append(f"{G}OK{N}")
                    tallies[tool_name]["correct"] += 1
        rows.append(row)
    md_table(headers, rows, align)

    # Scoreboard
    print(f"\n### Scoreboard\n")
    ranked = sorted(tallies.items(), key=lambda x: (-x[1]["correct"], x[1]["fn"]))
    sb_rows = []
    for name, t in ranked:
        approach = next(a for n, a, _ in VALIDATOR_TOOLS if n == name)
        pct = t["correct"] * 100 // total
        color = G if t["correct"] == total else Y if pct >= 75 else R
        bar = "█" * (t["correct"] * 20 // total)
        fn_str = str(t["fn"]) if t["fn"] > 0 else "-"
        fp_str = str(t["fp"]) if t["fp"] > 0 else "-"
        sb_rows.append([f"{color}{name}{N}", approach, f"{t['correct']}/{total} {bar}", fn_str, fp_str])
    md_table(["Tool", "Approach", "Correct", "Missed", "FP"], sb_rows, ['l', 'l', 'l', 'r', 'r'])
    print()

    # ── Speed benchmarks ──
    print(f"## Validation Speed\n")

    val_dir = os.path.join(DIR, "validator")
    os.makedirs(val_dir, exist_ok=True)

    bench_src = os.path.join(DIR, "bench_statements.sql")
    with open(bench_src) as f:
        bench_sql = f.read()

    # All tools get schema prepended — bench_statements.sql references tables
    # that must exist for sqlite3 runtime execution to succeed.
    schema = VALIDATOR_SCHEMA_SQL

    small_path = os.path.join(val_dir, "bench.sql")
    with open(small_path, "w") as f:
        f.write(schema + bench_sql)

    # For 30x: include DDL once (first copy), then repeat only DML/SELECT stmts.
    # sqlite3 can't re-execute CREATE TABLE / ALTER TABLE 30 times.
    # Split by blank lines to get statement blocks, filter out DDL blocks.
    ddl_prefixes = ('CREATE ', 'DROP ', 'ALTER ')
    blocks = re.split(r'\n\n+', bench_sql.strip())
    dml_blocks = []
    for block in blocks:
        # Get first non-comment line to check if it's DDL
        sql_lines = [l for l in block.strip().split('\n') if not l.strip().startswith('--')]
        first_sql = ' '.join(sql_lines).strip().upper()
        if not any(first_sql.startswith(p) for p in ddl_prefixes):
            dml_blocks.append(block)
    bench_dml_only = '\n\n'.join(dml_blocks) + '\n'

    large_path = os.path.join(val_dir, "bench_30x.sql")
    with open(large_path, "w") as f:
        f.write(schema + bench_sql)  # first copy with DDL
        for _ in range(29):
            f.write(bench_dml_only)

    small_lines = len(bench_sql.splitlines())
    large_lines = small_lines * 30
    print(f"- `bench.sql`: {small_lines} lines, {len(bench_sql):,} bytes (+ schema preamble)")
    print(f"- `bench_30x.sql`: {large_lines} lines, {len(bench_sql)*30:,} bytes (+ schema preamble)\n")

    # sql-lint wrapper
    with open(os.path.join(val_dir, "_wrap_sql_lint.sh"), "w") as f:
        f.write("#!/bin/bash\nnpx sql-lint \"$1\" > /dev/null 2>&1\n")
    os.chmod(os.path.join(val_dir, "_wrap_sql_lint.sh"), 0o755)

    # sqlite3: run all statements via .read
    with open(os.path.join(val_dir, "_wrap_sqlite3.sh"), "w") as f:
        f.write("#!/bin/bash\nsqlite3 ':memory:' < \"$1\" > /dev/null 2>&1\n")
    os.chmod(os.path.join(val_dir, "_wrap_sqlite3.sh"), 0o755)

    # sqlite-runner-lsp: LSP wrapper around sqlite3
    with open(os.path.join(val_dir, "_wrap_sqlite_runner_lsp.sh"), "w") as f:
        f.write(f"#!/bin/bash\nnode '{LSP_VALIDATE}' node '{SQLITE_RUNNER_LSP}' --stdio -- \"$1\" > /dev/null 2>&1\n")
    os.chmod(os.path.join(val_dir, "_wrap_sqlite_runner_lsp.sh"), 0o755)

    for label, sqlfile in [
        ("bench.sql (1x)", small_path),
        ("bench_30x.sql (30x)", large_path),
    ]:
        print(f"### {label}\n")
        warmup = "3" if "1x" in label else "2"
        runs = "10" if "1x" in label else "5"
        cmd = (
            f"hyperfine --warmup {warmup} --min-runs {runs} --shell=none "
            f"--export-markdown '{val_dir}/bench_{label.split()[0]}.md' "
            f"-n syntaqlite '{SYNQ} validate {sqlfile}' "
            f"-n sqlite3 '{val_dir}/_wrap_sqlite3.sh {sqlfile}' "
            f"-n sqlite-runner-lsp '{val_dir}/_wrap_sqlite_runner_lsp.sh {sqlfile}' "
            f"-n sql-lint '{val_dir}/_wrap_sql_lint.sh {sqlfile}' "
        )
        subprocess.run(cmd, shell=True, cwd=DIR)
        bench_md = os.path.join(val_dir, f"bench_{label.split()[0]}.md")
        if os.path.exists(bench_md):
            print(open(bench_md).read())
        print()


# ─── LSP CATEGORY ────────────────────────────────────────────────────

SQLS = os.path.join(os.path.expanduser("~"), ".local", "share", "mise", "installs", "go", "1.26.1", "bin", "sqls")
LSP_TEST = os.path.join(DIR, "lsp", "_lsp_test.js")

def run_lsp_comparison():
    print(f"\n# LSP Comparison\n")
    print(f"Feature testing for SQLite-aware language servers.\n")

    lsp_dir = os.path.join(DIR, "lsp")
    os.makedirs(lsp_dir, exist_ok=True)

    # Create test files
    test_sql = os.path.join(lsp_dir, "test.sql")
    with open(test_sql, "w") as f:
        f.write("SELECT id, name, email FROM users WHERE active = 1;\n")

    err_sql = os.path.join(lsp_dir, "test_error.sql")
    with open(err_sql, "w") as f:
        f.write("SELEC * FROM users;\n")

    # Create test DB with schema
    test_db = os.path.join(lsp_dir, "test.db")
    if os.path.exists(test_db):
        os.unlink(test_db)
    subprocess.run(["sqlite3", test_db,
        "CREATE TABLE users(id INTEGER PRIMARY KEY, name TEXT, email TEXT, active INT);"
        "CREATE TABLE orders(id INTEGER PRIMARY KEY, customer_id INT, total REAL);"],
        capture_output=True)

    # sqls config file
    sqls_config = os.path.join(lsp_dir, "sqls_config.yml")
    with open(sqls_config, "w") as f:
        f.write(f"lowercaseKeywords: false\nconnections:\n  - alias: test\n    driver: sqlite3\n    dataSourceName: {test_db}\n")

    # LSP servers to test
    lsp_servers = [
        ("syntaqlite", [SYNQ, "lsp"], []),
        ("sqls", [SQLS, "-c", sqls_config], []),
        ("sql-language-server", ["npx", "sql-language-server", "up", "--method", "stdio"], []),
    ]

    # ── Capability & Feature Testing ──
    print(f"## Tested Capabilities\n")
    print(f"Each server is started, sent a test file, and probed for completion, hover,\n"
          f"diagnostics, and formatting. Results are from actual LSP responses.\n")

    import json

    def _test_lsp_server(name, cmd, test_sql, err_sql):
        full_cmd = f"node '{LSP_TEST}' {' '.join(cmd)} -- '{test_sql}'"
        ok, out, err = run(full_cmd, timeout=15)
        try:
            data = json.loads(out) if ok and out.strip() else {}
        except json.JSONDecodeError:
            data = {}
        err_cmd = f"node '{LSP_TEST}' {' '.join(cmd)} -- '{err_sql}'"
        ok2, out2, _ = run(err_cmd, timeout=15)
        try:
            err_data = json.loads(out2) if ok2 and out2.strip() else {}
        except json.JSONDecodeError:
            err_data = {}
        return name, {"valid": data, "error": err_data}

    # Test all LSP servers in parallel
    all_results = {}
    with ThreadPoolExecutor(max_workers=len(lsp_servers)) as pool:
        futures = [pool.submit(_test_lsp_server, name, cmd, test_sql, err_sql) for name, cmd, _ in lsp_servers]
        for fut in as_completed(futures):
            name, result = fut.result()
            all_results[name] = result

    # Build feature matrix from real results
    tool_names = [n for n, _, _ in lsp_servers]
    feature_rows = []

    def cap(name, field):
        caps = all_results.get(name, {}).get("valid", {}).get("capabilities", {})
        return caps.get(field, False)

    def val(b, label="Yes", neg="No"):
        if b:
            return f"{G}{label}{N}"
        return f"{R}{neg}{N}"

    # Completion
    row = ["Completion"]
    for name in tool_names:
        c = all_results.get(name, {}).get("valid", {}).get("completion", {})
        if cap(name, "completion") and c.get("count", 0) > 0:
            row.append(f"{G}Yes ({c['count']} items){N}")
        elif cap(name, "completion"):
            row.append(f"{Y}Advertised (0 items){N}")
        else:
            row.append(f"{R}No{N}")
    feature_rows.append(row)

    # Hover
    row = ["Hover"]
    for name in tool_names:
        h = all_results.get(name, {}).get("valid", {}).get("hover", {})
        row.append(val(cap(name, "hover") and h.get("supported")))
    feature_rows.append(row)

    # Go to definition
    row = ["Go to definition"]
    for name in tool_names:
        row.append(val(cap(name, "definition")))
    feature_rows.append(row)

    # References
    row = ["Find references"]
    for name in tool_names:
        row.append(val(cap(name, "references")))
    feature_rows.append(row)

    # Diagnostics: syntax errors
    row = ["Diagnostics: syntax"]
    for name in tool_names:
        diag = all_results.get(name, {}).get("error", {}).get("diagnostics")
        if diag and len(diag) > 0:
            # Check if any diagnostic mentions actual syntax error (not style lint)
            has_syntax = any("syntax" in d.get("message", "").lower() or
                            "near" in d.get("message", "").lower() or
                            "parse" in d.get("message", "").lower()
                            for d in diag)
            has_style = any("linebreak" in d.get("message", "").lower() or
                           "new line" in d.get("message", "").lower() or
                           "column" in d.get("message", "").lower()
                           for d in diag)
            if has_syntax:
                row.append(f"{G}Yes{N}")
            elif has_style:
                row.append(f"{Y}Style lint only{N}")
            else:
                row.append(f"{G}Yes{N}")
        else:
            row.append(f"{R}No{N}")
    feature_rows.append(row)

    # Diagnostics: semantic errors
    row = ["Diagnostics: semantic"]
    for name in tool_names:
        diag = all_results.get(name, {}).get("valid", {}).get("diagnostics")
        if diag and len(diag) > 0:
            has_semantic = any("unknown" in d.get("message", "").lower() or
                              "table" in d.get("message", "").lower()
                              for d in diag)
            has_style = all("linebreak" in d.get("message", "").lower() or
                           "new line" in d.get("message", "").lower() or
                           "column" in d.get("message", "").lower()
                           for d in diag)
            if has_semantic:
                row.append(f"{G}Yes{N}")
            elif has_style:
                row.append(f"{R}No (style only){N}")
            else:
                row.append(f"{Y}Partial{N}")
        else:
            row.append(f"{R}No{N}")
    feature_rows.append(row)

    # Formatting
    row = ["Formatting"]
    for name in tool_names:
        f_data = all_results.get(name, {}).get("valid", {}).get("formatting", {})
        row.append(val(cap(name, "formatting") and f_data.get("supported")))
    feature_rows.append(row)

    # Rename
    row = ["Rename"]
    for name in tool_names:
        row.append(val(cap(name, "rename")))
    feature_rows.append(row)

    # Signature help
    row = ["Signature help"]
    for name in tool_names:
        row.append(val(cap(name, "signatureHelp")))
    feature_rows.append(row)

    # Requires DB
    row = ["Requires DB connection"]
    db_req = {"syntaqlite": False, "sqls": True, "sql-language-server": False}
    for name in tool_names:
        needs = db_req.get(name, False)
        row.append(f"{G}No{N}" if not needs else f"{Y}Yes{N}")
    feature_rows.append(row)

    md_table(["Feature"] + tool_names, feature_rows, ['l'] + ['c'] * len(tool_names))

    # ── Diagnostic detail ──
    print(f"\n## Diagnostic Detail\n")
    print(f"What each server reports for `SELEC * FROM users;` (syntax error):\n")

    for name in tool_names:
        diag = all_results.get(name, {}).get("error", {}).get("diagnostics")
        print(f"### {name}\n")
        if diag and len(diag) > 0:
            print("```")
            for d in diag:
                print(f"{d['line']}:{d['col']} {d['severity']} {d['message']}")
            print("```\n")
        else:
            print("```")
            print("(no diagnostics)")
            print("```\n")

    # ── Speed benchmarks ──
    print(f"## LSP Startup + Response Speed\n")

    bench_sql = os.path.join(lsp_dir, "bench.sql")
    with open(bench_sql, "w") as f:
        f.write(TEST_SCHEMA_SQL)
        with open(os.path.join(DIR, "bench_statements.sql")) as bs:
            f.write(bs.read())

    # Wrapper scripts for each LSP: start, send file, get diagnostics, exit
    for name, cmd, _ in lsp_servers:
        script = os.path.join(lsp_dir, f"_wrap_{name.replace('-', '_')}.sh")
        cmd_str = ' '.join(cmd)
        if name == "sqls":
            cmd_str += f" -c '{sqls_config}'"
        with open(script, "w") as f:
            f.write(f"#!/bin/bash\nnode '{LSP_VALIDATE}' {cmd_str} -- \"$1\" > /dev/null 2>&1\n")
        os.chmod(script, 0o755)

    print(f"Time to start server, send document, receive diagnostics, and exit:\n")

    # Use the existing LSP validate script for benchmarks (simpler, just measures diagnostics round-trip)
    bench_cmds = []
    for name, cmd, _ in lsp_servers:
        script = os.path.join(lsp_dir, f"_wrap_{name.replace('-', '_')}.sh")
        bench_cmds.append(f"-n '{name}' '{script} {bench_sql}'")

    cmd = (
        f"hyperfine --warmup 2 --min-runs 5 --shell=none -i "
        f"--export-markdown '{lsp_dir}/bench.md' "
        + " ".join(bench_cmds)
    )
    subprocess.run(cmd, shell=True, cwd=DIR)
    bench_md = os.path.join(lsp_dir, "bench.md")
    if os.path.exists(bench_md):
        print(open(bench_md).read())
    print()


# ─── ANSI stripping ─────────────────────────────────────────────────

ANSI_RE = re.compile(r'\033\[[0-9;]*m')

def strip_ansi(text):
    return ANSI_RE.sub('', text)


# ─── MAIN ────────────────────────────────────────────────────────────

if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="syntaqlite competitive comparison")
    parser.add_argument("categories", nargs="*", default=["parser", "formatter", "validator", "lsp"],
                        help="Categories to run (parser, formatter, validator, lsp)")
    parser.add_argument("--output", choices=["terminal", "markdown"], default="terminal",
                        help="Output format: terminal (colored) or markdown (plain, for docs)")
    parser.add_argument("--output-file", type=str, default=None,
                        help="Write output to file instead of stdout")
    args = parser.parse_args()

    # For markdown output, suppress ANSI colors and optionally redirect to file
    original_print = print
    output_lines = []

    if args.output == "markdown":
        # Disable ANSI codes globally
        globals().update(G="", R="", Y="", C="", N="", B="", DIM="")
        import builtins
        _real_print = builtins.print
        def capturing_print(*a, **kw):
            import io
            buf = io.StringIO()
            _real_print(*a, **kw, file=buf)
            text = strip_ansi(buf.getvalue())
            output_lines.append(text)
            _real_print(text, end='', file=sys.stderr)  # progress to stderr
        builtins.print = capturing_print

    print(f"# syntaqlite — Competitive Comparison\n")
    print(f"SQLite SQL tooling landscape.\n")

    if "parser" in args.categories:
        run_parser_comparison()
    if "formatter" in args.categories:
        run_formatter_comparison()
    if "validator" in args.categories:
        run_validator_comparison()
    if "lsp" in args.categories:
        run_lsp_comparison()

    if args.output == "markdown":
        import builtins
        builtins.print = _real_print
        full_output = ''.join(output_lines)
        if args.output_file:
            with open(args.output_file, 'w') as f:
                f.write(full_output)
            print(f"\nWritten to {args.output_file}", file=sys.stderr)
        else:
            sys.stdout.write(full_output)
