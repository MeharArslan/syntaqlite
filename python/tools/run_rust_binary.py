# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""A wrapper to run cargo, rustc and other Rust binaries from third_party/."""

import os
import platform
import subprocess
import sys

ROOT_DIR = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))


def get_platform_dir():
    """Returns the platform-specific buildtools subdirectory name."""
    sys_name = platform.system().lower()
    machine = platform.machine().lower()
    arch = "arm64" if machine in ("arm64", "aarch64") else "amd64"

    if sys_name == "darwin":
        return "mac-" + arch, ""
    elif sys_name == "linux":
        return "linux-" + arch, ""
    elif sys_name == "windows":
        return "win-" + arch, ".exe"
    else:
        return None, ""


def run_rust_binary(binary_name, args=None, cwd=None):
    """Run a Rust toolchain binary (cargo, rustc, etc.)."""
    if args is None:
        args = []

    os_dir, ext = get_platform_dir()
    if os_dir is None:
        print("OS not supported: %s" % platform.system())
        return 1

    rust_root = os.path.join(ROOT_DIR, "third_party", "bin", os_dir, "rust")

    # Rust binaries are in third_party/bin/{platform}/rust/{component}/bin/
    component = binary_name  # cargo or rustc
    exe_path = os.path.join(rust_root, component, "bin", binary_name) + ext

    if not os.path.exists(exe_path):
        print("Rust binary not found: %s" % exe_path)
        print("Run tools/dev/install-build-deps to install the Rust toolchain.")
        return 1

    # Set up environment for Rust toolchain
    rustc_path = os.path.join(rust_root, "rustc", "bin", "rustc") + ext
    if os.path.exists(rustc_path):
        os.environ["RUSTC"] = rustc_path

    # Set sysroot so rustc can find the standard library
    rustc_sysroot = os.path.join(rust_root, "rustc")
    os.environ["RUSTFLAGS"] = f"--sysroot {rustc_sysroot}"

    if cwd or platform.system().lower() == "windows":
        # subprocess for cross-platform compatibility and cwd support
        sys.exit(subprocess.call([exe_path] + args, cwd=cwd))
    else:
        os.execl(exe_path, os.path.basename(exe_path), *args)
