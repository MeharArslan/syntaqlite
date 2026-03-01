#!/usr/bin/env python3
# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Check C header dependency boundaries between crates.

Each crate has:
  - csrc/   — private headers; must not be included by other crates
  - include/ — public headers, organised by namespace prefix

Rules enforced:
  Rule A — Private boundary: a #include "csrc/..." in crate X must
            resolve to a file that exists under <crate-X>/csrc/.
  Rule B — Namespace boundary: a #include "<ns>/..." is only allowed
            when <ns> is owned by crate X itself or a declared dependency.

Usage:
    python3 python/syntaqlite/checks/c_deps.py   # exits 0 on success
    tools/check-c-deps                            # thin wrapper
"""

import re
import sys
from dataclasses import dataclass, field
from pathlib import Path

ROOT = Path(__file__).parent.parent.parent.parent  # repo root

# ---------------------------------------------------------------------------
# Crate registry
# ---------------------------------------------------------------------------

@dataclass
class Crate:
    name: str
    # Public include-namespace prefixes this crate owns (e.g. "syntaqlite/").
    namespaces: list[str] = field(default_factory=list)
    # Names of crates this crate may use public headers from.
    deps: list[str] = field(default_factory=list)

    @property
    def root(self) -> Path:
        return ROOT / self.name


CRATES: list[Crate] = [
    Crate(
        name="syntaqlite-parser",
        namespaces=["syntaqlite/", "syntaqlite_dialect/"],
        deps=[],
    ),
    Crate(
        name="syntaqlite-parser-sqlite",
        namespaces=["syntaqlite_sqlite/"],
        deps=["syntaqlite-parser"],
    ),
]

_CRATE_BY_NAME: dict[str, Crate] = {c.name: c for c in CRATES}

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

INCLUDE_RE = re.compile(r'#\s*include\s+"([^"]+)"')

_ANSI_RED = "\033[31m"
_ANSI_GREEN = "\033[32m"
_ANSI_RESET = "\033[0m"


def _c_files(crate: Crate) -> list[Path]:
    files: list[Path] = []
    for subdir in ("csrc", "include"):
        d = crate.root / subdir
        if d.is_dir():
            files.extend(sorted(d.rglob("*.[ch]")))
    return files


def _allowed_namespaces(crate: Crate) -> set[str]:
    ns = set(crate.namespaces)
    for dep_name in crate.deps:
        dep = _CRATE_BY_NAME[dep_name]
        ns.update(dep.namespaces)
    return ns


def _forbidden_namespaces(crate: Crate) -> set[str]:
    allowed = _allowed_namespaces(crate)
    result: set[str] = set()
    for other in CRATES:
        if other.name == crate.name:
            continue
        for ns in other.namespaces:
            if ns not in allowed:
                result.add(ns)
    return result

# ---------------------------------------------------------------------------
# Core check
# ---------------------------------------------------------------------------

@dataclass
class Violation:
    file: Path
    lineno: int
    message: str

    def __str__(self) -> str:
        rel = self.file.relative_to(ROOT)
        return f"{rel}:{self.lineno}: {self.message}"


def check_crate(crate: Crate) -> list[Violation]:
    violations: list[Violation] = []
    forbidden_ns = _forbidden_namespaces(crate)

    for src_file in _c_files(crate):
        try:
            lines = src_file.read_text(encoding="utf-8", errors="replace").splitlines()
        except OSError as e:
            violations.append(Violation(src_file, 0, f"could not read file: {e}"))
            continue

        for lineno, line in enumerate(lines, 1):
            m = INCLUDE_RE.search(line)
            if not m:
                continue
            include_path = m.group(1)

            # Rule A: csrc/ includes must resolve within this crate.
            if include_path.startswith("csrc/"):
                resolved = crate.root / include_path
                if not resolved.exists():
                    violations.append(Violation(
                        src_file, lineno,
                        f"private include not found in own crate: \"{include_path}\""
                    ))

            # Rule B: namespace includes must come from allowed crates.
            for ns in forbidden_ns:
                if include_path.startswith(ns):
                    violations.append(Violation(
                        src_file, lineno,
                        f"forbidden include from undeclared dependency "
                        f"(namespace \"{ns}\"): \"{include_path}\""
                    ))
                    break

    return violations


def check_all() -> list[Violation]:
    all_violations: list[Violation] = []
    for crate in CRATES:
        if not crate.root.is_dir():
            print(
                f"warning: crate directory not found, skipping: {crate.name}",
                file=sys.stderr,
            )
            continue
        all_violations.extend(check_crate(crate))
    return all_violations

# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------

def main() -> int:
    violations = check_all()
    if violations:
        print(f"{_ANSI_RED}C dependency boundary violations:{_ANSI_RESET}", file=sys.stderr)
        for v in violations:
            print(f"  {v}", file=sys.stderr)
        return 1
    print(f"{_ANSI_GREEN}C dependency boundaries OK{_ANSI_RESET}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
