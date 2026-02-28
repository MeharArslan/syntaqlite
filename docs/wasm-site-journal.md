# WASM Playground Journal

Date: 2026-02-20

## Entry 01

- Read `docs/plan.md` and confirmed V0 includes a WASM playground target.
- Audited `syntaqlite-cli/src/lib.rs` to locate AST, formatter, dialect codegen, and dynamic dialect loader flows.
- Audited runtime/dialect ABI files to understand feasibility constraints for browser-side extension loading.
- Created persistent task tracking documents:
  - `docs/wasm-site-plan.md`
  - `docs/wasm-site-journal.md`

## Entry 02

- Temporarily extracted source-level AST dump and formatter execution helpers.
- Rewired `syntaqlite-cli` AST/format paths to use helper functions.
- Kept native dynamic extension loading in CLI (not in shared crate), after confirming web cannot use `libloading`.

## Entry 03

- Added stable ABI exports and then migrated them to `syntaqlite` + `syntaqlite-runtime` layering.
- Added static web app in `web-playground/`:
  - `index.html`
  - `styles.css`
  - `app.js`
  - `README.md`
- Implemented engine selection (built-in vs uploaded extension), extension upload, optional dialect-name-based symbol prefix resolution, and output panels for formatted SQL + AST dump.
- Added build helper: `tools/build-web-playground`.

## Entry 04

- Removed temporary crates that added unnecessary surface area:
  - `syntaqlite-tools`
  - `syntaqlite-web-engine`
- Added runtime ABI implementation in `syntaqlite-runtime/src/abi.rs`.
- Added `abi` Cargo feature:
  - `syntaqlite-runtime`: `abi = ["fmt"]`
  - `syntaqlite`: `abi = ["fmt", "syntaqlite-runtime/abi"]`
- Added ABI symbol exports in `syntaqlite/src/abi.rs`:
  - default: `syntaqlite_abi_*`
  - named alias: `syntaqlite_sqlite_abi_*`
  - temporary compatibility aliases for prior `syntaqlite_web_*`
- Added C header contract for ABI in `syntaqlite-runtime/include/syntaqlite/abi.h`.
- Updated playground defaults and docs to use ABI prefixes and `--features abi` build mode.

## Entry 05

- Moved the ABI behind a dedicated feature flag per request:
  - `syntaqlite-runtime`: `abi`
  - `syntaqlite`: `abi`
- Retained native `syntaqlite-cli` dynamic extension loading as-is.
- Validation:
  - `cargo check -p syntaqlite -p syntaqlite-cli` passed
  - `cargo check -p syntaqlite --features abi` passed
- Build script attempt for `wasm32-unknown-unknown` currently fails because C toolchain headers are missing in this environment (`stdio.h`, `string.h` not found by clang for that target).

## Entry 06

- Switched playground build target from `wasm32-unknown-unknown` to `wasm32-unknown-emscripten`:
  - Updated `tools/build-web-playground`
  - Updated `web-playground/README.md`
- Added fallback artifact copy logic for either `syntaqlite.wasm` or `libsyntaqlite.wasm`.
- Build validation after switch failed due missing `emcc` in environment.
- Added explicit `emcc` preflight check and install guidance to the build script and README prerequisites.

## Entry 07

- Added `syntaqlite-wasm` wrapper crate with no-op `main()` plus ABI exports to satisfy emscripten executable linking expectations.
- Updated build script to target `syntaqlite-wasm` and write wasm output to `web-playground/syntaqlite.wasm`.
- Added `--rustflags "<...>"` passthrough argument to `tools/build-web-playground` so extra linker flags can be injected without overriding global `RUSTFLAGS`.

## Entry 08

- Updated `python/tools/build_web_playground.py` to add `-C link-arg=-g3` by default so emscripten preserves ABI export names in wasm.
- Updated `web-playground/app.js` with runtime import shims (`env`, `wasi_snapshot_preview1`, and compatibility module `a`) so direct `WebAssembly.instantiate` works for emscripten-built modules.
- Added explicit detection/error when module exports are minified and expected ABI names are unavailable.
- Verified:
  - `tools/build-web-playground` builds successfully and writes `web-playground/syntaqlite.wasm`.
  - wasm exports include `syntaqlite_abi_*` and `memory`.
  - ABI call flow works (`syntaqlite_abi_ast("select 1")` returns AST output).

## Entry 09

- Pivoted ABI placement so runtime/browser ABI is no longer in `syntaqlite` or `syntaqlite-runtime`.
- Removed runtime ABI files and features from core crates.
- Implemented ABI directly in `syntaqlite-wasm` with runtime exports:
  - `wasm_set_dialect`, `wasm_clear_dialect`
  - `wasm_alloc`, `wasm_free`
  - `wasm_ast`, `wasm_fmt`
  - `wasm_result_ptr`, `wasm_result_len`, `wasm_result_free`
- Kept `syntaqlite-wasm` dependency surface limited to `syntaqlite-runtime` only.

## Entry 10

- Reworked `web-playground/app.js` from old engine-prefix ABI loading to runtime+dialect composition:
  - Load runtime wasm (`syntaqlite.wasm`)
  - Upload dialect wasm and resolve `syntaqlite_dialect` or `syntaqlite_<name>_dialect`
  - Pass returned pointer into runtime via `wasm_set_dialect`
- Added runtime/dialect shared-memory + shared-table validation in JS to guard incompatible uploads.
- Updated UI copy (`web-playground/index.html`) and playground ABI documentation (`web-playground/README.md`) to reflect the new contract.

## Entry 11

- Added `syntaqlite/src/bin/wasm.rs` so the `syntaqlite` package can be built as a dialect wasm module for the playground.
- Exported `syntaqlite_dialect` from that wasm binary as an alias to `syntaqlite_sqlite_dialect`.
- Updated `python/tools/build_web_playground.py` to build both:
  - runtime wasm from `syntaqlite-wasm`
  - built-in SQLite dialect wasm from `syntaqlite --bin wasm`
- Updated playground startup flow to auto-load built-in dialect wasm (`syntaqlite-sqlite.wasm`) after runtime initialization.
- Upload flow now overrides built-in dialect; unloading restores built-in SQLite dialect automatically.

## Entry 12

- Moved small runtime helper implementations to headers to reduce standalone dialect build friction:
  - `synq_arena_*` moved into `include/syntaqlite_ext/arena.h` as `static inline`.
  - `synq_parse_ctx_*`, `synq_parse_build`, `synq_parse_list_append`, `synq_parse_list_flush` moved into `include/syntaqlite_ext/ast_builder.h` as `static inline`.
- Removed now-redundant runtime C split files:
  - deleted `syntaqlite-runtime/csrc/arena.c`
  - deleted `syntaqlite-runtime/csrc/parse_ctx.h`
- Updated `syntaqlite-runtime/csrc/parser.c` to use header-provided helpers and include `syntaqlite_ext/ast_builder.h` directly.
- Updated `syntaqlite-runtime/build.rs` to stop compiling `arena.c`.
- Updated `python/tools/build_web_playground.py` dialect wasm compilation to use dialect-only C inputs again (plus tiny shim), no runtime C objects required.

## Entry 13

- Added `tools/run-web-playground` helper to build and serve the static playground:
  - builds with `tools/build-web-playground` by default
  - serves `web-playground/` via `python -m http.server`
  - supports `--port` and `--skip-build`

## Entry 14

- Switched playground runtime loading to Emscripten's loader path end-to-end:
  - load `syntaqlite-runtime.js` and runtime wasm as the main module
  - use `loadDynamicLibrary` for built-in/uploaded dialect wasm modules
- Fixed dynamic-library glue bug in `web-playground/app.js`:
  - `locateFile` now remaps only the runtime wasm filename, not every `.wasm` path
  - prevents dialect URLs from being accidentally redirected to runtime wasm
- Fixed dialect symbol resolution in `web-playground/app.js`:
  - resolve dialect exports from `localScope` passed to `loadDynamicLibrary(..., { global: false }, localScope)`
  - keep module-level fallback lookup for compatibility
- Fixed heap access in `web-playground/app.js` for non-modularized emscripten output:
  - read/write through `Module.HEAPU8 || globalThis.HEAPU8`
- Validation:
  - `tools/build-web-playground` succeeds
  - `node --check web-playground/app.js` succeeds
  - runtime+side-module smoke test passes:
    - `loadDynamicLibrary` loads `syntaqlite-sqlite.wasm`
    - `syntaqlite_sqlite_dialect()` pointer is set via `wasm_set_dialect`
    - `wasm_ast("select 1;")` returns AST (`SelectStmt` first line)
- Updated `web-playground/README.md` to document current artifacts and loader model.

## Entry 15

- Refreshed `web-playground/styles.css` to a minimal, more elegant visual system:
  - simplified palette (neutral surfaces, restrained accent)
  - reduced visual noise (lighter gradients, softer shadows, clearer borders)
  - improved type hierarchy and spacing for controls/output readability
  - cleaner button/input styling with consistent focus states
- Kept layout and interaction behavior unchanged (`index.html` + `app.js` logic untouched).
- Added subtle one-time load animation (`settle`) on key sections for a polished feel without constant motion.
- Preserved responsive behavior for mobile by keeping single-column collapse for controls and output panels.

## Entry 16

- Reworked the playground into a workspace-first two-pane layout:
  - compact top control bar for dialect loading + formatting options + run actions
  - full-height split below with editor pane (left) and visualizer pane (right)
- Updated `web-playground/index.html` structure to:
  - move most controls out of the content area and into `topbar`
  - dedicate main viewport area to `workspace` with `editor-pane` and `viewer-pane`
- Replaced `web-playground/styles.css` to enforce full-height usage:
  - `app` uses `min-height: 100vh` and `grid-template-rows: auto 1fr`
  - `workspace` uses a 2-column split with `minmax(0, 1fr)` columns
  - panes and output blocks use `min-height: 0` + grid rows so textareas/previews fill available space
- Kept all runtime functionality and element IDs unchanged so existing `app.js` bindings continue to work.

## Entry 17

- Added a dedicated Perfetto dialect wasm build script:
  - `python/tools/build_perfetto_wasm.py`
  - `tools/build-perfetto-wasm` wrapper
- Script flow:
  - generates Perfetto amalgamated C via `syntaqlite-cli dialect --name perfetto ... csrc`
  - writes shim headers (`syntaqlite_runtime.h`, `syntaqlite_ext.h`) into the generated csrc dir
  - compiles `syntaqlite_perfetto.c` with `emcc` as a side module (`-sSIDE_MODULE=1`)
- Default output is `web-playground/syntaqlite-perfetto.wasm`.
- Validation:
  - `tools/build-perfetto-wasm` succeeds
  - resulting module exports `syntaqlite_perfetto_dialect` and `syntaqlite_dialect`
  - runtime+dialect smoke test passes for:
    - `CREATE PERFETTO TABLE foo AS select a, b from t where c = 1`
    - AST root `CreatePerfettoTableStmt`
- Updated `web-playground/README.md` with Perfetto build instructions.

## Entry 18

- Updated `.gitignore` to ignore generated web-playground artifacts:
  - `web-playground/syntaqlite-runtime.js`
  - `web-playground/syntaqlite-*.wasm`
  - `web-playground/syntaqlite.wasm`
- Kept source files in `web-playground/` (HTML/CSS/JS/README) unaffected and trackable.
