---
description: Run tests to verify correctness after code changes
user_invocable: true
---

# run-tests

Run tests to verify correctness after code changes.

## Instructions

1. **Build the CLI binary first** (needed for diff tests):
   ```sh
   cargo build -p syntaqlite-cli
   ```

2. **Run Rust unit tests**:
   ```sh
   cargo nextest run --workspace
   ```

3. **Run diff test suites** based on what changed:
   - `.synq` files or codegen changes → run both AST and fmt diff tests
   - `src/fmt/` changes → `tools/run-fmt-diff-tests`
   - `src/sqlite/` or parser changes → `tools/run-ast-diff-tests`
   - When unclear → run both AST and fmt diff tests

   ```sh
   tools/run-ast-diff-tests
   tools/run-fmt-diff-tests
   ```

4. **Useful flags**:
   - `--filter <pattern>` — run only tests matching pattern
   - `-v` — verbose output
   - `--rebaseline` — update expected outputs to match current output
   - `-j <N>` — parallel test execution

5. **Report results** to the user, including any failures with relevant output.
