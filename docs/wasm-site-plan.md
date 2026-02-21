# WASM Playground Implementation Plan

Status: Completed (V0)
Last updated: 2026-02-20

## Goal

Build a static WASM playground UI for:
- Formatting SQL
- Displaying AST dumps
- Loading extension `.wasm` modules in a way analogous to CLI extension symbol loading

## Milestones

1. Tracking
- [x] Create persistent plan and journal files
- [x] Keep this file and journal updated as work lands

2. Shared logic refactor
- [x] Keep CLI AST/format logic minimal and local
- [x] Rewire/refine `syntaqlite-cli` without adding extra crates
- [x] Keep dynamic symbol naming conventions reusable (default + named)

3. WASM engine ABI
- [x] Move ABI ownership fully into `syntaqlite-wasm`
- [x] Keep `syntaqlite-wasm` independent of `syntaqlite` crate
- [x] Export runtime entrypoints as `wasm_*`
- [x] Add runtime dialect pointer setter (`wasm_set_dialect`) for JS glue

4. Static site
- [x] Add static web assets (`index.html`, `app.js`, `styles.css`)
- [x] Implement SQL input + AST output + formatter output UX
- [x] Implement dialect wasm upload and symbol-name-based binding
- [x] Bind uploaded dialect pointer into runtime via `wasm_set_dialect`

5. Tooling and validation
- [x] Add build instructions/tooling for generating web wasm asset
- [x] Validate with `cargo check` for changed crates
- [x] Document extension wasm ABI contract
- [x] Build and include a built-in SQLite dialect wasm from `syntaqlite`
- [x] Switch playground loading to Emscripten main-module + side-module loader path
- [x] Validate runtime+dialect smoke flow (`loadDynamicLibrary` + `wasm_set_dialect` + `wasm_ast`)

## Open Follow-up

- Add a first-class dialect codegen command that scaffolds a dialect-wasm artifact for generated dialect crates so browser upload is one command.

## Notes

- CLI today loads native dynamic libraries and resolves `syntaqlite_dialect` or `syntaqlite_<name>_dialect`.
- In browser, direct native dynamic loading is unavailable; JS glue resolves the dialect symbol in a dialect wasm module and passes its pointer to runtime via `wasm_set_dialect`.
