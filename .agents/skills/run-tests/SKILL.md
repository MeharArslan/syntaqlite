---
name: run-tests
description: Run tests to verify correctness after code changes. Use when the user asks to run tests, verify changes, or check that things still work. NOTE: Only run Rust unit tests (tools/run-unit-tests). The diff/integration test scripts are currently broken.
user_invocable: true
---

# run-tests

Run tests to verify correctness after code changes.

> **WARNING**: The following diff/integration test scripts are currently broken — do NOT run them. `tools/run-unit-tests` is fine.

## Instructions

1. **Run Rust unit tests**:
   ```sh
   tools/run-unit-tests
   ```

2. **Report results** to the user, including any failures with relevant output.

## Do NOT run (currently broken)

The following test scripts are broken and must be skipped until further notice:

- `tools/run-ast-diff-tests`
- `tools/run-fmt-diff-tests`
- `tools/run-perfetto-fmt-diff-tests`
- `tools/run-perfetto-validation-diff-tests`
- `tools/run-amalg-tests`
