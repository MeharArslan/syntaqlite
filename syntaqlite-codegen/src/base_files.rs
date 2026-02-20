// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Embedded base SQLite grammar (.y) and node definition (.synq) files.
//!
//! These are compiled into the binary so that `generate-dialect` can run
//! without requiring the user to supply the base SQLite files — only
//! extension files (if any) are needed.

/// Returns all base `.y` action files (filename, contents) in alphabetical order.
pub fn base_y_files() -> &'static [(&'static str, &'static str)] {
    &[
        ("_common.y", include_str!("../parser-actions/_common.y")),
        ("aggregate.y", include_str!("../parser-actions/aggregate.y")),
        ("cast.y", include_str!("../parser-actions/cast.y")),
        ("column_ref_select.y", include_str!("../parser-actions/column_ref_select.y")),
        ("column_refs.y", include_str!("../parser-actions/column_refs.y")),
        ("compound.y", include_str!("../parser-actions/compound.y")),
        ("conditionals.y", include_str!("../parser-actions/conditionals.y")),
        ("create_table.y", include_str!("../parser-actions/create_table.y")),
        ("cte.y", include_str!("../parser-actions/cte.y")),
        ("dml.y", include_str!("../parser-actions/dml.y")),
        ("expressions.y", include_str!("../parser-actions/expressions.y")),
        ("exprlists.y", include_str!("../parser-actions/exprlists.y")),
        ("functions.y", include_str!("../parser-actions/functions.y")),
        ("identifiers.y", include_str!("../parser-actions/identifiers.y")),
        ("literals.y", include_str!("../parser-actions/literals.y")),
        ("misc_expr.y", include_str!("../parser-actions/misc_expr.y")),
        ("orderby.y", include_str!("../parser-actions/orderby.y")),
        ("raise_expr.y", include_str!("../parser-actions/raise_expr.y")),
        ("schema_ops.y", include_str!("../parser-actions/schema_ops.y")),
        ("select.y", include_str!("../parser-actions/select.y")),
        ("table_source.y", include_str!("../parser-actions/table_source.y")),
        ("trigger.y", include_str!("../parser-actions/trigger.y")),
        ("utility_stmts.y", include_str!("../parser-actions/utility_stmts.y")),
        ("values.y", include_str!("../parser-actions/values.y")),
        ("virtual_table.y", include_str!("../parser-actions/virtual_table.y")),
        ("window.y", include_str!("../parser-actions/window.y")),
        ("ztokens.y", include_str!("../parser-actions/ztokens.y")),
    ]
}

/// Returns all base `.synq` node definition files (filename, contents) in alphabetical order.
pub fn base_synq_files() -> &'static [(&'static str, &'static str)] {
    &[
        ("aggregate.synq", include_str!("../parser-nodes/aggregate.synq")),
        ("cast.synq", include_str!("../parser-nodes/cast.synq")),
        ("column_ref.synq", include_str!("../parser-nodes/column_ref.synq")),
        ("common.synq", include_str!("../parser-nodes/common.synq")),
        ("compound.synq", include_str!("../parser-nodes/compound.synq")),
        ("conditionals.synq", include_str!("../parser-nodes/conditionals.synq")),
        ("create_table.synq", include_str!("../parser-nodes/create_table.synq")),
        ("cte.synq", include_str!("../parser-nodes/cte.synq")),
        ("dml.synq", include_str!("../parser-nodes/dml.synq")),
        ("expressions.synq", include_str!("../parser-nodes/expressions.synq")),
        ("functions.synq", include_str!("../parser-nodes/functions.synq")),
        ("misc_expr.synq", include_str!("../parser-nodes/misc_expr.synq")),
        ("raise_expr.synq", include_str!("../parser-nodes/raise_expr.synq")),
        ("schema_ops.synq", include_str!("../parser-nodes/schema_ops.synq")),
        ("select.synq", include_str!("../parser-nodes/select.synq")),
        ("table_source.synq", include_str!("../parser-nodes/table_source.synq")),
        ("trigger.synq", include_str!("../parser-nodes/trigger.synq")),
        ("utility_stmts.synq", include_str!("../parser-nodes/utility_stmts.synq")),
        ("values.synq", include_str!("../parser-nodes/values.synq")),
        ("window.synq", include_str!("../parser-nodes/window.synq")),
    ]
}

/// Merge base files with extension files.
///
/// Extension files with the same name as a base file replace the base version.
/// The result is sorted alphabetically by filename.
pub fn merge_file_sets(
    base: &[(&str, &str)],
    extensions: &[(String, String)],
) -> Vec<(String, String)> {
    use std::collections::BTreeMap;

    let mut merged = BTreeMap::new();
    for (name, content) in base {
        merged.insert(name.to_string(), content.to_string());
    }
    for (name, content) in extensions {
        merged.insert(name.clone(), content.clone());
    }
    merged.into_iter().collect()
}
