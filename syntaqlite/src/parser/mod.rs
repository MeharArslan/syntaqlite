// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Grammar-agnostic parser infrastructure.
//!
//! This module contains the core parsing machinery shared by all dialects:
//! tokenization, incremental token feeding, session management, and the
//! arena-allocated node representation. Most users should use the typed
//! wrappers ([`crate::Parser`], [`crate::ast`]) rather than these
//! internals directly.

#[cfg(feature = "json")]
pub mod node_ref_json;
