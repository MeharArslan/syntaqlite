// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Maps byte offsets in processed SQL text back to host-file byte offsets.
//!
//! When interpolation holes are replaced with placeholder identifiers, the
//! offsets shift because `{some_long_expr}` might become `__hole_0__` (or
//! vice-versa). The `OffsetMap` handles this translation.

use super::EmbeddedFragment;

/// An entry in the offset map: a region where the SQL text and host text differ.
#[derive(Debug)]
struct Segment {
    /// Start offset in SQL text.
    sql_start: usize,
    /// Length of this segment in SQL text.
    sql_len: usize,
    /// Start offset in host file.
    host_start: usize,
    /// Length of this segment in host file.
    host_len: usize,
}

/// Maps byte offsets from processed SQL text back to host-file positions.
pub struct OffsetMap {
    /// Base offset of the SQL content in the host file.
    base_offset: usize,
    /// Sorted segments where SQL and host offsets diverge (holes).
    segments: Vec<Segment>,
}

impl OffsetMap {
    /// Build an offset map from an `EmbeddedFragment`.
    pub fn new(fragment: &EmbeddedFragment) -> Self {
        let segments = fragment
            .holes
            .iter()
            .map(|h| Segment {
                sql_start: h.sql_offset,
                sql_len: h.placeholder.len(),
                host_start: h.host_range.start,
                host_len: h.host_range.len(),
            })
            .collect();

        OffsetMap {
            base_offset: fragment.sql_range.start,
            segments,
        }
    }

    /// Convert a SQL-text byte offset to a host-file byte offset.
    pub fn to_host(&self, sql_offset: usize) -> usize {
        // Walk through segments to compute the cumulative drift.
        let mut drift: isize = 0;

        for seg in &self.segments {
            if sql_offset < seg.sql_start {
                break;
            }
            if sql_offset < seg.sql_start + seg.sql_len {
                // Inside a hole — clamp to the hole's host range.
                return seg.host_start;
            }
            // Past this hole: accumulate the difference in lengths.
            drift += seg.host_len as isize - seg.sql_len as isize;
        }

        // Apply base offset and accumulated drift.
        ((sql_offset as isize) + (self.base_offset as isize) + drift) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::embedded::{EmbeddedFragment, Hole};

    #[test]
    fn identity_map_no_holes() {
        let fragment = EmbeddedFragment {
            sql_range: 10..30,
            sql_text: "SELECT * FROM users".to_string(),
            holes: vec![],
        };
        let map = OffsetMap::new(&fragment);
        // Offset 0 in SQL → offset 10 in host.
        assert_eq!(map.to_host(0), 10);
        assert_eq!(map.to_host(7), 17);
    }

    #[test]
    fn single_hole_shorter() {
        // Host: "SELECT * FROM {table_name}" (range 10..36)
        // SQL:  "SELECT * FROM __hole_0__"
        // Hole: {table_name} at host 24..36 (12 bytes), placeholder at sql offset 14 (10 bytes)
        let fragment = EmbeddedFragment {
            sql_range: 10..36,
            sql_text: "SELECT * FROM __hole_0__".to_string(),
            holes: vec![Hole {
                host_range: 24..36,
                sql_offset: 14,
                placeholder: "__hole_0__".to_string(),
            }],
        };
        let map = OffsetMap::new(&fragment);

        // Before hole: offset 0 → 10, offset 13 → 23.
        assert_eq!(map.to_host(0), 10);
        assert_eq!(map.to_host(13), 23);

        // Inside hole: maps to start of hole in host.
        assert_eq!(map.to_host(14), 24);
        assert_eq!(map.to_host(20), 24);
    }
}
