// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// ── Public API ───────────────────────────────────────────────────────────────

/// Metadata for a single compile-time flag.
#[derive(Debug, Clone)]
pub struct CflagInfo {
    /// The suffix shared across C and Rust (e.g. `"SQLITE_OMIT_WINDOWFUNC"`).
    pub suffix: String,
    /// Bit index in [`Cflags`] (matches `SYNQ_CFLAG_IDX_*` constants).
    pub index: u32,
    /// Minimum SQLite version (encoded integer) at which this cflag has any
    /// observable effect on keyword recognition. Zero means baseline (all versions).
    pub min_version: i32,
    /// Feature-area category for UI grouping:
    /// `"parser"`, `"functions"`, `"vtable"`, or `"extensions"`.
    pub category: String,
}

// ── Crate-internal ───────────────────────────────────────────────────────────

pub(crate) use ffi::CCflags as Cflags;

// ── ffi ───────────────────────────────────────────────────────────────────────

mod ffi {
    /// Mirrors C `SyntaqliteCflags` from `include/syntaqlite/cflags.h`.
    ///
    /// A packed bitfield struct. On the Rust side we represent it as raw bytes
    /// and provide index-based accessors matching the C `SYNQ_CFLAG_IDX_*` constants.
    ///
    /// Only parser-group cflags are stored here (22 flags, group-local indices 0–21,
    /// packed into 3 bytes). This mirrors the generated `cflags.h` which is filtered
    /// to the "parser" group.
    #[repr(C)]
    #[derive(Clone, Copy)]
    pub(crate) struct CCflags {
        pub(super) bytes: [u8; 3],
    }

    impl CCflags {
        /// Create a zero-initialized (all flags off) cflags.
        pub(crate) const fn new() -> Self {
            Self { bytes: [0; 3] }
        }

        /// Check if cflag at `idx` is set.
        #[inline]
        pub(crate) fn has(&self, idx: u32) -> bool {
            let byte = idx / 8;
            let bit = idx % 8;
            (byte < 3) && (self.bytes[byte as usize] >> bit) & 1 != 0
        }

        /// Set cflag at `idx`.
        #[inline]
        pub(crate) fn set(&mut self, idx: u32) {
            let byte = idx / 8;
            let bit = idx % 8;
            if byte < 3 {
                self.bytes[byte as usize] |= 1 << bit;
            }
        }

        /// Clear cflag at `idx`.
        #[inline]
        pub(crate) fn clear(&mut self, idx: u32) {
            let byte = idx / 8;
            let bit = idx % 8;
            if byte < 3 {
                self.bytes[byte as usize] &= !(1 << bit);
            }
        }

        /// Reset all cflags to zero.
        #[inline]
        pub(crate) fn clear_all(&mut self) {
            self.bytes = [0; 3];
        }
    }

    impl Default for CCflags {
        fn default() -> Self {
            Self::new()
        }
    }

    impl std::fmt::Debug for CCflags {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            // 3 bytes, each shown as 2 hex digits.
            write!(f, "Cflags({:02x?})", &self.bytes)
        }
    }
}
