// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Parser generation, tokenizer/keyword assembly, base files, and amalgamation.

pub mod amalgamate;
pub mod base_files;
pub(crate) mod grammar_codegen;
pub(crate) mod keyword_hash;
#[allow(dead_code)]
pub(crate) mod lemon;
#[allow(dead_code)]
pub(crate) mod mkkeyword;
pub(crate) mod parser_pipeline;
pub(crate) mod sqlite_fragments;
pub(crate) mod tokenizer_assembly;
