"""Build a platform-specific wheel with a pre-built syntaqlite binary.

Follows the go-to-wheel pattern: binary goes in {package}/bin/{name},
wheel is constructed directly as a zip with correct metadata and tags.

Usage:
    python -m python.dev.build_wheel \\
        --binary target/release/syntaqlite \\
        --platform aarch64-apple-darwin \\
        --out dist/
"""

import argparse
import base64
import csv
import hashlib
import io
import os
import stat
import sys
import zipfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent.parent
PYTHON_ROOT = ROOT / "python"

# Rust target → Python platform tag.
PLATFORM_TAGS: dict[str, str] = {
    "aarch64-apple-darwin": "macosx_11_0_arm64",
    "x86_64-apple-darwin": "macosx_10_12_x86_64",
    "x86_64-unknown-linux-gnu": "manylinux_2_17_x86_64.manylinux2014_x86_64",
    "aarch64-unknown-linux-gnu": "manylinux_2_17_aarch64.manylinux2014_aarch64",
    "x86_64-pc-windows-msvc": "win_amd64",
}


def _file_hash(data: bytes) -> str:
    digest = hashlib.sha256(data).digest()
    return "sha256=" + base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")


def _read_version() -> str:
    for line in (PYTHON_ROOT / "pyproject.toml").read_text().splitlines():
        if line.startswith("version"):
            return line.split('"')[1]
    sys.exit("Could not find version in pyproject.toml")


def build_wheel(binary_path: Path, platform_tag: str, out_dir: Path) -> Path:
    version = _read_version()
    name = "syntaqlite"
    is_windows = "win" in platform_tag
    binary_name = "syntaqlite.exe" if is_windows else "syntaqlite"

    # Collect all files as (archive_path, content) pairs.
    files: dict[str, bytes] = {}

    # 1. Binary in {package}/bin/
    files[f"{name}/bin/{binary_name}"] = binary_path.read_bytes()

    # 2. Python package files.
    pkg_dir = PYTHON_ROOT / name
    for py_file in sorted(pkg_dir.rglob("*.py")):
        arc = str(py_file.relative_to(PYTHON_ROOT))
        files[arc] = py_file.read_bytes()

    # 3. dist-info/METADATA
    readme_path = ROOT / "README.md"
    readme_text = readme_path.read_text() if readme_path.exists() else ""

    metadata = (
        "Metadata-Version: 2.1\n"
        f"Name: {name}\n"
        f"Version: {version}\n"
        "Summary: SQLite SQL tools — parser, formatter, validator, and MCP server\n"
        "License: Apache-2.0\n"
        "Home-page: https://github.com/LalitMaganti/syntaqlite\n"
        "Requires-Python: >=3.10\n"
        "Provides-Extra: mcp\n"
        'Requires-Dist: mcp>=1.0; extra == "mcp"\n'
        "Description-Content-Type: text/markdown\n"
        "\n"
        f"{readme_text}"
    )

    dist_info = f"{name}-{version}.dist-info"
    files[f"{dist_info}/METADATA"] = metadata.encode()

    # 4. dist-info/WHEEL
    wheel_meta = (
        "Wheel-Version: 1.0\n"
        "Generator: syntaqlite-build-wheel\n"
        "Root-Is-Purelib: false\n"
        f"Tag: py3-none-{platform_tag}\n"
    )
    files[f"{dist_info}/WHEEL"] = wheel_meta.encode()

    # 5. dist-info/entry_points.txt
    entry_points = (
        "[console_scripts]\n"
        f"syntaqlite = {name}:main\n"
        f"syntaqlite-mcp = {name}.mcp.server:mcp.run\n"
    )
    files[f"{dist_info}/entry_points.txt"] = entry_points.encode()

    # 6. dist-info/RECORD (must be last)
    record_path = f"{dist_info}/RECORD"
    files[record_path] = b""  # placeholder
    buf = io.StringIO()
    writer = csv.writer(buf)
    for path, content in files.items():
        if path == record_path:
            writer.writerow([path, "", ""])
        else:
            writer.writerow([path, _file_hash(content), len(content)])
    files[record_path] = buf.getvalue().encode()

    # Build the wheel zip.
    out_dir.mkdir(parents=True, exist_ok=True)
    wheel_name = f"{name}-{version}-py3-none-{platform_tag}.whl"
    wheel_path = out_dir / wheel_name

    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as whl:
        for arc_path, content in files.items():
            if "/bin/" in arc_path:
                info = zipfile.ZipInfo(arc_path)
                info.external_attr = (
                    stat.S_IRWXU | stat.S_IRGRP | stat.S_IXGRP |
                    stat.S_IROTH | stat.S_IXOTH
                ) << 16
                whl.writestr(info, content)
            else:
                whl.writestr(arc_path, content)

    print(f"Built {wheel_path} ({wheel_path.stat().st_size:,} bytes)")
    return wheel_path


def main():
    parser = argparse.ArgumentParser(
        description="Build a syntaqlite wheel with a pre-built binary"
    )
    parser.add_argument(
        "--binary", required=True, type=Path,
        help="Path to pre-built syntaqlite binary"
    )
    parser.add_argument(
        "--platform", required=True,
        help="Rust target triple (e.g. aarch64-apple-darwin) or Python platform tag"
    )
    parser.add_argument(
        "--out", required=True, type=Path,
        help="Output directory for the wheel"
    )
    args = parser.parse_args()

    platform = PLATFORM_TAGS.get(args.platform, args.platform)

    if not args.binary.exists():
        sys.exit(f"Binary not found: {args.binary}")

    build_wheel(args.binary, platform, args.out)


if __name__ == "__main__":
    main()
