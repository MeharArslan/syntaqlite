// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Stage 1: Extract C fragments from raw SQLite source.
//!
//! This module is only compiled when the `sqlite-extract` feature is enabled.
//! It reads raw SQLite source files (tokenize.c, global.c, sqliteInt.h,
//! mkkeywordhash.c) and produces the committed fragment files in
//! `data/sqlite_fragments/`.

pub mod base_files;
pub mod keywords;
pub mod mkkeywordhash;
pub mod tokenizer;
