#!/usr/bin/env python3
"""
Check whether SQLite's parse.y grammar rules have been strictly additive.

Downloads parse.y from many SQLite versions via the GitHub mirror and extracts
all Lemon grammar production rules. Reports any rules that existed in version N
but were REMOVED in version N+1.
"""

import re
import subprocess
import sys
import tempfile
import json
from pathlib import Path
from collections import OrderedDict

VERSIONS = [
    "3.8.0",
    "3.9.0",
    "3.10.0",
    "3.11.0",
    "3.12.0",
    "3.13.0",
    "3.14.0",
    "3.15.0",
    "3.16.0",
    "3.17.0",
    "3.18.0",
    "3.19.0",
    "3.20.0",
    "3.21.0",
    "3.22.0",
    "3.23.0",
    "3.24.0",  # upsert
    "3.25.0",  # window functions
    "3.26.0",
    "3.27.0",
    "3.28.0",
    "3.29.0",
    "3.30.0",  # generated columns
    "3.31.0",
    "3.32.0",
    "3.33.0",
    "3.34.0",
    "3.35.0",  # RETURNING, MATERIALIZED
    "3.36.0",
    "3.37.0",
    "3.38.0",  # -> and ->> operators
    "3.39.0",
    "3.40.0",
    "3.41.0",
    "3.42.0",
    "3.43.0",
    "3.44.0",
    "3.45.0",
    "3.46.0",  # WITHIN keyword
    "3.47.0",
    "3.48.0",
    "3.49.0",
    "3.50.0",
    "3.51.0",
]

CACHE_DIR = Path(__file__).parent / ".parse-y-cache"


def download_parse_y(version: str) -> str | None:
    """Download parse.y for a given version from the GitHub mirror."""
    cache_file = CACHE_DIR / f"parse_y_{version}.txt"
    if cache_file.exists():
        return cache_file.read_text()

    tag = f"version-{version}"
    url = f"https://raw.githubusercontent.com/sqlite/sqlite/{tag}/src/parse.y"

    result = subprocess.run(
        ["curl", "-sL", "-f", url],
        capture_output=True,
        text=True,
        timeout=30,
    )
    if result.returncode != 0:
        print(f"  WARN: Failed to download parse.y for {version}", file=sys.stderr)
        return None

    CACHE_DIR.mkdir(parents=True, exist_ok=True)
    cache_file.write_text(result.stdout)
    return result.stdout


def extract_rules(parse_y_content: str) -> set[str]:
    """
    Extract all grammar production rules from a Lemon .y file.

    Returns a set of normalized rule signatures like:
        "cmd ::= BEGIN transtype trans_opt"
        "expr ::= expr AND expr"

    Normalization:
    - Strip aliases: expr(A) -> expr
    - Strip action blocks: everything in { }
    - Strip %ifdef/%endif blocks tracking (we extract rules regardless)
    - Collapse whitespace
    """
    rules = set()

    # Remove C action blocks. They can be nested (rare but possible).
    # We find the `. {` or `.  {` pattern that ends a rule and starts an action.
    # Strategy: process line by line, tracking brace depth.
    lines = parse_y_content.split("\n")
    cleaned_lines = []
    brace_depth = 0
    in_c_code_block = False  # top-level %{ %} blocks

    for line in lines:
        stripped = line.strip()

        # Handle %include { ... } and %{ ... %} blocks
        if stripped.startswith("%include"):
            in_c_code_block = True
            continue
        if in_c_code_block:
            if stripped == "}":
                in_c_code_block = False
            continue

        # Track brace depth for action blocks
        if brace_depth > 0:
            brace_depth += line.count("{") - line.count("}")
            continue

        # Check if this line starts an action block
        # Action blocks start after the `.` that ends the rule
        if ". {" in line or ".\t{" in line or ".{" in line:
            # Extract the part before the action
            idx = line.index(".")
            before_action = line[: idx + 1]
            cleaned_lines.append(before_action)
            # Count braces in the rest
            rest = line[idx + 1 :]
            brace_depth = rest.count("{") - rest.count("}")
            continue

        # Skip directives, comments, blank lines
        if (
            stripped.startswith("%")
            or stripped.startswith("//")
            or stripped.startswith("/*")
            or stripped.startswith("**")
            or stripped.startswith("*")
            or stripped == ""
            or stripped == "}"
        ):
            continue

        cleaned_lines.append(line)

    # Now parse rules from cleaned lines
    # Lemon rules look like: lhs(ALIAS) ::= rhs_symbol1(A) rhs_symbol2 .
    # They can span multiple lines
    full_text = " ".join(cleaned_lines)

    # Split on the rule separator pattern: ". " followed by a new rule or end
    # Actually, find all "LHS ::= RHS ." patterns
    rule_pattern = re.compile(
        r"(\w+)"  # LHS nonterminal
        r"(?:\([^)]*\))?"  # optional alias
        r"\s*::=\s*"  # separator
        r"(.*?)"  # RHS (non-greedy)
        r"\s*\."  # terminating dot
        r"(?=\s|$)"  # followed by space or end
    )

    for m in rule_pattern.finditer(full_text):
        lhs = m.group(1)
        rhs_raw = m.group(2).strip()

        # Normalize RHS: remove aliases like (A), (B), etc.
        rhs = re.sub(r"\([^)]*\)", "", rhs_raw)
        # Collapse whitespace
        rhs = re.sub(r"\s+", " ", rhs).strip()

        # Build normalized rule
        if rhs:
            rule = f"{lhs} ::= {rhs}"
        else:
            rule = f"{lhs} ::= (empty)"

        rules.add(rule)

    return rules


def main():
    print("=" * 70)
    print("SQLite parse.y Additivity Check")
    print("=" * 70)
    print()
    print(f"Checking {len(VERSIONS)} versions: {VERSIONS[0]} through {VERSIONS[-1]}")
    print()

    # Download and extract rules for each version
    version_rules: OrderedDict[str, set[str]] = OrderedDict()

    for ver in VERSIONS:
        sys.stdout.write(f"  Downloading {ver}...")
        sys.stdout.flush()
        content = download_parse_y(ver)
        if content is None:
            print(" FAILED")
            continue
        rules = extract_rules(content)
        version_rules[ver] = rules
        print(f" {len(rules)} rules extracted")

    print()

    # Compare consecutive versions
    versions_list = list(version_rules.keys())
    all_removals: list[tuple[str, str, set[str]]] = []
    all_additions: list[tuple[str, str, set[str]]] = []

    print("=" * 70)
    print("Version-to-version comparison")
    print("=" * 70)

    for i in range(len(versions_list) - 1):
        v_old = versions_list[i]
        v_new = versions_list[i + 1]
        old_rules = version_rules[v_old]
        new_rules = version_rules[v_new]

        added = new_rules - old_rules
        removed = old_rules - new_rules

        if added or removed:
            print(f"\n--- {v_old} -> {v_new} ---")
            if added:
                print(f"  ADDED ({len(added)}):")
                for r in sorted(added):
                    print(f"    + {r}")
                all_additions.append((v_old, v_new, added))
            if removed:
                print(f"  REMOVED ({len(removed)}):")
                for r in sorted(removed):
                    print(f"    - {r}")
                all_removals.append((v_old, v_new, removed))
        else:
            print(f"  {v_old} -> {v_new}: no changes")

    # Also check: rules that existed in any version but are missing from latest
    print()
    print("=" * 70)
    print("GLOBAL ANALYSIS")
    print("=" * 70)

    all_ever = set()
    for rules in version_rules.values():
        all_ever |= rules

    latest = versions_list[-1]
    latest_rules = version_rules[latest]

    disappeared = all_ever - latest_rules
    if disappeared:
        print(f"\nRules that existed in SOME version but are MISSING from {latest}:")
        for r in sorted(disappeared):
            # Find which versions had this rule
            present_in = [v for v, rules in version_rules.items() if r in rules]
            print(f"  - {r}")
            print(f"    present in: {present_in[0]} through {present_in[-1]}")
    else:
        print(f"\nAll rules ever seen are present in the latest version ({latest}).")
        print("The grammar is STRICTLY ADDITIVE.")

    print()
    print("=" * 70)
    print("SUMMARY")
    print("=" * 70)
    print(f"Versions checked: {len(version_rules)}")
    print(f"Total unique rules ever seen: {len(all_ever)}")
    print(f"Rules in latest ({latest}): {len(latest_rules)}")
    print(f"Total addition events: {sum(len(a[2]) for a in all_additions)}")
    print(f"Total removal events: {sum(len(r[2]) for r in all_removals)}")

    if all_removals:
        print()
        print("WARNING: Grammar is NOT strictly additive!")
        print("The following version transitions removed rules:")
        for v_old, v_new, removed in all_removals:
            print(f"  {v_old} -> {v_new}: {len(removed)} rules removed")
        return 1
    else:
        print()
        print("CONFIRMED: Grammar is strictly additive across all checked versions.")
        return 0


if __name__ == "__main__":
    sys.exit(main())
