// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! SQLite dialect: typed AST, token types, and built-in functions.
//!
//! This module provides the concrete SQLite binding that powers the
//! crate-root [`Parser`](crate::Parser), [`Formatter`](crate::Formatter),
//! and [`Validator`](crate::Validator).

#[cfg(feature = "sqlite")]
pub(crate) mod wrappers;

#[cfg(feature = "sqlite")]
pub(crate) use syntaqlite_parser_sqlite::dialect;
