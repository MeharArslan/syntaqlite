# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Amalgamation test executor.

Generates dialect amalgamations, compiles test binaries, and runs
diff tests against them.

Three amalgamation modes are supported:

  FULL         -- runtime inlined into dialect amalgam; self-contained
                  syntaqlite_<name>.{h,c} that compiles with no extra deps.
  DIALECT_ONLY -- dialect references an external syntaqlite_runtime.h;
                  runtime must be compiled and linked separately.
  RUNTIME_ONLY -- just the engine (syntaqlite_runtime.{h,c} +
                  syntaqlite_dialect.h); used to verify the runtime itself.
"""

import enum
import os
import subprocess
import tempfile
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, Optional

from python.syntaqlite.diff_tests.testing import DiffTestBlueprint


class AmalgMode(enum.Enum):
    FULL = "full"
    DIALECT_ONLY = "dialect_only"
    RUNTIME_ONLY = "runtime_only"


@dataclass
class DialectConfig:
    """Configuration for a dialect under test."""
    name: str
    mode: AmalgMode = AmalgMode.FULL
    actions_dir: Optional[str] = None
    nodes_dir: Optional[str] = None

    @property
    def key(self) -> str:
        """Unique build-cache key."""
        return f"{self.name}_{self.mode.value}"


# ---------------------------------------------------------------------------
# Amalgamation generation
# ---------------------------------------------------------------------------

def _build_full(cli_binary: Path, dialect: DialectConfig, output_dir: Path) -> None:
    cmd = [str(cli_binary), "dialect", "--name", dialect.name]
    if dialect.actions_dir:
        cmd += ["--actions-dir", dialect.actions_dir]
    if dialect.nodes_dir:
        cmd += ["--nodes-dir", dialect.nodes_dir]
    cmd += ["csrc", "--full", "--output-dir", str(output_dir)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"full dialect generation failed for {dialect.name}:\n{proc.stderr}"
        )


def _build_dialect_only(
    cli_binary: Path, dialect: DialectConfig, output_dir: Path
) -> None:
    cmd = [str(cli_binary), "dialect", "--name", dialect.name]
    if dialect.actions_dir:
        cmd += ["--actions-dir", dialect.actions_dir]
    if dialect.nodes_dir:
        cmd += ["--nodes-dir", dialect.nodes_dir]
    cmd += ["csrc", "--output-dir", str(output_dir)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"dialect-only generation failed for {dialect.name}:\n{proc.stderr}"
        )


def _build_runtime_only(cli_binary: Path, output_dir: Path) -> None:
    cmd = [str(cli_binary), "dialect", "--name", "sqlite", "runtime",
           "--output-dir", str(output_dir)]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"runtime-only generation failed:\n{proc.stderr}"
        )


# ---------------------------------------------------------------------------
# Compilation
# ---------------------------------------------------------------------------

def _compile_full_binary(
    test_c: Path, amalg_dir: Path, dialect_name: str, output_binary: Path
) -> None:
    """Compile test_ast.c against a self-contained full amalgamation."""
    header = f'"syntaqlite_{dialect_name}.h"'
    create_fn = f"syntaqlite_create_{dialect_name}_parser"
    source = amalg_dir / f"syntaqlite_{dialect_name}.c"
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
            f"Compilation failed (full) for {dialect_name}:\n{proc.stderr}"
        )


def _compile_dialect_only_binary(
    test_c: Path,
    amalg_dir: Path,
    runtime_dir: Path,
    dialect_name: str,
    output_binary: Path,
) -> None:
    """Compile test_ast.c against dialect-only + separate runtime."""
    header = f'"syntaqlite_{dialect_name}.h"'
    create_fn = f"syntaqlite_create_{dialect_name}_parser"
    dialect_src = amalg_dir / f"syntaqlite_{dialect_name}.c"
    runtime_src = runtime_dir / "syntaqlite_runtime.c"
    cmd = [
        "cc", "-o", str(output_binary),
        str(test_c), str(dialect_src), str(runtime_src),
        f"-I{amalg_dir}",
        f"-I{runtime_dir}",
        f"-DDIALECT_HEADER={header}",
        f"-DDIALECT_CREATE_PARSER={create_fn}",
        "-Werror",
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"Compilation failed (dialect-only) for {dialect_name}:\n{proc.stderr}"
        )


def _compile_runtime_only_binary(
    test_c: Path, runtime_dir: Path, output_binary: Path
) -> None:
    """Compile test_ast.c against the runtime-only amalgamation using the
    built-in sqlite dialect that ships inside the runtime."""
    header = '"syntaqlite_runtime.h"'
    create_fn = "syntaqlite_create_sqlite_parser"
    runtime_src = runtime_dir / "syntaqlite_runtime.c"
    cmd = [
        "cc", "-o", str(output_binary),
        str(test_c), str(runtime_src),
        f"-I{runtime_dir}",
        f"-DDIALECT_HEADER={header}",
        f"-DDIALECT_CREATE_PARSER={create_fn}",
        "-Werror",
    ]
    proc = subprocess.run(cmd, capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(
            f"Compilation failed (runtime-only):\n{proc.stderr}"
        )


# ---------------------------------------------------------------------------
# Context
# ---------------------------------------------------------------------------

class AmalgTestContext:
    """Manages build artifacts for amalgamation tests.

    Generates amalgamations and compiles test binaries once per unique
    (dialect, mode) configuration, then provides the binary path for
    running individual test cases.
    """

    def __init__(self, root_dir: Path, cli_binary: Path):
        self.root_dir = root_dir
        self.cli_binary = cli_binary
        self.test_c = root_dir / "tests/amalg_tests/test_ast.c"
        self._temp_dir = tempfile.TemporaryDirectory(prefix="syntaqlite_amalg_test_")
        self._binaries: Dict[str, Path] = {}
        # Shared runtime dir — built once and reused by all DIALECT_ONLY configs.
        self._runtime_dir: Optional[Path] = None

    def cleanup(self):
        self._temp_dir.cleanup()

    def _ensure_runtime(self) -> Path:
        if self._runtime_dir is not None:
            return self._runtime_dir
        temp = Path(self._temp_dir.name)
        runtime_dir = temp / "_runtime"
        runtime_dir.mkdir(exist_ok=True)
        _build_runtime_only(self.cli_binary, runtime_dir)
        self._runtime_dir = runtime_dir
        return runtime_dir

    def get_binary(self, dialect: DialectConfig) -> Path:
        """Get the compiled test binary for a dialect+mode, building if needed."""
        key = dialect.key
        if key in self._binaries:
            return self._binaries[key]

        temp = Path(self._temp_dir.name)
        amalg_dir = temp / key
        amalg_dir.mkdir(exist_ok=True)

        if dialect.mode == AmalgMode.FULL:
            _build_full(self.cli_binary, dialect, amalg_dir)
            binary = temp / f"test_{key}"
            _compile_full_binary(self.test_c, amalg_dir, dialect.name, binary)

        elif dialect.mode == AmalgMode.DIALECT_ONLY:
            _build_dialect_only(self.cli_binary, dialect, amalg_dir)
            runtime_dir = self._ensure_runtime()
            binary = temp / f"test_{key}"
            _compile_dialect_only_binary(
                self.test_c, amalg_dir, runtime_dir, dialect.name, binary
            )

        elif dialect.mode == AmalgMode.RUNTIME_ONLY:
            runtime_dir = self._ensure_runtime()
            binary = temp / f"test_{key}"
            _compile_runtime_only_binary(self.test_c, runtime_dir, binary)

        else:
            raise ValueError(f"Unknown AmalgMode: {dialect.mode}")

        self._binaries[key] = binary
        return binary
