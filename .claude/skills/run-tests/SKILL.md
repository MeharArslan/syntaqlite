---
name: run-tests
description: Run tests to verify correctness after code changes. Use when the user asks to run tests, verify changes, or check that things still work. Covers Rust unit tests, AST diff tests, formatter diff tests, Perfetto dialect formatter tests, Perfetto validation diff tests, and amalgamation integration tests.
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
   tools/run-unit-tests
   ```

3. **Run diff test suites** based on what changed:
   - `.synq` files or codegen changes → run AST, fmt, and Perfetto fmt diff tests
   - `src/fmt/` changes → `tools/run-fmt-diff-tests`
   - `src/sqlite/` or parser changes → `tools/run-ast-diff-tests`
   - `src/validation/` changes → `tools/run-perfetto-validation-diff-tests`
   - `dialects/perfetto/` changes → `tools/run-perfetto-fmt-diff-tests`, `tools/run-perfetto-validation-diff-tests`, and `tools/run-amalg-tests`
   - When unclear → run all diff test suites

   ```sh
   tools/run-ast-diff-tests
   tools/run-fmt-diff-tests
   tools/run-perfetto-fmt-diff-tests
   tools/run-perfetto-validation-diff-tests
   tools/run-amalg-tests
   ```

4. **Useful flags**:
   - `--filter <pattern>` — run only tests matching pattern
   - `-v` — verbose output
   - `--rebaseline` — update expected outputs to match current output
   - `-j <N>` — parallel test execution

5. **Report results** to the user, including any failures with relevant output.
