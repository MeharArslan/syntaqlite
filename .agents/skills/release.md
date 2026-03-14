# release

Bump version, commit, tag, and push a release. Use when the user asks to
"release", "bump version", "tag a release", or "publish".

## Instructions

1. **Determine the new version.** The user must provide it (e.g. `0.0.3`).

2. **Run the bump script:**
   ```sh
   python3 tools/bump-version <new_version> --check
   ```
   This updates all Cargo.toml files, pyproject.toml, CHANGELOG, README,
   docs, and lib.rs references. `--check` verifies `cargo check` passes.

3. **Commit:**
   ```sh
   git add -A
   git commit -m "$(cat <<'EOF'
   synq: bump version to <new_version>
   EOF
   )"
   ```

4. **Push and tag:**
   ```sh
   git push origin HEAD:main
   git tag v<new_version>
   git push origin v<new_version>
   ```

5. **Report** the tag and remind the user which GitHub Actions will fire:
   - `release.yml` — cargo-dist builds CLI binaries, Homebrew tap, installers
   - `publish-crates.yml` — publishes to crates.io
   - `vscode-extension.yml` — builds VS Code .vsix artifacts
