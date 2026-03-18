# cibuildwheel before-all hook for Windows.
# Builds the Rust static library + CLI binary, handling ARM64 cross-compilation.
$ErrorActionPreference = "Stop"

$RepoRoot = (git rev-parse --show-toplevel) | Out-String
$RepoRoot = $RepoRoot.Trim()
Set-Location $RepoRoot

$Arch = $env:CIBW_ARCHS
if ($Arch -eq "ARM64") {
    $Target = "aarch64-pc-windows-msvc"
    rustup target add $Target
    cargo build --release -p syntaqlite -p syntaqlite-cli --target $Target
    $BinaryDir = "target\$Target\release"
} else {
    cargo build --release -p syntaqlite -p syntaqlite-cli
    $BinaryDir = "target\release"
}

# Copy CLI binary into the package so setuptools includes it.
New-Item -ItemType Directory -Force -Path "python\syntaqlite\bin" | Out-Null
Copy-Item "$BinaryDir\syntaqlite.exe" "python\syntaqlite\bin\syntaqlite.exe"
