# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Installs build dependencies to third_party/bin/ and third_party/src/."""

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
VERBOSITY = 0


def vprint(level, *args, **kwargs):
    """Print only if verbosity level is high enough."""
    if VERBOSITY >= level:
        print(*args, **kwargs)


ROOT_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
THIRD_PARTY_DIR = os.path.join(ROOT_DIR, "third_party")
THIRD_PARTY_BIN_DIR = os.path.join(THIRD_PARTY_DIR, "bin")
THIRD_PARTY_SRC_DIR = os.path.join(THIRD_PARTY_DIR, "src")

GN_VERSION = "5550ba0f4053c3cbb0bff3d60ded9d867b6fa371"
NINJA_VERSION = "1.13.2"
SQLITE_VERSION = "3510200"  # 3.51.2
SQLITE_YEAR = "2026"
RUST_VERSION = "1.93.0"  # Latest stable


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
    BinaryDep("gn", GN_VERSION, f"https://chrome-infra-packages.appspot.com/dl/gn/gn/mac-amd64/+/git_revision:{GN_VERSION}", "68c9ad9456dd93090c39134781833ee7865d19627541cb9ba9003aeea9ce4e26", "darwin", "x64"),
    BinaryDep("gn", GN_VERSION, f"https://chrome-infra-packages.appspot.com/dl/gn/gn/mac-arm64/+/git_revision:{GN_VERSION}", "2e55c4f65ce690fef9c03af8abe2b76e01e0017fc040c2d7529c089abbe48309", "darwin", "arm64"),
    BinaryDep("gn", GN_VERSION, f"https://chrome-infra-packages.appspot.com/dl/gn/gn/linux-amd64/+/git_revision:{GN_VERSION}", "be32d9e5a79d52145baf22f83a5e4aa83724d6bdcdf53370fa00e5eba45596fa", "linux", "x64"),
    BinaryDep("gn", GN_VERSION, f"https://chrome-infra-packages.appspot.com/dl/gn/gn/linux-arm64/+/git_revision:{GN_VERSION}", "bbd8bab058398a005d240c09c17ce0af4fab69ae8d022a40ac7d0a218681de73", "linux", "arm64"),
    BinaryDep("gn", GN_VERSION, f"https://chrome-infra-packages.appspot.com/dl/gn/gn/windows-amd64/+/git_revision:{GN_VERSION}", "c10f4622abd995a1c070e46b0df8bbe7b83278b9ff2b05ae8245dabf7cb02b8c", "windows", "x64"),
    BinaryDep("ninja", NINJA_VERSION, f"https://github.com/ninja-build/ninja/releases/download/v{NINJA_VERSION}/ninja-mac.zip", "c99048673aa765960a99cf10c6ddb9f1fad506099ff0a0e137ad8960a88f321b", "darwin", "all"),
    BinaryDep("ninja", NINJA_VERSION, f"https://github.com/ninja-build/ninja/releases/download/v{NINJA_VERSION}/ninja-linux.zip", "5749cbc4e668273514150a80e387a957f933c6ed3f5f11e03fb30955e2bbead6", "linux", "x64"),
    BinaryDep("ninja", NINJA_VERSION, f"https://github.com/ninja-build/ninja/releases/download/v{NINJA_VERSION}/ninja-linux-aarch64.zip", "fd2cacc8050a7f12a16a2e48f9e06fca5c14fc4c2bee2babb67b58be17a607fc", "linux", "arm64"),
    BinaryDep("ninja", NINJA_VERSION, f"https://github.com/ninja-build/ninja/releases/download/v{NINJA_VERSION}/ninja-win.zip", "07fc8261b42b20e71d1720b39068c2e14ffcee6396b76fb7a795fb460b78dc65", "windows", "x64"),
    # clang-format: raw binaries from Chromium's cloud storage.
    # SHA1s from https://chromium.googlesource.com/chromium/src/buildtools/+/refs/heads/master/{mac,linux64,win}/
    BinaryDep("clang-format", "8503422f", "https://storage.googleapis.com/chromium-clang-format/8503422f469ae56cc74f0ea2c03f2d872f4a2303", "dabf93691361e8bd1d07466d67584072ece5c24e2b812c16458b8ff801c33e29", "darwin", "arm64", "raw"),
    BinaryDep("clang-format", "7d46d237", "https://storage.googleapis.com/chromium-clang-format/7d46d237f9664f41ef46b10c1392dcb559250f25", "0c3c13febeb0495ef0086509c24605ecae9e3d968ff9669d12514b8a55c7824e", "darwin", "x64", "raw"),
    BinaryDep("clang-format", "79a7b4e5", "https://storage.googleapis.com/chromium-clang-format/79a7b4e5336339c17b828de10d80611ff0f85961", "889266a51681d55bd4b9e02c9a104fa6ee22ecdfa7e8253532e5ea47e2e4cb4a", "linux", "x64", "raw"),
    BinaryDep("clang-format", "565cab9c", "https://storage.googleapis.com/chromium-clang-format/565cab9c66d61360c27c7d4df5defe1a78ab56d3", "5557943a174e3b67cdc389c10b0ceea2195f318c5c665dd77a427ed01a094557", "windows", "x64", "raw"),
    # Rust toolchain: tar.gz on all platforms.
    # SHA256 hashes from https://static.rust-lang.org/dist/channel-rust-1.93.0.toml
    BinaryDep("rust", RUST_VERSION, f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-aarch64-apple-darwin.tar.gz", "e33cf237cfff8af75581fedece9f3c348e976bb8246078786f1888c3b251d380", "darwin", "arm64", "tar.gz", f"rust-{RUST_VERSION}-aarch64-apple-darwin"),
    BinaryDep("rust", RUST_VERSION, f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-x86_64-apple-darwin.tar.gz", "0297504189bdee029bacb61245cb131e3a2cc4bfd50c9e11281ea8957706e675", "darwin", "x64", "tar.gz", f"rust-{RUST_VERSION}-x86_64-apple-darwin"),
    BinaryDep("rust", RUST_VERSION, f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-x86_64-unknown-linux-gnu.tar.gz", "ca55df589f7cd68eec883086c5ff63ece04a1820e6d23e514fbb412cc8bf77a4", "linux", "x64", "tar.gz", f"rust-{RUST_VERSION}-x86_64-unknown-linux-gnu"),
    BinaryDep("rust", RUST_VERSION, f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-aarch64-unknown-linux-gnu.tar.gz", "091f981b95cbc6713ce6d6c23817286d4c10fd35fc76a990a3af430421751cfc", "linux", "arm64", "tar.gz", f"rust-{RUST_VERSION}-aarch64-unknown-linux-gnu"),
    BinaryDep("rust", RUST_VERSION, f"https://static.rust-lang.org/dist/2026-01-22/rust-{RUST_VERSION}-x86_64-pc-windows-msvc.tar.gz", "4d171c1a5a0e4b2450b6426be70faa9bf31848362262d4dc8e9b29072e268e43", "windows", "x64", "tar.gz", f"rust-{RUST_VERSION}-x86_64-pc-windows-msvc"),
]

SOURCE_DEPS = [
    SourceDep("sqlite", SQLITE_VERSION, f"https://sqlite.org/{SQLITE_YEAR}/sqlite-src-{SQLITE_VERSION}.zip", "e436bb919850445ce5168fb033d2d0d5c53a9d8c9602c7fa62b3e0025541d481", f"sqlite-src-{SQLITE_VERSION}"),
]
# fmt: on


def get_platform():
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


def sha256_file(path):
    h = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def sha3_256_file(path):
    h = hashlib.sha3_256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            h.update(chunk)
    return h.hexdigest()


def extract(archive_path, dest_dir, fmt):
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


def install_rust(dep, target_dir):
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
            print(f"Download failed", file=sys.stderr)
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


def install_binary_dep(dep, target_dir):
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
            print(f"Download failed", file=sys.stderr)
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


def install_source_dep(dep, target_dir):
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
            print(f"Download failed", file=sys.stderr)
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


def main():
    global VERBOSITY

    parser = argparse.ArgumentParser(description="Install build dependencies to third_party/")
    parser.add_argument(
        "-v", "--verbose",
        action="count",
        default=0,
        help="Increase verbosity (can be repeated: -v, -vv, -vvv)"
    )
    parser.add_argument(
        "--rust",
        action="store_true",
        help="Install Rust toolchain (optional, for Rust rewrite)"
    )
    args = parser.parse_args()
    VERBOSITY = args.verbose

    host_os, host_arch, platform_dir = get_platform()
    bin_target_dir = os.path.join(THIRD_PARTY_BIN_DIR, platform_dir)

    success = True

    # Install binary dependencies
    for dep in BINARY_DEPS:
        # Skip Rust unless --rust flag is provided
        if dep.name == "rust" and not args.rust:
            continue

        os_match = dep.target_os == "all" or dep.target_os == host_os
        arch_match = dep.target_arch == "all" or dep.target_arch == host_arch
        if os_match and arch_match:
            if not install_binary_dep(dep, bin_target_dir):
                success = False

    # Install source dependencies
    for dep in SOURCE_DEPS:
        if not install_source_dep(dep, THIRD_PARTY_SRC_DIR):
            success = False

    return 0 if success else 1


if __name__ == "__main__":
    sys.exit(main())
