// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Shared primitives used by both the runtime (`syntaqlite`) and the
//! build-time codegen pipeline (`syntaqlite-buildtools`).
//!
//! This crate has no dependencies and no generated code, making it safe
//! to depend on from the bootstrap codegen tool without pulling in any
//! generated-file requirements.

pub mod fmt;
