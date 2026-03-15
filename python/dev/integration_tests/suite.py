# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Suite protocol and context for integration tests."""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path
from typing import Protocol


@dataclass
class SuiteContext:
    """Common arguments forwarded to every suite."""
    root_dir: Path
    binary: Path
    verbose: int        # -1 = quiet, 0 = normal, 1 = verbose
    filter_pattern: str | None
    rebaseline: bool
    jobs: int | None
    analyze_only: bool = False
    validate: bool = False


class Suite(Protocol):
    """Protocol every suite module must satisfy."""

    #: Short identifier used with --suite (e.g. "ast", "perfetto-fmt").
    NAME: str
    #: One-line description shown in --list output.
    DESCRIPTION: str

    def run(self, ctx: SuiteContext) -> int:
        """Run the suite. Return 0 on success, non-zero on failure."""
        ...
