# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Builds the Perfetto dialect wasm side module for web-playground upload."""

import argparse
import os
import platform
import shutil
import subprocess
import sys

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def get_platform_dir():
    sys_name = platform.system().lower()
    machine = platform.machine().lower()
    arch = "arm64" if machine in ("arm64", "aarch64") else "amd64"

    if sys_name == "darwin":
        return "mac-" + arch
    if sys_name == "linux":
        return "linux-" + arch
    return None


def configure_emscripten_env(env):
    """Prefer hermetic emscripten when present; fall back to PATH emcc."""
    platform_dir = get_platform_dir()
    if platform_dir is not None:
        bin_dir = os.path.join(ROOT_DIR, "third_party", "bin", platform_dir)
        emscripten_dir = os.path.join(bin_dir, "emscripten")
        nodejs_dir = os.path.join(bin_dir, "nodejs")
        if os.path.isdir(emscripten_dir):
            path_prepend = os.pathsep.join([
                os.path.join(nodejs_dir, "bin"),
                os.path.join(emscripten_dir, "emscripten"),
                os.path.join(emscripten_dir, "bin"),
            ])
            env["PATH"] = path_prepend + os.pathsep + env.get("PATH", "")
            env["EM_CONFIG"] = os.path.join(emscripten_dir, "emscripten_config")
            em_cache_dir = env.get("EM_CACHE", os.path.join(ROOT_DIR, ".cache", "emscripten-cache"))
            os.makedirs(em_cache_dir, exist_ok=True)
            env["EM_CACHE"] = em_cache_dir

    if shutil.which("emcc", path=env.get("PATH")) is None:
        print("error: emcc not found in PATH and hermetic emscripten is unavailable", file=sys.stderr)
        print("run: tools/dev/install-build-deps --ui  (or install emscripten manually)", file=sys.stderr)
        return False

    return True


def write_shim_headers(csrc_dir):
    runtime_h = os.path.join(csrc_dir, "syntaqlite_runtime.h")
    ext_h = os.path.join(csrc_dir, "syntaqlite_ext.h")

    with open(runtime_h, "w", encoding="utf-8") as f:
        f.write("""\
#ifndef SYNTAQLITE_RUNTIME_H
#define SYNTAQLITE_RUNTIME_H
#include "syntaqlite/config.h"
#include "syntaqlite/types.h"
#include "syntaqlite/dialect.h"
#include "syntaqlite/parser.h"
#include "syntaqlite/tokenizer.h"
#endif
""")

    with open(ext_h, "w", encoding="utf-8") as f:
        f.write("""\
#ifndef SYNTAQLITE_EXT_H
#define SYNTAQLITE_EXT_H
#include "syntaqlite_ext/sqlite_compat.h"
#include "syntaqlite_ext/arena.h"
#include "syntaqlite_ext/vec.h"
#include "syntaqlite_ext/ast_builder.h"
#endif
""")


def main():
    parser = argparse.ArgumentParser(
        description="Build Perfetto dialect wasm side module for browser upload.",
    )
    parser.add_argument(
        "--output",
        default=os.path.join("web-playground", "syntaqlite-perfetto.wasm"),
        help="output wasm path (default: web-playground/syntaqlite-perfetto.wasm)",
    )
    parser.add_argument(
        "--work-dir",
        default=os.path.join(".cache", "perfetto-wasm"),
        help="working directory for generated amalgamation inputs",
    )
    args = parser.parse_args()

    env = os.environ.copy()
    if not configure_emscripten_env(env):
        return 1

    work_dir = os.path.join(ROOT_DIR, args.work_dir)
    csrc_dir = os.path.join(work_dir, "csrc")
    os.makedirs(csrc_dir, exist_ok=True)

    actions_dir = os.path.join(ROOT_DIR, "tests", "amalg_tests", "perfetto_ext", "actions")
    nodes_dir = os.path.join(ROOT_DIR, "tests", "amalg_tests", "perfetto_ext", "nodes")

    cargo = os.path.join(ROOT_DIR, "tools", "dev", "cargo")
    rc = subprocess.call(
        [
            sys.executable,
            cargo,
            "--no-sysroot",
            "run",
            "-p",
            "syntaqlite-cli",
            "--",
            "dialect",
            "--name",
            "perfetto",
            "--actions-dir",
            actions_dir,
            "--nodes-dir",
            nodes_dir,
            "csrc",
            "--output-dir",
            csrc_dir,
        ],
        cwd=ROOT_DIR,
        env=env,
    )
    if rc != 0:
        print("failed to generate perfetto amalgamated C sources", file=sys.stderr)
        return rc

    write_shim_headers(csrc_dir)

    out_wasm = os.path.join(ROOT_DIR, args.output)
    os.makedirs(os.path.dirname(out_wasm), exist_ok=True)
    c_file = os.path.join(csrc_dir, "syntaqlite_perfetto.c")

    rc = subprocess.call(
        [
            "emcc",
            "-O3",
            "-g3",
            "-fPIC",
            c_file,
            "-I",
            csrc_dir,
            "-I",
            os.path.join(ROOT_DIR, "syntaqlite-runtime", "include"),
            "-sWASM_BIGINT",
            "-sSIDE_MODULE=1",
            "--no-entry",
            "-o",
            out_wasm,
        ],
        cwd=ROOT_DIR,
        env=env,
    )
    if rc != 0:
        print("failed to compile perfetto dialect wasm", file=sys.stderr)
        return rc

    print("wrote %s" % out_wasm)
    print("symbol: syntaqlite_perfetto_dialect (or syntaqlite_dialect)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
