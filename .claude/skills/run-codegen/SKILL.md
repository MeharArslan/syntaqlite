---
name: run-codegen
description: Regenerate all generated code from .synq definitions and SQLite grammar. Use when .synq files, grammar actions, or dialect definitions have been modified and generated C/Rust code needs updating.
user_invocable: true
---

# run-codegen

Regenerate all generated code from `.synq` definitions and SQLite grammar.

## Instructions

1. **Run codegen**:
   ```sh
   tools/run-codegen
   ```

   This regenerates:
   - C headers in `syntaqlite-parser-sys/csrc/`
   - Rust node types, token types, and fmt bytecode in `syntaqlite/src/generated/`

2. **Verify the result**:
   ```sh
   cargo check && cargo clippy
   ```

   Both must pass with zero warnings.

3. **Report results** to the user, including any errors or warnings.
