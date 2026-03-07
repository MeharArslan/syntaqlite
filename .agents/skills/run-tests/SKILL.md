---
name: run-tests
description: Run tests to verify correctness after code changes. Use when the user asks to run tests, verify changes, or check that things still work. Runs Rust unit tests and integration tests.
user_invocable: true
---

# run-tests

Run tests to verify correctness after code changes.

## Instructions

1. **Run Rust unit tests**:
   ```sh
   tools/run-unit-tests
   ```

2. **Run integration tests**:
   ```sh
   tools/run-integration-tests
   ```
   You can also run a specific suite:
   ```sh
   tools/run-integration-tests --suite ast
   tools/run-integration-tests --suite fmt
   tools/run-integration-tests --suite perfetto-fmt
   tools/run-integration-tests --suite perfetto-val
   tools/run-integration-tests --suite amalg
   tools/run-integration-tests --suite grammar
   ```
   Use `tools/run-integration-tests --list` to see all available suites.

3. **Report results** to the user, including any failures with relevant output.
