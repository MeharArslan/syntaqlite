// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Raw FFI bindings for the syntaqlite C parser engine.
//!
//! This crate compiles the C parser/tokenizer (via `build.rs`) and exports
//! the Rust-side `#[repr(C)]` mirror types and `extern "C"` declarations.

pub mod dialect;
pub mod parser;
