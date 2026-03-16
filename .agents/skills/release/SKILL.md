# release

Bump version, commit, tag, and create a draft GitHub Release. Use when the user
asks to "release", "bump version", "tag a release", or "publish".

## Instructions

1. **Determine the new version.** Read the current version from
   `python/pyproject.toml` and increment the patch number automatically.
   If the user specifies a version explicitly, use that instead.

2. **Write the CHANGELOG.** Read `CHANGELOG.md` and replace the
   `*No changes yet.*` placeholder for the **current** version (the one
   about to be released) with real entries. Use `git log` to find commits
   since the previous release tag.

   Focus on **user-visible changes only**:
   - New features, new CLI flags, new API surface
   - Bug fixes that affected users
   - Breaking changes or renamed options
   - Documentation improvements (briefly)

   Do NOT include:
   - Internal refactors, code cleanup, clippy fixes
   - CI/release pipeline plumbing
   - Changes to dev tooling or build scripts

   Keep entries concise — one line per change, no sub-bullets.

3. **Run the bump script:**
   ```sh
   python3 tools/bump-version <new_version> --check
   ```
   This updates all Cargo.toml files, pyproject.toml, CHANGELOG, README,
   docs, and lib.rs references. `--check` verifies `cargo check` passes.

4. **Commit and push:**
   ```sh
   git add -A
   git commit -m "$(cat <<'EOF'
   synq: bump version to <new_version>
   EOF
   )"
   git push origin HEAD:main
   ```

5. **Create the tag (don't push yet):**
   ```sh
   git tag v<new_version>
   ```

6. **Create a draft GitHub Release.** Use the CHANGELOG entries as the
   release notes body. Focus on user-visible changes — same rules as the
   CHANGELOG. Ask the user if they want to edit the notes.
   ```sh
   gh release create v<new_version> --draft --title "v<new_version>" \
     --notes "$(release notes here)"
   ```

7. **Push the tag** to trigger build workflows:
   ```sh
   git push origin v<new_version>
   ```
   Build workflows upload artifacts to the draft release:
   - `release.yml` — cargo-dist builds CLI binaries, Homebrew tap, installers
   - `publish-crates.yml` — publishes to crates.io
   - `vscode-extension.yml` — builds VS Code .vsix artifacts
   - `release-amalgamation.yml` — C source amalgamation archive
   - `release-clib.yml` — prebuilt C shared library archive

8. **Report** the draft release URL to the user and remind them to publish
   the release manually once the builds complete and look good.
