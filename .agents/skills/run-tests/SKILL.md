---
name: run-tests
description: Run tests to verify correctness after code changes. Use when the user asks to run tests, verify changes, or check that things still work. NOTE: All tools/ integration tests are currently broken — only run Rust unit tests.
user_invocable: true
---

# run-tests

Run tests to verify correctness after code changes.

> **WARNING**: All `tools/` integration test scripts are currently broken (AST diff tests, fmt diff tests, Perfetto tests, amalgamation tests). Do NOT run them. Only run Rust unit tests.

## Instructions

1. **Run Rust unit tests**:
   ```sh
   tools/run-unit-tests
   ```

2. **Report results** to the user, including any failures with relevant output.

## Do NOT run (currently broken)

The following are broken and must be skipped until further notice:

- `cargo build -p syntaqlite-cli`
- `tools/run-ast-diff-tests`
- `tools/run-fmt-diff-tests`
- `tools/run-perfetto-fmt-diff-tests`
- `tools/run-perfetto-validation-diff-tests`
- `tools/run-amalg-tests`
