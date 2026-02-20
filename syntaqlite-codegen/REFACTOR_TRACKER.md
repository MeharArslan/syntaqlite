# syntaqlite-codegen Refactor Tracker

## Plan
1. Establish tracking and execute changes in small, compile-safe slices.
2. Prioritize architecture-level reductions in coupling and module size.
3. Apply DRY improvements only after module boundaries are clearer.
4. Keep public API compatibility where practical by adding `try_*` APIs before changing existing signatures.
5. Run `cargo check`/`cargo test` at each major milestone.

## TODO
- [x] A1: Split `lib.rs` orchestration from transformation internals into focused modules.
- [x] A2: Add a library-level output artifact manifest and reduce filename/path wiring in CLI.
- [x] A3: Further split `ast_codegen` responsibilities beyond current `c_codegen`/`rust_codegen`.
- [x] A4: Replace panic-driven compile/codegen paths with typed errors (`fmt_compiler`, C meta/fmt generation paths).
- [x] A5: Simplify parser pipeline by removing duplicate grammar writes/artifacts unless explicitly needed.
- [x] A6: Strengthen amalgamation include parsing/stripping logic.
- [x] D1: Precompute reusable item partitions in `AstModel` and reuse them across emitters.
- [x] D2: Unify duplicated C/Rust writer indentation core.
- [x] D3: Extract reusable `RawOp` constructors/helpers in `fmt_compiler`.
- [x] D4: Break up static Rust wrapper/lib generation into templated section emitters.
- [x] D5: Auto-generate embedded base file tables (instead of manually maintained lists).

## Journal
- 2026-02-20: Tracker created; starting sequential execution from A1.
- 2026-02-20: Completed A1 by extracting `grammar_codegen`, `parser_pipeline`, `sqlite_codegen`, `codegen_pipeline`, and shared `subprocess` helper modules; `lib.rs` now delegates.
- 2026-02-20: Completed A2 by adding `output_manifest` (`OutputBucket`/`OutputArtifact`) and switching CLI codegen output writing to manifest-driven routing.
- 2026-02-20: Completed A3 by splitting `ast_codegen` into `c_ast`, `c_dialect`, `rust_ast`, and `rust_dialect` modules.
- 2026-02-20: Completed D1 by adding precomputed `AstModel` partitions (`enums`, `flags`, `nodes`, `lists`, `node_like_items`) and rewiring major emit loops to use them.
- 2026-02-20: Completed D4 by converting rust dialect/lib wrapper generation to section-template constants and small emit helpers.
- 2026-02-20: Completed A5 by removing duplicate `parse_raw.y` writes and using a single `parse.y` artifact for grammar extraction + Lemon generation.
- 2026-02-20: Completed A6 by hardening include parsing (`# include` support, explicit include classification) and selective quoted-include stripping in amalgamation.
- 2026-02-20: Completed D2 by extracting shared indentation/buffer state into `text_writer::TextWriterCore` and refactoring both `CWriter` and `RustWriter` to use it.
- 2026-02-20: Completed D5 by generating base `.y`/`.synq` file tables in `build.rs` and switching `base_files.rs` to generated constants.
- 2026-02-20: Completed A4 by adding typed C codegen errors (`CCodegenError`) and switching the main codegen pipeline to typed `try_*` C meta/fmt paths.
- 2026-02-20: Completed D3 by standardizing `fmt_compiler` op construction through `op0`/`opa`/`opab`/`opabc`.
- 2026-02-20: Validation milestone: `cargo check` (workspace) and `cargo test -p syntaqlite-codegen` both pass.
