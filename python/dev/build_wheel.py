"""Build a platform-specific wheel with a pre-built syntaqlite binary.

Follows the go-to-wheel pattern: binary goes in {package}/bin/{name},
wheel is constructed directly as a zip with correct metadata and tags.

Usage:
    python -m python.dev.build_wheel \\
        --binary target/release/syntaqlite \\
        --platform aarch64-apple-darwin \\
        --out dist/

    # With C extension (per-Python-version wheel):
    python -m python.dev.build_wheel \\
        --binary target/release/syntaqlite \\
        --ext _syntaqlite.cpython-312-darwin.so \\
        --python-tag cp312 \\
        --platform aarch64-apple-darwin \\
        --out dist/
"""

import argparse
import base64
import csv
import glob as globmod
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
    "aarch64-pc-windows-msvc": "win_arm64",
}


def _file_hash(data: bytes) -> str:
    digest = hashlib.sha256(data).digest()
    return "sha256=" + base64.urlsafe_b64encode(digest).rstrip(b"=").decode("ascii")


def _read_version() -> str:
    for line in (PYTHON_ROOT / "pyproject.toml").read_text().splitlines():
        if line.startswith("version"):
            return line.split('"')[1]
    sys.exit("Could not find version in pyproject.toml")


def build_wheel(binary_path: Path, platform_tag: str, out_dir: Path,
                lib_path: Path | None = None,
                ext_path: Path | None = None,
                python_tag: str | None = None) -> Path:
    version = _read_version()
    name = "syntaqlite"
    is_windows = "win" in platform_tag
    binary_name = "syntaqlite.exe" if is_windows else "syntaqlite"

    # Determine wheel tags.
    if ext_path is not None:
        if python_tag is None:
            sys.exit("--python-tag is required when --ext is provided")
        # CPython extension wheel: cp312-cp312-{platform}
        # (CPython uses identical python and ABI tags since 3.2+)
        tag = f"{python_tag}-{python_tag}-{platform_tag}"
    else:
        # CLI-only wheel: py3-none-{platform}
        tag = f"py3-none-{platform_tag}"

    # Collect all files as (archive_path, content) pairs.
    files: dict[str, bytes] = {}

    # 1. Binary in {package}/bin/
    files[f"{name}/bin/{binary_name}"] = binary_path.read_bytes()

    # 1b. Shared library in {package}/lib/ (optional)
    if lib_path is not None:
        lib_name = lib_path.name
        files[f"{name}/lib/{lib_name}"] = lib_path.read_bytes()

    # 1c. C extension module (optional)
    if ext_path is not None:
        # Preserve the original filename (e.g. _syntaqlite.cpython-312-darwin.so)
        ext_name = ext_path.name
        files[f"{name}/{ext_name}"] = ext_path.read_bytes()

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
        f"Tag: {tag}\n"
    )
    files[f"{dist_info}/WHEEL"] = wheel_meta.encode()

    # 5. dist-info/entry_points.txt
    entry_points = (
        "[console_scripts]\n"
        f"syntaqlite = {name}:main\n"
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
    wheel_name = f"{name}-{version}-{tag}.whl"
    wheel_path = out_dir / wheel_name

    with zipfile.ZipFile(wheel_path, "w", zipfile.ZIP_DEFLATED) as whl:
        for arc_path, content in files.items():
            # Set executable permissions on binaries, libs, and extension modules
            needs_exec = "/bin/" in arc_path or "/lib/" in arc_path
            if not needs_exec and ext_path is not None:
                needs_exec = arc_path.endswith((".so", ".pyd", ".dylib"))
            if needs_exec:
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
    parser.add_argument(
        "--lib", type=Path, default=None,
        help="Path to shared library (libsyntaqlite.so/dylib/dll)"
    )
    parser.add_argument(
        "--ext", default=None,
        help="Glob pattern for compiled C extension (.so/.pyd)"
    )
    parser.add_argument(
        "--python-tag", default=None,
        help="CPython version tag (e.g. cp312). Required with --ext"
    )
    args = parser.parse_args()

    platform = PLATFORM_TAGS.get(args.platform, args.platform)

    if not args.binary.exists():
        sys.exit(f"Binary not found: {args.binary}")
    if args.lib and not args.lib.exists():
        sys.exit(f"Library not found: {args.lib}")

    # Resolve ext glob pattern to a single file.
    ext_path = None
    if args.ext:
        matches = globmod.glob(args.ext)
        if not matches:
            sys.exit(f"No files matching ext pattern: {args.ext}")
        if len(matches) > 1:
            sys.exit(f"Multiple files matching ext pattern: {matches}")
        ext_path = Path(matches[0])

    build_wheel(args.binary, platform, args.out,
                lib_path=args.lib, ext_path=ext_path,
                python_tag=args.python_tag)


if __name__ == "__main__":
    main()
