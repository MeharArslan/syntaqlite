# Semantic Legacy

This directory contains the pre-refactor semantic validation pipeline
(`analyzer`, `walker`, `scope`, `checks`, `fuzzy`, `model`, `render`).

It is intentionally parked during the current refactor and is not part of the
active `semantic` module graph.

Active semantic surface today lives in:
- `semantic/mod.rs`
- `semantic/diagnostics.rs`
- `semantic/schema.rs` (for `embedded`/`lsp` feature paths)
