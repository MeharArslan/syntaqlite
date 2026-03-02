# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Amalgamation test executor.

Generates dialect amalgamations, compiles test binaries, and runs
diff tests against them.
"""

import os
import subprocess
import tempfile
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint


@dataclass
class DialectConfig:
    """Configuration for a dialect under test."""
    name: str
    actions_dir: Optional[str] = None
    nodes_dir: Optional[str] = None


def build_amalgamation(
    cli_binary: Path,
    dialect: DialectConfig,
    output_dir: Path,
) -> Path:
    """Generate amalgamated C files for a dialect.

    Returns the output directory.
    """
    cmd = [
        str(cli_binary), "dialect",
        "--name", dialect.name,
    ]
    if dialect.actions_dir:
        cmd += ["--actions-dir", dialect.actions_dir]
    if dialect.nodes_dir:
        cmd += ["--nodes-dir", dialect.nodes_dir]
    cmd += [
        "csrc",
        "--output-dir", str(output_dir),
    ]

    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"dialect generation failed for {dialect.name}:\n{proc.stderr}"
        )
    return output_dir


def compile_test_binary(
    test_c: Path,
    amalg_dir: Path,
    dialect: str,
    output_binary: Path,
) -> None:
    """Compile test_ast.c against an amalgamated dialect."""
    header = f'"syntaqlite_{dialect}.h"'
    create_fn = f"syntaqlite_create_{dialect}_parser"
    source = amalg_dir / f"syntaqlite_{dialect}.c"

    cmd = [
        "cc", "-o", str(output_binary),
        str(test_c), str(source),
        f"-I{amalg_dir}",
        f"-DDIALECT_HEADER={header}",
        f"-DDIALECT_CREATE_PARSER={create_fn}",
        "-Werror",
    ]

    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"Compilation failed for {dialect}:\n{proc.stderr}"
        )


class AmalgTestContext:
    """Manages build artifacts for amalgamation tests.

    Generates amalgamations and compiles test binaries once, then
    provides the binary path for running individual test cases.
    """

    def __init__(self, root_dir: Path, cli_binary: Path):
        self.root_dir = root_dir
        self.cli_binary = cli_binary
        self.test_c = root_dir / "tests/amalg_tests/test_ast.c"
        self._temp_dir = tempfile.TemporaryDirectory(prefix="syntaqlite_amalg_test_")
        self._binaries: Dict[str, Path] = {}

    def cleanup(self):
        self._temp_dir.cleanup()

    def get_binary(self, dialect: DialectConfig) -> Path:
        """Get the compiled test binary for a dialect, building if needed."""
        if dialect.name in self._binaries:
            return self._binaries[dialect.name]

        temp = Path(self._temp_dir.name)
        amalg_dir = temp / dialect.name
        amalg_dir.mkdir(exist_ok=True)

        build_amalgamation(
            self.cli_binary, dialect,
            amalg_dir,
        )

        binary = temp / f"test_{dialect.name}"
        compile_test_binary(self.test_c, amalg_dir, dialect.name, binary)

        self._binaries[dialect.name] = binary
        return binary
