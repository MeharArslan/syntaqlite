#!/bin/bash
# cibuildwheel before-all hook for native platforms.
# Installs Rust (if needed) and builds the static library + CLI binary.
# {project} is substituted by cibuildwheel and points to the python/ directory.
set -euo pipefail

PROJECT_DIR="$1"
REPO_ROOT="$(cd "$PROJECT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# Install Rust if not present (needed inside manylinux containers).
if ! command -v cargo &>/dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    # shellcheck source=/dev/null
    source "$HOME/.cargo/env"
fi

# Build static library (for C extension) + CLI binary (bundled in wheel).
cargo build --release -p syntaqlite -p syntaqlite-cli

# Copy CLI binary into the package so setuptools includes it.
BINARY_NAME="syntaqlite"
mkdir -p python/syntaqlite/bin
cp "target/release/$BINARY_NAME" "python/syntaqlite/bin/"
