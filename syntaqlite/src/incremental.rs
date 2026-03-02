// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Incremental (token-by-token) SQL parsing.

#[cfg(feature = "sqlite")]
pub use crate::sqlite_api::{IncrementalCursor, IncrementalParser};

pub use syntaqlite_parser::{Comment, MacroRegion};
