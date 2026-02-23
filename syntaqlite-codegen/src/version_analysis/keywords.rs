// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Keyword table parsing from `mkkeywordhash.c`.
//!
//! Re-exports from `crate::mkkeywordhash_parser` — the parser is available
//! without the `version-analysis` feature gate.

pub use crate::mkkeywordhash_parser::{
    KeywordEntry, KeywordTable, MaskDefine, parse_keyword_table,
};
