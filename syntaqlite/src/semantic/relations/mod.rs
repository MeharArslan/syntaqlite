// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Relation catalog for semantic analysis.
//!
//! Provides [`RelationCatalog`] — a borrowed view over session and document
//! relation definitions for name resolution, column lookup, and fuzzy matching.

mod catalog;
mod types;

pub(crate) use catalog::RelationCatalog;
pub(crate) use types::{ColumnDef, RelationDef, RelationKind};
