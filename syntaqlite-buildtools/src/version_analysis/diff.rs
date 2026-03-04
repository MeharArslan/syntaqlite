// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Variant grouping and unified diff generation.
//!
//! Groups consecutive versions with identical fragment hashes into
//! `VariantGroup`s and produces unified diffs between adjacent variants.

use serde::Serialize;
use similar::TextDiff;

use super::{SqliteVersion, VariantGroup};

/// A unified diff between two consecutive variants.
#[derive(Debug, Clone, Serialize)]
pub struct VariantDiff {
    pub(crate) from_id: String,
    pub(crate) to_id: String,
    pub(crate) unified_diff: String,
}

/// Group consecutive (version, hash, text) triples into variant groups.
///
/// Versions must be sorted. Consecutive versions with the same hash
/// are merged into one group.
pub(super) fn group_variants(entries: &[(SqliteVersion, String, String)]) -> Vec<VariantGroup> {
    if entries.is_empty() {
        return Vec::new();
    }

    let mut groups: Vec<VariantGroup> = Vec::new();
    let mut variant_counter = 0u32;

    for (version, hash, text) in entries {
        if let Some(last) = groups.last_mut()
            && last.hash == *hash
        {
            last.versions.push(version.clone());
            continue;
        }

        variant_counter += 1;
        groups.push(VariantGroup {
            id: format!("v{variant_counter}"),
            hash: hash.clone(),
            versions: vec![version.clone()],
            text: text.clone(),
        });
    }

    groups
}

/// Compute unified diffs between consecutive variant groups.
pub(super) fn compute_diffs(variants: &[VariantGroup]) -> Vec<VariantDiff> {
    let mut diffs = Vec::new();

    for pair in variants.windows(2) {
        let from = &pair[0];
        let to = &pair[1];

        let diff = TextDiff::from_lines(&from.text, &to.text);
        let unified = diff
            .unified_diff()
            .header(
                &format!("{} ({})", from.id, from.first()),
                &format!("{} ({})", to.id, to.first()),
            )
            .to_string();

        diffs.push(VariantDiff {
            from_id: from.id.clone(),
            to_id: to.id.clone(),
            unified_diff: unified,
        });
    }

    diffs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ver(s: &str) -> SqliteVersion {
        SqliteVersion::parse(s).unwrap()
    }

    #[test]
    fn group_identical_versions() {
        let entries = vec![
            (ver("3.24.0"), "hash_a".into(), "code_a".into()),
            (ver("3.25.0"), "hash_a".into(), "code_a".into()),
            (ver("3.26.0"), "hash_b".into(), "code_b".into()),
        ];
        let groups = group_variants(&entries);
        assert_eq!(groups.len(), 2);
        assert_eq!(groups[0].id, "v1");
        assert_eq!(groups[0].versions.len(), 2);
        assert_eq!(groups[1].id, "v2");
        assert_eq!(groups[1].versions.len(), 1);
    }

    #[test]
    fn diff_between_variants() {
        let entries = vec![
            (ver("3.24.0"), "hash_a".into(), "line1\nline2\n".into()),
            (ver("3.25.0"), "hash_b".into(), "line1\nline3\n".into()),
        ];
        let groups = group_variants(&entries);
        let diffs = compute_diffs(&groups);
        assert_eq!(diffs.len(), 1);
        assert!(diffs[0].unified_diff.contains("-line2"));
        assert!(diffs[0].unified_diff.contains("+line3"));
    }

    #[test]
    fn no_diff_for_single_variant() {
        let entries = vec![
            (ver("3.24.0"), "hash_a".into(), "code".into()),
            (ver("3.25.0"), "hash_a".into(), "code".into()),
        ];
        let groups = group_variants(&entries);
        let diffs = compute_diffs(&groups);
        assert!(diffs.is_empty());
    }
}
