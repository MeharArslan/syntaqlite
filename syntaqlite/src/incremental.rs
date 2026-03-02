// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Incremental (token-by-token) SQL parsing.

pub use crate::parser::typed::{IncrementalCursor, IncrementalParser};

#[cfg(feature = "sqlite")]
pub type SqliteIncrementalParser =
    IncrementalParser<'static, syntaqlite_parser_sqlite::SqliteNodeFamily>;
