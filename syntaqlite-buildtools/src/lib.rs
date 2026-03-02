// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![warn(unreachable_pub)]

// Shared modules (needed by both stage 1 and stage 2/3).
pub mod util;

// Stage 1: extraction from raw SQLite source (feature-gated).
#[cfg(feature = "sqlite-extract")]
pub mod extract;

// Stage 2/3: codegen pipeline (feature-gated).
#[cfg(feature = "codegen-pipeline")]
pub mod dialect_codegen;

#[cfg(feature = "codegen-pipeline")]
pub(crate) mod parser_tools;

// Re-export parser_tools submodules at crate root for API stability.
#[cfg(feature = "codegen-pipeline")]
pub use parser_tools::{amalgamate, base_files};

// Output resolver: OutputLayout + write_codegen_artifacts.
#[cfg(feature = "codegen-pipeline")]
pub mod output_resolver;

#[cfg(feature = "version-analysis")]
pub mod version_analysis;

#[cfg(all(test, feature = "grammar-verify"))]
mod grammar_verify;

// --- Codegen pipeline types and functions (stage 2/3) ---

#[cfg(feature = "codegen-pipeline")]
pub mod codegen_api;

// --- SQLite-specific API (stage 2/3, sqlite-codegen only) ---

// Lemon and mkkeyword subprocess entry points — needed by any binary that
// calls generate_codegen_artifacts(), which spawns `current_exe() lemon/mkkeyword`.
#[cfg(feature = "codegen-pipeline")]
pub fn run_lemon(args: &[String]) -> ! {
    parser_tools::lemon::run_lemon(args)
}

#[cfg(feature = "codegen-pipeline")]
pub fn run_mkkeyword(args: &[String]) -> ! {
    parser_tools::mkkeyword::run_mkkeyword(args)
}
