// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub mod parser;

pub use parser::{NodeId, NodeReader, ParseError, Parser, StatementCursor};

#[cfg(feature = "fmt")]
pub mod fmt;

pub mod dialect;

pub use dialect::Dialect;