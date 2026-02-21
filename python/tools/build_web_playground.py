# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Builds the emscripten-target wasm module used by web-playground."""

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
    elif sys_name == "linux":
        return "linux-" + arch
    else:
        return None


def main():
    parser = argparse.ArgumentParser(
        description="Build the emscripten-target wasm module for web-playground.",
    )
    parser.add_argument(
        "--rustflags",
        default="",
        help="extra target rustc flags appended for wasm32-unknown-emscripten",
    )
    args = parser.parse_args()

    platform_dir = get_platform_dir()
    if platform_dir is None:
        print("error: unsupported platform %s-%s" % (platform.system(), platform.machine()),
              file=sys.stderr)
        return 1

    bin_dir = os.path.join(ROOT_DIR, "third_party", "bin", platform_dir)
    emscripten_dir = os.path.join(bin_dir, "emscripten")
    nodejs_dir = os.path.join(bin_dir, "nodejs")

    if not os.path.isdir(emscripten_dir):
        print("emscripten not found at %s" % emscripten_dir, file=sys.stderr)
        print("run: tools/dev/install-build-deps --ui", file=sys.stderr)
        return 1

    # Add hermetic node and emscripten to PATH.
    env = os.environ.copy()
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

    # Point rustc at the wasm32 standard library installed by install-build-deps --ui.
    # --no-sysroot prevents the global RUSTFLAGS --sysroot so that the target-specific
    # CARGO_TARGET_*_RUSTFLAGS can set it for wasm32 without conflict.
    wasm32_sysroot = os.path.join(
        bin_dir, "rust-wasm32", "rust-std-wasm32-unknown-emscripten",
    )
    target_rustflags = env.get("CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS", "")
    target_rustflags += " --sysroot " + wasm32_sysroot
    # Build runtime as emscripten main module and expose dynamic linker APIs.
    target_rustflags += " -C link-arg=-sMAIN_MODULE=2"
    target_rustflags += " -C link-arg=-sALLOW_TABLE_GROWTH"
    target_rustflags += (
        " -C link-arg=-sEXPORTED_RUNTIME_METHODS=[\"loadDynamicLibrary\",\"ccall\",\"cwrap\"]"
    )
    if args.rustflags:
        target_rustflags += " " + args.rustflags
    env["CARGO_TARGET_WASM32_UNKNOWN_EMSCRIPTEN_RUSTFLAGS"] = target_rustflags.strip()

    cargo = os.path.join(ROOT_DIR, "tools", "dev", "cargo")
    rc = subprocess.call(
        [
            sys.executable, cargo, "--no-sysroot",
            "build", "-p", "syntaqlite-wasm",
            "--target", "wasm32-unknown-emscripten", "--release",
        ],
        cwd=ROOT_DIR,
        env=env,
    )
    if rc != 0:
        return rc

    # Copy the built wasm to web-playground/.
    wasm_target_dir = os.path.join(ROOT_DIR, "target", "wasm32-unknown-emscripten", "release")
    out_runtime_js = os.path.join(ROOT_DIR, "web-playground", "public", "syntaqlite-runtime.js")
    out_runtime_wasm = os.path.join(ROOT_DIR, "web-playground", "public", "syntaqlite-runtime.wasm")
    out_dialect = os.path.join(ROOT_DIR, "web-playground", "public", "syntaqlite-sqlite.wasm")

    runtime_js_src = None
    runtime_wasm_src = None
    js_candidates = [
        os.path.join(wasm_target_dir, "syntaqlite-wasm.js"),
        os.path.join(wasm_target_dir, "syntaqlite_wasm.js"),
    ]
    wasm_candidates = [
        os.path.join(wasm_target_dir, "syntaqlite-wasm.wasm"),
        os.path.join(wasm_target_dir, "syntaqlite_wasm.wasm"),
    ]
    for js_path in js_candidates:
        if not os.path.isfile(js_path):
            continue
        for wasm_path in wasm_candidates:
            if os.path.isfile(wasm_path):
                runtime_js_src = js_path
                runtime_wasm_src = wasm_path
                break
        if runtime_js_src and runtime_wasm_src:
            break
    else:
        print("could not locate built runtime js/wasm in %s" % wasm_target_dir, file=sys.stderr)
        return 1

    shutil.copy2(runtime_js_src, out_runtime_js)
    shutil.copy2(runtime_wasm_src, out_runtime_wasm)

    # Build built-in SQLite dialect wasm from C sources directly.
    wrapper_dir = os.path.join(ROOT_DIR, ".cache", "web-playground")
    os.makedirs(wrapper_dir, exist_ok=True)
    wrapper_path = os.path.join(wrapper_dir, "dialect_alias.c")
    with open(wrapper_path, "w", encoding="utf-8") as f:
        f.write("""\
#include "syntaqlite/dialect.h"
#include "syntaqlite/parser.h"

const SyntaqliteDialect* syntaqlite_sqlite_dialect(void);

const SyntaqliteDialect* syntaqlite_dialect(void) {
    return syntaqlite_sqlite_dialect();
}

SyntaqliteParser* syntaqlite_create_parser_with_dialect(
    const SyntaqliteMemMethods* mem,
    const SyntaqliteDialect* dialect
) {
    (void)mem;
    (void)dialect;
    return 0;
}
""")

    emcc_cmd = [
        "emcc",
        "-O3",
        "-fPIC",
        os.path.join(ROOT_DIR, "syntaqlite", "csrc", "dialect.c"),
        os.path.join(ROOT_DIR, "syntaqlite", "csrc", "sqlite_parse.c"),
        os.path.join(ROOT_DIR, "syntaqlite", "csrc", "sqlite_tokenize.c"),
        os.path.join(ROOT_DIR, "syntaqlite", "csrc", "sqlite_keyword.c"),
        wrapper_path,
        "-I", os.path.join(ROOT_DIR, "syntaqlite"),
        "-I", os.path.join(ROOT_DIR, "syntaqlite", "include"),
        "-I", os.path.join(ROOT_DIR, "syntaqlite-runtime", "include"),
        "-sWASM_BIGINT",
        "-sSIDE_MODULE=1",
        "--no-entry",
        "-o", out_dialect,
    ]
    rc = subprocess.call(emcc_cmd, cwd=ROOT_DIR, env=env)
    if rc != 0:
        print("failed to build sqlite dialect wasm from C sources", file=sys.stderr)
        return 1

    print("wrote %s" % out_runtime_js)
    print("wrote %s" % out_runtime_wasm)
    print("wrote %s" % out_dialect)

    # Build Perfetto dialect wasm side module.
    perfetto_work_dir = os.path.join(ROOT_DIR, ".cache", "perfetto-wasm")
    perfetto_csrc_dir = os.path.join(perfetto_work_dir, "csrc")
    os.makedirs(perfetto_csrc_dir, exist_ok=True)

    rc = subprocess.call(
        [
            sys.executable, cargo, "--no-sysroot",
            "run", "-p", "syntaqlite-cli", "--",
            "dialect", "--name", "perfetto",
            "--actions-dir", os.path.join(ROOT_DIR, "dialects", "perfetto", "actions"),
            "--nodes-dir", os.path.join(ROOT_DIR, "dialects", "perfetto", "nodes"),
            "csrc", "--output-dir", perfetto_csrc_dir,
        ],
        cwd=ROOT_DIR, env=env,
    )
    if rc != 0:
        print("failed to generate perfetto amalgamated C sources", file=sys.stderr)
        return rc

    # Write shim headers for side module compilation.
    with open(os.path.join(perfetto_csrc_dir, "syntaqlite_runtime.h"), "w", encoding="utf-8") as f:
        f.write("#ifndef SYNTAQLITE_RUNTIME_H\n#define SYNTAQLITE_RUNTIME_H\n"
                "#include \"syntaqlite/config.h\"\n#include \"syntaqlite/types.h\"\n"
                "#include \"syntaqlite/dialect.h\"\n#include \"syntaqlite/parser.h\"\n"
                "#include \"syntaqlite/tokenizer.h\"\n#endif\n")
    with open(os.path.join(perfetto_csrc_dir, "syntaqlite_ext.h"), "w", encoding="utf-8") as f:
        f.write("#ifndef SYNTAQLITE_EXT_H\n#define SYNTAQLITE_EXT_H\n"
                "#include \"syntaqlite_ext/sqlite_compat.h\"\n#include \"syntaqlite_ext/arena.h\"\n"
                "#include \"syntaqlite_ext/vec.h\"\n#include \"syntaqlite_ext/ast_builder.h\"\n"
                "#endif\n")

    out_perfetto = os.path.join(ROOT_DIR, "web-playground", "public", "syntaqlite-perfetto.wasm")
    rc = subprocess.call(
        [
            "emcc", "-O3", "-fPIC",
            os.path.join(perfetto_csrc_dir, "syntaqlite_perfetto.c"),
            "-I", perfetto_csrc_dir,
            "-I", os.path.join(ROOT_DIR, "syntaqlite-runtime", "include"),
            "-sWASM_BIGINT", "-sSIDE_MODULE=1", "--no-entry",
            "-o", out_perfetto,
        ],
        cwd=ROOT_DIR, env=env,
    )
    if rc != 0:
        print("failed to compile perfetto dialect wasm", file=sys.stderr)
        return rc

    print("wrote %s" % out_perfetto)
    return 0


if __name__ == "__main__":
    sys.exit(main())
