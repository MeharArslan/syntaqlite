// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! C ABI mirror structs for the dialect.
//!
//! All `#[repr(C)]` FFI types live in `syntaqlite_parser::dialect::ffi` and
//! are re-exported here for crate-internal use.

pub use syntaqlite_parser::dialect::ffi::*;
