// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Semantic analysis types for SQL tooling.
//!
//! This module provides types and catalogs for semantic analysis of SQL:
//! - [`functions`] — Function catalog with name/arity checking and fuzzy matching.
//! - [`relations`] — Relation catalog with name resolution and column lookup.

pub mod functions;
pub mod relations;
