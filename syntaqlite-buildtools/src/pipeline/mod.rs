// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Stage 2/3: codegen pipeline — parser generation, tokenizer/keyword assembly,
//! dialect codegen, and amalgamation.

pub mod amalgamate;
pub mod base_files;
pub mod c_writer;
pub mod dialect_codegen;
pub mod fmt_compiler;
pub mod grammar_codegen;
pub mod keyword_hash;
pub(crate) mod lemon;
pub(crate) mod mkkeyword;
pub mod output_manifest;
pub mod parser_pipeline;
pub mod rust_writer;
pub mod sqlite_fragments;
pub mod text_writer;
pub(crate) mod tool_run;
pub mod tokenizer_assembly;
