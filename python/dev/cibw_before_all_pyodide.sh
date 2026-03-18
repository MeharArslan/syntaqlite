#!/bin/bash
# cibuildwheel before-all hook for Pyodide (WASM) builds.
# Cross-compiles the Rust static library to wasm32-unknown-emscripten.
set -euo pipefail

REPO_ROOT="$(git rev-parse --show-toplevel)"
cd "$REPO_ROOT"

rustup target add wasm32-unknown-emscripten
cargo build -p syntaqlite --release --target wasm32-unknown-emscripten
