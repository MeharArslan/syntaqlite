// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Raw FFI bindings and dialect-agnostic arena infrastructure for the
//! syntaqlite C parser engine.
//!
//! This crate compiles the C parser/tokenizer (via `build.rs`) and exports
//! the Rust-side `#[repr(C)]` mirror types, `extern "C"` declarations,
//! and the grammar-agnostic arena / session machinery.

pub mod ast_traits;
pub mod catalog;
pub mod dialect;
pub mod nodes;
pub mod parser;
pub mod session;
pub mod typed_list;

pub mod cflag_versions;
pub mod functions_catalog;
pub mod sqlite;
