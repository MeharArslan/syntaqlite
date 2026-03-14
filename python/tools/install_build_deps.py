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
RUST_VERSION: str = "1.94.0"  # Latest stable
EMSCRIPTEN_VERSION: str = "4.0.8"  # Pre-built tarballs from Perfetto's GCS
NODE_VERSION: str = "20.11.0"  # Pre-built from Chromium's storage (same as Perfetto)

# Pinned commit for the Perfetto stdlib checkout.
PERFETTO_REPO: str = "https://github.com/google/perfetto.git"
PERFETTO_COMMIT: str = "HEAD"  # TODO: pin to a specific commit for CI reproducibility


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
    # SHA256 hashes from https://static.rust-lang.org/dist/channel-rust-1.94.0.toml
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-03-05/rust-{RUST_VERSION}-aarch64-apple-darwin.tar.gz",
              "94903e93a4334d42bb6d92377a39903349c07f3709c792864bcdf7959f3c8c7d",
              "darwin", "arm64", "tar.gz",
              f"rust-{RUST_VERSION}-aarch64-apple-darwin"),
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-03-05/rust-{RUST_VERSION}-x86_64-apple-darwin.tar.gz",
              "97724032da92646194a802a7991f1166c4dc9f0a63f3bb01a53860e98f31d08c",
              "darwin", "x64", "tar.gz",
              f"rust-{RUST_VERSION}-x86_64-apple-darwin"),
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-03-05/rust-{RUST_VERSION}-x86_64-unknown-linux-gnu.tar.gz",
              "3bb1925a0a5ad2c17be731ee6e977e4a68490ab2182086db897bd28be21e965f",
              "linux", "x64", "tar.gz",
              f"rust-{RUST_VERSION}-x86_64-unknown-linux-gnu"),
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-03-05/rust-{RUST_VERSION}-aarch64-unknown-linux-gnu.tar.gz",
              "a0dc5a65ab337421347533e5be11d3fab11f119683a0dbd257ef3fe968bd2d72",
              "linux", "arm64", "tar.gz",
              f"rust-{RUST_VERSION}-aarch64-unknown-linux-gnu"),
    BinaryDep("rust", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-03-05/rust-{RUST_VERSION}-x86_64-pc-windows-msvc.tar.gz",
              "b349a6eace4063e4a89d9be1de2e77b20bd0193016a43036522f453be709c0f8",
              "windows", "x64", "tar.gz",
              f"rust-{RUST_VERSION}-x86_64-pc-windows-msvc"),
    # sqlite3 CLI tools: precompiled binaries from sqlite.org.
    # SHA256 hashes computed from https://sqlite.org/2026/sqlite-tools-*-3510200.zip
    BinaryDep("sqlite-tools", SQLITE_VERSION,
              f"https://sqlite.org/{SQLITE_YEAR}/sqlite-tools-osx-arm64-{SQLITE_VERSION}.zip",
              "0d672a4729817bc92034004b7c81b6876e961f4701416593e051b7bf535c943c",
              "darwin", "arm64", "zip"),
    BinaryDep("sqlite-tools", SQLITE_VERSION,
              f"https://sqlite.org/{SQLITE_YEAR}/sqlite-tools-osx-x64-{SQLITE_VERSION}.zip",
              "0960f6221bc58605e6099bbeac21bd31ed53e9a6a9fd5ecb2af4e3c4e14364f6",
              "darwin", "x64", "zip"),
    BinaryDep("sqlite-tools", SQLITE_VERSION,
              f"https://sqlite.org/{SQLITE_YEAR}/sqlite-tools-linux-x64-{SQLITE_VERSION}.zip",
              "4d8fbfe3548ff28906c3e91cd2b0415c490a459c78901b6594084984dc17e818",
              "linux", "x64", "zip"),
    BinaryDep("sqlite-tools", SQLITE_VERSION,
              f"https://sqlite.org/{SQLITE_YEAR}/sqlite-tools-win-x64-{SQLITE_VERSION}.zip",
              "042805d77076e2b806c86b1ac4082d65c2f2d4bdef5ce8995eed7845878fd69f",
              "windows", "x64", "zip"),
]

# UI deps: emscripten toolchain, node.js, and wasm32 rust std.
# Installed only when --ui is passed.
UI_DEPS = [
    # Rust std for wasm32-unknown-emscripten target.
    BinaryDep("rust-wasm32", RUST_VERSION,
              f"https://static.rust-lang.org/dist/2026-03-05/rust-std-{RUST_VERSION}-wasm32-unknown-emscripten.tar.gz",
              "2f9048f9254278249492b132ae2ac3b969aadafd659a1d2fe63a591dcb7651cc",
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
    SourceDep("sqlite-amalgamation", SQLITE_VERSION,
              f"https://sqlite.org/{SQLITE_YEAR}/sqlite-amalgamation-{SQLITE_VERSION}.zip",
              "9a9dd4eef7a97809bfacd84a7db5080a5c0eff7aaf1fc1aca20a6dc9a0c26f96",
              f"sqlite-amalgamation-{SQLITE_VERSION}"),
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
            # Try to chmod the binary matching dep.name; if it doesn't exist
            # (e.g. sqlite-tools zip contains sqlite3, not sqlite-tools),
            # make all extracted files executable.
            exe_path = os.path.join(target_dir, dep.name)
            if os.path.exists(exe_path):
                os.chmod(exe_path, 0o755)
            else:
                if dep.format == "zip":
                    with zipfile.ZipFile(tmp_path) as zf:
                        for name in zf.namelist():
                            p = os.path.join(target_dir, name)
                            if os.path.isfile(p):
                                os.chmod(p, 0o755)

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

    if os.path.exists(stamp_path) and os.path.isdir(dest_dir):
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


def install_perfetto(target_dir: str, commit: str) -> bool:
    """Shallow-clone the Perfetto repo for stdlib testing.

    Clones only the stdlib subtree to third_party/src/perfetto/.
    """
    dest_dir = os.path.join(target_dir, "perfetto")
    stamp_path = os.path.join(target_dir, ".perfetto.stamp")

    # Check if already installed at the right commit.
    if os.path.exists(stamp_path) and os.path.isdir(dest_dir):
        with open(stamp_path) as f:
            if f.read().strip() == commit:
                return True

    vprint(1, "Cloning Perfetto repository...")
    os.makedirs(target_dir, exist_ok=True)

    # Remove stale checkout.
    if os.path.exists(dest_dir):
        shutil.rmtree(dest_dir)

    result = subprocess.run(
        ["git", "clone", "--depth=1", "--filter=blob:none",
         "--sparse", PERFETTO_REPO, dest_dir],
        capture_output=VERBOSITY == 0,
    )
    if result.returncode != 0:
        print("Failed to clone Perfetto repo", file=sys.stderr)
        return False

    # Sparse checkout: only the stdlib directory.
    result = subprocess.run(
        ["git", "-C", dest_dir, "sparse-checkout", "set",
         "src/trace_processor/perfetto_sql/stdlib"],
        capture_output=VERBOSITY == 0,
    )
    if result.returncode != 0:
        print("Failed to set sparse checkout", file=sys.stderr)
        return False

    # Record the actual HEAD commit for the stamp.
    actual_commit = subprocess.run(
        ["git", "-C", dest_dir, "rev-parse", "HEAD"],
        capture_output=True, text=True,
    ).stdout.strip()

    with open(stamp_path, "w") as f:
        f.write(actual_commit if commit == "HEAD" else commit)

    vprint(1, f"Installed Perfetto stdlib to {dest_dir} (commit {actual_commit[:12]})")
    return True


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
    parser.add_argument(
        "--perfetto",
        action="store_true",
        help="Clone the Perfetto repo stdlib for format testing"
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

    # Install Perfetto stdlib (--perfetto only).
    if args.perfetto:
        if not install_perfetto(THIRD_PARTY_SRC_DIR, PERFETTO_COMMIT):
            success = False

    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
