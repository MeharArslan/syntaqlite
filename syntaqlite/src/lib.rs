// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub mod ast_traits;
pub mod parser;

pub use parser::{NodeId, NodeReader, ParseError, Parser, StatementCursor};

#[cfg(feature = "fmt")]
pub mod fmt;

pub mod catalog;
pub mod dialect;

pub use dialect::Dialect;

#[cfg(feature = "validation")]
pub mod validation;

#[cfg(feature = "lsp")]
pub mod lsp;

#[cfg(feature = "sqlite")]
pub mod sqlite;
