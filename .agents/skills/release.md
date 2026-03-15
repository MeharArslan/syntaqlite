# release

Bump version, commit, tag, and create a draft GitHub Release. Use when the user
asks to "release", "bump version", "tag a release", or "publish".

## Instructions

1. **Determine the new version.** The user must provide it (e.g. `0.0.3`).

2. **Run the bump script:**
   ```sh
   python3 tools/bump-version <new_version> --check
   ```
   This updates all Cargo.toml files, pyproject.toml, CHANGELOG, README,
   docs, and lib.rs references. `--check` verifies `cargo check` passes.

3. **Commit and push:**
   ```sh
   git add -A
   git commit -m "$(cat <<'EOF'
   synq: bump version to <new_version>
   EOF
   )"
   git push origin HEAD:main
   ```

4. **Create the tag (don't push yet):**
   ```sh
   git tag v<new_version>
   ```

5. **Create a draft GitHub Release.** Read CHANGELOG.md to get the release
   notes for this version. Ask the user if they want to edit the notes or
   if the CHANGELOG content is sufficient.
   ```sh
   gh release create v<new_version> --draft --title "v<new_version>" \
     --notes "$(release notes here)"
   ```

6. **Push the tag** to trigger build workflows:
   ```sh
   git push origin v<new_version>
   ```
   Build workflows upload artifacts to the draft release:
   - `release.yml` — cargo-dist builds CLI binaries, Homebrew tap, installers
   - `publish-crates.yml` — publishes to crates.io
   - `vscode-extension.yml` — builds VS Code .vsix artifacts
   - `release-amalgamation.yml` — C amalgamation archive

7. **Report** the draft release URL to the user and remind them to publish
   the release manually once the builds complete and look good.
