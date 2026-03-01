// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Parser and tokenizer FFI declarations.
//!
//! All `#[repr(C)]` types and `extern "C"` functions live in
//! `syntaqlite_parser_sys::parser` and are re-exported here for
//! crate-internal use.

pub use syntaqlite_parser_sys::parser::*;
