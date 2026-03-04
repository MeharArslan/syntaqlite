# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Installs build dependencies to third_party/bin/ and third_party/src/."""

from __future__ import annotations

import argparse
import hashlib
import os
import platform
import shutil
import subprocess
import sys
import tarfile
import tempfile
import zipfile
from dataclasses import dataclass

# Global verbosity level
VERBOSITY: int = 0


def vprint(level: int, *args: object, **kwargs: object) -> None:
    """Print only if verbosity level is high enough."""
    if VERBOSITY >= level:
        print(*args, **kwargs)


ROOT_DIR: str = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
THIRD_PARTY_DIR: str = os.path.join(ROOT_DIR, "third_party")
THIRD_PARTY_BIN_DIR: str = os.path.join(THIRD_PARTY_DIR, "bin")
THIRD_PARTY_SRC_DIR: str = os.path.join(THIRD_PARTY_DIR, "src")

SQLITE_VERSION: str = "3510200"  # 3.51.2
SQLITE_YEAR: str = "2026"
RUST_VERSION: str = "1.93.0"  # Latest stable
EMSCRIPTEN_VERSION: str = "4.0.8"  # Pre-built tarballs from Perfetto's GCS
NODE_VERSION: str = "20.11.0"  # Pre-built from Chromium's storage (same as Perfetto)


@dataclass
class BinaryDep:
    """Binary dependency (platform-specific)."""
    name: str
    version: str
    url: str
    sha256: str
    target_os: str  # darwin, linux, windows, or all
    target_arch: str  # x64, arm64, or all
    format: str = "zip"
    strip_prefix: str = ""  # Directory prefix to strip from archive


@dataclass
class SourceDep:
    """Source dependency (platform-independent)."""
    name: str
    version: str
    url: str
    checksum: str
    strip_prefix: str  # Directory prefix to strip from archive
    format: str = "zip"
    hash_type: str = "sha3_256"  # sha256 or sha3_256


# fmt: off
BINARY_DEPS = [
    # clang-format: raw binaries from Chromium's cloud storage.
    BinaryDep("clang-format", "8503422f",
              "https://storage.googleapis.com/chromium-clang-format/8503422f469ae56cc74f0ea2c03f2d872f4a2303",
              "dabf93691361e8bd1d07466d67584072ece5c24e2b812c16458b8ff801c33e29",
              "darwin", "arm64", "raw"),
    BinaryDep("clang-format", "7d46d237",
              "https://storage.googleapis.com/chromium-clang-format/7d46d237f9664f41ef46b10c1392dcb559250f25",
              "0c3c13febeb0495ef0086509c24605ecae9e3d968ff9669d12514b8a55c7824e",
              "darwin", "x64", "raw"),
    BinaryDep("clang-format", "79a7b4e5",
              "https://storage.googleapis.com/chromium-clang-format/79a7b4e5336339c17b828de10d80611ff0f85961",
              "889266a51681d55bd4b9e02c9a104fa6ee22ecdfa7e8253532e5ea47e2e4cb4a",
              "linux", "x64", "raw"),
    BinaryDep("clang-format", "565cab9c",
              "https://storage.googleapis.com/chromium-clang-format/565cab9c66d61360c27c7d4df5defe1a78ab56d3",
              "5557943a174e3b67cdc389c10b0ceea2195f318c5c665dd77a427ed01a094557",
              "windows", "x64", "raw"),
    # Rust toolchain.
    # SHA256 hashes from https://static.rust-lang.org/dist/channel-rust-1.93.0.toml
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-aarch64-apple-darwin.tar.gz",
              "e33cf237cfff8af75581fedece9f3c348e976bb8246078786f1888c3b251d380",
              "darwin", "arm64", "tar.gz",
              f"rust-{RUST_VERSION}-aarch64-apple-darwin"),
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-x86_64-apple-darwin.tar.gz",
              "0297504189bdee029bacb61245cb131e3a2cc4bfd50c9e11281ea8957706e675",
              "darwin", "x64", "tar.gz",
              f"rust-{RUST_VERSION}-x86_64-apple-darwin"),
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-x86_64-unknown-linux-gnu.tar.gz",
              "ca55df589f7cd68eec883086c5ff63ece04a1820e6d23e514fbb412cc8bf77a4",
              "linux", "x64", "tar.gz",
              f"rust-{RUST_VERSION}-x86_64-unknown-linux-gnu"),
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-aarch64-unknown-linux-gnu.tar.gz",
              "091f981b95cbc6713ce6d6c23817286d4c10fd35fc76a990a3af430421751cfc",
              "linux", "arm64", "tar.gz",
              f"rust-{RUST_VERSION}-aarch64-unknown-linux-gnu"),
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-x86_64-pc-windows-msvc.tar.gz",
              "4d171c1a5a0e4b2450b6426be70faa9bf31848362262d4dc8e9b29072e268e43",
              "windows", "x64", "tar.gz",
              f"rust-{RUST_VERSION}-x86_64-pc-windows-msvc"),
]

# UI deps: emscripten toolchain, node.js, and wasm32 rust std.
# Installed only when --ui is passed.
UI_DEPS = [
    # Rust std for wasm32-unknown-emscripten target.
    BinaryDep("rust-wasm32", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-01-22/rust-std-{RUST_VERSION}-wasm32-unknown-emscripten.tar.gz",
              "3695cbe0527720c1104f327c356a6015756f15f0a63fdc2a00f78c316b0f7f8a",
              "all", "all", "tar.gz",
              f"rust-std-{RUST_VERSION}-wasm32-unknown-emscripten"),
    # Node.js: pre-built from Chromium's storage (same as Perfetto).
    BinaryDep("nodejs", NODE_VERSION,
              "https://storage.googleapis.com/chromium-nodejs/20.11.0/5b5681e12a21cda986410f69e03e6220a21dd4d2",
              "cecb99fbb369a9090dddc27e228b66335cd72555b44fa8839ef78e56c51682c5",
              "darwin", "arm64", "tar.gz",
              "node-darwin-arm64"),
    BinaryDep("nodejs", NODE_VERSION,
              "https://storage.googleapis.com/chromium-nodejs/20.11.0/e3c0fd53caae857309815f3f8de7c2dce49d7bca",
              "20affacca2480c368b75a1d91ec1a2720604b325207ef0cf39cfef3c235dad19",
              "darwin", "x64", "tar.gz",
              "node-darwin-x64"),
    BinaryDep("nodejs", NODE_VERSION,
              "https://storage.googleapis.com/chromium-nodejs/20.11.0/f9a337cfa0e2b92d3e5c671c26b454bd8e99769e",
              "0ba9cc91698c1f833a1fdc1fe0cb392d825ad484c71b0d84388ac80bfd3d6079",
              "linux", "x64", "tar.gz",
              "node-linux-x64"),
    # Emscripten: pre-built toolchain tarballs from Perfetto's GCS.
    # Contains emcc, clang, wasm-ld etc. ready to use.
    BinaryDep("emscripten", EMSCRIPTEN_VERSION,
              f"https://storage.googleapis.com/perfetto/emscripten-{EMSCRIPTEN_VERSION}-mac.tgz",
              "2682c43580ae2265b4c7f3c7963629f7f501eb24a8ffa01be0059f9f5b3b8cd0",
              "darwin", "arm64", "tar.gz",
              "install"),
    BinaryDep("emscripten", EMSCRIPTEN_VERSION,
              f"https://storage.googleapis.com/perfetto/emscripten-{EMSCRIPTEN_VERSION}-mac-x64.tgz",
              "e1b2e6d4797338ed884f9d8a8419f93fc42cfcdea5e8a8b29fe13c6fd3fe7f7a",
              "darwin", "x64", "tar.gz",
              "install"),
    BinaryDep("emscripten", EMSCRIPTEN_VERSION,
              f"https://storage.googleapis.com/perfetto/emscripten-{EMSCRIPTEN_VERSION}-linux.tgz",
              "2fd3e39b5e233bad39799c31029b6d6d5295135cb00c1bb2fd9a4b2c4b7c264b",
              "linux", "x64", "tar.gz",
              "install"),
]

SOURCE_DEPS = [
    SourceDep("sqlite", SQLITE_VERSION,
              f"https://sqlite.org/{SQLITE_YEAR}/sqlite-src-{SQLITE_VERSION}.zip",
              "e436bb919850445ce5168fb033d2d0d5c53a9d8c9602c7fa62b3e0025541d481",
              f"sqlite-src-{SQLITE_VERSION}"),
]
# fmt: on


def get_platform() -> tuple[str, str, str]:
    """Returns (os, arch, platform_dir)."""
    sys_name = platform.system().lower()
    machine = platform.machine().lower()

    if sys_name == "darwin":
        host_os, prefix = "darwin", "mac"
    elif sys_name == "linux":
        host_os, prefix = "linux", "linux"
    elif sys_name == "windows":
        host_os, prefix = "windows", "win"
    else:
        sys.exit(f"Unsupported OS: {sys_name}")

    host_arch = "arm64" if machine in ("arm64", "aarch64") else "x64"
    platform_dir = f"{prefix}-{'arm64' if host_arch == 'arm64' else 'amd64'}"

    return host_os, host_arch, platform_dir


def sha256_file(path: str) -> str:
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def sha3_256_file(path: str) -> str:
    h = hashlib.sha3_256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def extract(archive_path: str, dest_dir: str, fmt: str) -> None:
    if fmt == "zip":
        with zipfile.ZipFile(archive_path) as zf:
            zf.extractall(dest_dir)
    elif fmt == "tar.gz":
        with tarfile.open(archive_path, "r:gz") as tf:
            tf.extractall(dest_dir)
    elif fmt == "tar.xz":
        with tarfile.open(archive_path, "r:xz") as tf:
            tf.extractall(dest_dir)
    else:
        sys.exit(f"Unsupported format: {fmt}")


def install_rust(dep: BinaryDep, target_dir: str) -> bool:
    """Install Rust toolchain with proper directory structure."""
    rust_dir = os.path.join(target_dir, "rust")
    stamp_path = os.path.join(target_dir, f".{dep.name}.stamp")

    if os.path.exists(stamp_path):
        with open(stamp_path) as f:
            if f.read().strip() == dep.version:
                return True

    vprint(1, f"Downloading Rust {dep.version}...")
    os.makedirs(target_dir, exist_ok=True)

    with tempfile.NamedTemporaryFile(suffix=".tar.gz", delete=False) as tmp:
        tmp_path = tmp.name

    try:
        # Show progress bar only if not verbose (verbose mode shows curl output)
        curl_args = ["curl", "-fL", "-o", tmp_path, dep.url]
        if VERBOSITY == 0:
            curl_args.insert(2, "--progress-bar")
        result = subprocess.run(curl_args)
        if result.returncode != 0:
            print("Download failed", file=sys.stderr)
            return False

        vprint(1, "Verifying checksum...")
        actual_sha256 = sha256_file(tmp_path)
        if actual_sha256 != dep.sha256:
            print(f"SHA256 mismatch for Rust: expected {dep.sha256}, got {actual_sha256}", file=sys.stderr)
            return False

        # Extract and install Rust components
        vprint(1, "Extracting Rust tarball (this may take a minute)...")
        with tempfile.TemporaryDirectory() as extract_dir:
            extract(tmp_path, extract_dir, dep.format)
            src_dir = os.path.join(extract_dir, dep.strip_prefix)

            # Remove old installation
            if os.path.exists(rust_dir):
                vprint(1, "Removing old Rust installation...")
                shutil.rmtree(rust_dir)
            os.makedirs(rust_dir)

            # Copy rustc, cargo, and rust-std components
            vprint(1, "Installing rustc...")
            for component in ["rustc", "cargo"]:
                comp_dir = os.path.join(src_dir, component)
                if os.path.exists(comp_dir):
                    if component == "cargo":
                        vprint(1, "Installing cargo...")
                    dest_comp = os.path.join(rust_dir, component)
                    shutil.copytree(comp_dir, dest_comp)

            # Copy rust-std into rustc's lib/rustlib structure
            vprint(1, "Installing standard library...")

            # Find the rust-std directory
            std_dirs = [d for d in os.listdir(src_dir) if d.startswith("rust-std-")]
            if not std_dirs:
                print("Warning: rust-std component not found", file=sys.stderr)
                return False

            src_std_rustlib = os.path.join(src_dir, std_dirs[0], "lib", "rustlib")
            if not os.path.exists(src_std_rustlib):
                print("Warning: rust-std rustlib not found", file=sys.stderr)
                return False

            dest_rustlib = os.path.join(rust_dir, "rustc", "lib", "rustlib")

            # Merge each item from rust-std's rustlib into rustc's rustlib
            for item_name in os.listdir(src_std_rustlib):
                src_path = os.path.join(src_std_rustlib, item_name)
                dest_path = os.path.join(dest_rustlib, item_name)

                # If it's a file, just copy it
                if not os.path.isdir(src_path):
                    shutil.copy2(src_path, dest_path)
                    continue

                # If destination doesn't exist, just copy the whole directory
                if not os.path.exists(dest_path):
                    shutil.copytree(src_path, dest_path)
                    continue

                # Both exist - merge subdirectories (rustc has bin/, std has lib/)
                for sub_name in os.listdir(src_path):
                    src_sub = os.path.join(src_path, sub_name)
                    dest_sub = os.path.join(dest_path, sub_name)

                    # Remove existing destination
                    if os.path.exists(dest_sub):
                        if os.path.isdir(dest_sub):
                            shutil.rmtree(dest_sub)
                        else:
                            os.unlink(dest_sub)

                    # Move the item
                    shutil.move(src_sub, dest_sub)

            # Make binaries executable
            vprint(1, "Setting permissions...")
            for bindir in ["cargo/bin", "rustc/bin"]:
                bin_path = os.path.join(rust_dir, bindir)
                if os.path.exists(bin_path):
                    for exe in os.listdir(bin_path):
                        exe_path = os.path.join(bin_path, exe)
                        if os.path.isfile(exe_path):
                            os.chmod(exe_path, 0o755)

            vprint(1, "Cleaning up temporary files...")

        with open(stamp_path, "w") as f:
            f.write(dep.version)

        return True
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)


def install_binary_dep(dep: BinaryDep, target_dir: str) -> bool:
    """Install a binary dependency. Returns True on success."""
    # Special handling for Rust
    if dep.name == "rust":
        return install_rust(dep, target_dir)

    stamp_path = os.path.join(target_dir, f".{dep.name}.stamp")

    if os.path.exists(stamp_path):
        with open(stamp_path) as f:
            if f.read().strip() == dep.version:
                return True

    vprint(1, f"Downloading {dep.name}...")
    os.makedirs(target_dir, exist_ok=True)

    suffix = {"tar.gz": ".tar.gz", "zip": ".zip", "raw": ""}.get(dep.format, ".zip")
    with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as tmp:
        tmp_path = tmp.name

    try:
        curl_args = ["curl", "-fL", "-o", tmp_path, dep.url]
        if VERBOSITY == 0:
            curl_args.insert(2, "--progress-bar")
        result = subprocess.run(curl_args)
        if result.returncode != 0:
            print("Download failed", file=sys.stderr)
            return False

        actual_sha256 = sha256_file(tmp_path)
        if actual_sha256 != dep.sha256:
            print(f"SHA256 mismatch for {dep.name}: expected {dep.sha256}, got {actual_sha256}", file=sys.stderr)
            return False

        if dep.format == "raw":
            exe_path = os.path.join(target_dir, dep.name)
            shutil.copy2(tmp_path, exe_path)
            os.chmod(exe_path, 0o755)
        elif dep.strip_prefix:
            # Directory-structured dep: extract, strip prefix, move contents flat into target_dir
            with tempfile.TemporaryDirectory() as extract_dir:
                extract(tmp_path, extract_dir, dep.format)
                src_dir = os.path.join(extract_dir, dep.strip_prefix)
                for item in os.listdir(src_dir):
                    dest = os.path.join(target_dir, item)
                    if os.path.isdir(dest):
                        shutil.rmtree(dest)
                    elif os.path.exists(dest):
                        os.unlink(dest)
                    shutil.move(os.path.join(src_dir, item), dest)
            exe_path = os.path.join(target_dir, dep.name)
            if os.path.exists(exe_path):
                os.chmod(exe_path, 0o755)
        else:
            extract(tmp_path, target_dir, dep.format)
            exe_path = os.path.join(target_dir, dep.name)
            if os.path.exists(exe_path):
                os.chmod(exe_path, 0o755)

        with open(stamp_path, "w") as f:
            f.write(dep.version)

        return True
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)


def install_source_dep(dep: SourceDep, target_dir: str) -> bool:
    """Install a source dependency. Returns True on success."""
    dest_dir = os.path.join(target_dir, dep.name)
    stamp_path = os.path.join(target_dir, f".{dep.name}.stamp")

    if os.path.exists(stamp_path):
        with open(stamp_path) as f:
            if f.read().strip() == dep.version:
                return True

    vprint(1, f"Downloading {dep.name} source...")
    os.makedirs(target_dir, exist_ok=True)

    suffix = ".tar.gz" if dep.format == "tar.gz" else ".zip"
    with tempfile.NamedTemporaryFile(suffix=suffix, delete=False) as tmp:
        tmp_path = tmp.name

    try:
        curl_args = ["curl", "-fL", "-o", tmp_path, dep.url]
        if VERBOSITY == 0:
            curl_args.insert(2, "--progress-bar")
        result = subprocess.run(curl_args)
        if result.returncode != 0:
            print("Download failed", file=sys.stderr)
            return False

        if dep.hash_type == "sha256":
            actual_hash = sha256_file(tmp_path)
        else:
            actual_hash = sha3_256_file(tmp_path)
        if actual_hash != dep.checksum:
            print(f"Checksum mismatch for {dep.name}: expected {dep.checksum}, got {actual_hash}", file=sys.stderr)
            return False

        # Extract to temp dir first, then move stripped prefix to final location
        with tempfile.TemporaryDirectory() as extract_dir:
            extract(tmp_path, extract_dir, dep.format)

            # Remove existing destination if present
            if os.path.exists(dest_dir):
                shutil.rmtree(dest_dir)

            # Move the stripped prefix directory to final location
            src_path = os.path.join(extract_dir, dep.strip_prefix)
            shutil.move(src_path, dest_dir)

        with open(stamp_path, "w") as f:
            f.write(dep.version)

        vprint(1, f"Installed {dep.name} to {dest_dir}")
        return True
    finally:
        if os.path.exists(tmp_path):
            os.unlink(tmp_path)


def main() -> int:
    global VERBOSITY

    parser = argparse.ArgumentParser(description="Install build dependencies to third_party/")
    parser.add_argument(
        "-v", "--verbose",
        action="count",
        default=0,
        help="Increase verbosity (can be repeated: -v, -vv, -vvv)"
    )
    parser.add_argument(
        "--no-rust",
        action="store_true",
        help="Skip Rust toolchain installation"
    )
    parser.add_argument(
        "--ui",
        action="store_true",
        help="Install Emscripten SDK (for wasm/web-playground builds)"
    )
    args = parser.parse_args()
    VERBOSITY = args.verbose

    host_os, host_arch, platform_dir = get_platform()
    bin_target_dir = os.path.join(THIRD_PARTY_BIN_DIR, platform_dir)

    success = True

    def install_deps(deps, subdir=False):
        nonlocal success
        for dep in deps:
            if dep.name == "rust" and args.no_rust:
                continue
            os_match = dep.target_os == "all" or dep.target_os == host_os
            arch_match = dep.target_arch == "all" or dep.target_arch == host_arch
            if os_match and arch_match:
                dep_dir = os.path.join(bin_target_dir, dep.name) if subdir else bin_target_dir
                if not install_binary_dep(dep, dep_dir):
                    success = False

    # Install binary dependencies (always).
    install_deps(BINARY_DEPS)

    # Install UI dependencies (--ui only).
    # Each UI dep goes into its own subdirectory to avoid polluting the
    # shared bin dir with their bin/, lib/ etc.
    if args.ui:
        install_deps(UI_DEPS, subdir=True)

        # Patch emscripten_config to point NODE_JS at our hermetic node.
        em_config = os.path.join(bin_target_dir, "emscripten", "emscripten_config")
        node_bin = os.path.join(bin_target_dir, "nodejs", "bin", "node")
        if os.path.exists(em_config) and os.path.exists(node_bin):
            with open(em_config) as f:
                config = f.read()
            patched = []
            for line in config.splitlines():
                if line.startswith("NODE_JS"):
                    patched.append(f"NODE_JS = '{node_bin}'")
                else:
                    patched.append(line)
            with open(em_config, "w") as f:
                f.write("\n".join(patched) + "\n")

    # Install source dependencies
    for dep in SOURCE_DEPS:
        if not install_source_dep(dep, THIRD_PARTY_SRC_DIR):
            success = False

    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
