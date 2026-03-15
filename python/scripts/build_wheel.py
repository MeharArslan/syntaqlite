#!/usr/bin/env python3
# Copyright 2025 The syntaqlite Authors. All rights reserved.
# Licensed under the Apache License, Version 2.0.

"""Build a platform-specific wheel with a pre-built syntaqlite binary.

Usage:
    python build_wheel.py --binary /path/to/syntaqlite --platform macosx_11_0_arm64 --out dist/
    python build_wheel.py --binary /path/to/syntaqlite.exe --platform win_amd64 --out dist/

The wheel is a standard Python wheel with the binary in the .data/scripts/
directory, which pip installs to the user's bin/Scripts directory.
"""

import argparse
import hashlib
import base64
import csv
import io
import os
import shutil
import stat
import sys
import tempfile
import zipfile
from pathlib import Path

# Rust target → Python platform tag mapping.
PLATFORM_TAGS = {
    "aarch64-apple-darwin": "macosx_11_0_arm64",
    "x86_64-apple-darwin": "macosx_10_12_x86_64",
    "x86_64-unknown-linux-gnu": "manylinux_2_17_x86_64.manylinux2014_x86_64",
    "aarch64-unknown-linux-gnu": "manylinux_2_17_aarch64.manylinux2014_aarch64",
    "x86_64-pc-windows-msvc": "win_amd64",
}

ROOT = Path(__file__).resolve().parent.parent


def read_version():
    """Read version from pyproject.toml."""
    for line in (ROOT / "pyproject.toml").read_text().splitlines():
        if line.startswith("version"):
            return line.split('"')[1]
    sys.exit("Could not find version in pyproject.toml")


def sha256_digest(data: bytes) -> str:
    return base64.urlsafe_b64encode(hashlib.sha256(data).digest()).rstrip(b"=").decode()


def build_wheel(binary_path: Path, platform: str, out_dir: Path):
    version = read_version()
    name = "syntaqlite"
    tag = f"py3-none-{platform}"
    wheel_name = f"{name}-{version}-{tag}.whl"

    out_dir.mkdir(parents=True, exist_ok=True)
    wheel_path = out_dir / wheel_name

    dist_info = f"{name}-{version}.dist-info"
    data_dir = f"{name}-{version}.data"

    # Determine binary name in the wheel.
    bin_name = "syntaqlite.exe" if "win" in platform else "syntaqlite"

    # Build the wheel as a zip.
    records = []

    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as whl:
        # 1. Add the binary to .data/scripts/
        binary_data = binary_path.read_bytes()
        scripts_path = f"{data_dir}/scripts/{bin_name}"
        whl.writestr(scripts_path, binary_data)
        records.append((scripts_path, sha256_digest(binary_data), len(binary_data)))

        # Make binary executable (Unix).
        if "win" not in platform:
            info = whl.getinfo(scripts_path)
            info.external_attr = (0o755 | stat.S_IFREG) << 16

        # 2. Add the Python package.
        pkg_dir = ROOT / "syntaqlite"
        for py_file in sorted(pkg_dir.rglob("*.py")):
            rel = py_file.relative_to(ROOT)
            arc_name = str(rel)
            data = py_file.read_bytes()
            whl.writestr(arc_name, data)
            records.append((arc_name, sha256_digest(data), len(data)))

        # 3. METADATA
        metadata = f"""\
Metadata-Version: 2.1
Name: {name}
Version: {version}
Summary: SQLite SQL tools — parser, formatter, validator, and MCP server
License: Apache-2.0
Requires-Python: >=3.10
Project-URL: Homepage, https://github.com/LalitMaganti/syntaqlite
Project-URL: Documentation, https://docs.syntaqlite.com
Provides-Extra: mcp
Requires-Dist: mcp>=1.0; extra == "mcp"
"""
        metadata_bytes = metadata.encode()
        metadata_path = f"{dist_info}/METADATA"
        whl.writestr(metadata_path, metadata_bytes)
        records.append((metadata_path, sha256_digest(metadata_bytes), len(metadata_bytes)))

        # 4. WHEEL
        wheel_meta = f"""\
Wheel-Version: 1.0
Generator: syntaqlite-build-wheel
Root-Is-Purelib: false
Tag: {tag}
"""
        wheel_bytes = wheel_meta.encode()
        wheel_meta_path = f"{dist_info}/WHEEL"
        whl.writestr(wheel_meta_path, wheel_bytes)
        records.append((wheel_meta_path, sha256_digest(wheel_bytes), len(wheel_bytes)))

        # 5. entry_points.txt
        entry_points = """\
[console_scripts]
syntaqlite-mcp = syntaqlite.mcp.server:mcp.run
"""
        ep_bytes = entry_points.encode()
        ep_path = f"{dist_info}/entry_points.txt"
        whl.writestr(ep_path, ep_bytes)
        records.append((ep_path, sha256_digest(ep_bytes), len(ep_bytes)))

        # 6. RECORD (must be last, references itself without hash)
        record_buf = io.StringIO()
        writer = csv.writer(record_buf)
        for path, digest, size in records:
            writer.writerow([path, f"sha256={digest}", str(size)])
        record_path = f"{dist_info}/RECORD"
        writer.writerow([record_path, "", ""])
        record_bytes = record_buf.getvalue().encode()
        whl.writestr(record_path, record_bytes)

    print(f"Built {wheel_path} ({wheel_path.stat().st_size:,} bytes)")
    return wheel_path


def main():
    parser = argparse.ArgumentParser(description="Build a syntaqlite wheel with a pre-built binary")
    parser.add_argument("--binary", required=True, type=Path, help="Path to pre-built syntaqlite binary")
    parser.add_argument("--platform", required=True, help="Python platform tag or Rust target triple")
    parser.add_argument("--out", required=True, type=Path, help="Output directory for the wheel")
    args = parser.parse_args()

    # Accept either Rust target or Python platform tag.
    platform = PLATFORM_TAGS.get(args.platform, args.platform)

    if not args.binary.exists():
        sys.exit(f"Binary not found: {args.binary}")

    build_wheel(args.binary, platform, args.out)


if __name__ == "__main__":
    main()
