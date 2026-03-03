// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Shared modules (needed by both stage 1 and stage 2/3).
pub mod util;

// Stage 1: extraction from raw SQLite source.
pub mod extract;

// Stage 2/3: codegen pipeline.
pub mod dialect_codegen;

pub(crate) mod parser_tools;

// Re-export parser_tools submodules at crate root for API stability.
pub use parser_tools::{amalgamate, base_files};

// Output resolver: OutputLayout + write_codegen_artifacts.
pub mod output_resolver;

pub mod version_analysis;

#[cfg(test)]
mod grammar_verify;

// --- Codegen pipeline types and functions (stage 2/3) ---

pub mod codegen_api;

// Lemon and mkkeyword subprocess entry points — needed by any binary that
// calls generate_codegen_artifacts(), which spawns `current_exe() lemon/mkkeyword`.
pub fn run_lemon(args: &[String]) -> ! {
    parser_tools::lemon::run_lemon(args)
}

pub fn run_mkkeyword(args: &[String]) -> ! {
    parser_tools::mkkeyword::run_mkkeyword(args)
}
