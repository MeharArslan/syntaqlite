// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Parser generation, tokenizer/keyword assembly, base files, and amalgamation.

pub mod amalgamate;
pub mod base_files;
mod base_files_tables;
pub mod grammar_codegen;
pub mod keyword_hash;
#[cfg(feature = "codegen-pipeline")]
pub mod lemon;
#[cfg(feature = "codegen-pipeline")]
pub mod mkkeyword;
pub mod parser_pipeline;
pub mod sqlite_fragments;
pub mod tokenizer_assembly;
