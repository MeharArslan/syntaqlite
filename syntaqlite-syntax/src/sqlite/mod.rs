// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// `tokens` is always available — SQLite token type ordinals are stable and
// grammar-agnostic; callers use them without the full sqlite feature.
pub(crate) mod tokens;

#[cfg(feature = "sqlite")]
pub(crate) mod ast;
// `cflags` is always available — `SqliteSyntaxFlag` ordinals are stable and
// grammar-agnostic; callers use them without the full sqlite feature.
pub(crate) mod cflags;
#[cfg(feature = "sqlite")]
pub(crate) mod ffi;
#[cfg(feature = "sqlite")]
pub(crate) mod grammar;
