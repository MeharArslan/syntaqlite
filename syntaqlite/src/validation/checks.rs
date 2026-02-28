// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use super::ValidationConfig;
use super::fuzzy::best_suggestion;
use super::scope::{ColumnResolution, ScopeStack};
use super::types::{Diagnostic, FunctionDef};

pub fn check_table_ref(
    name: &str,
    offset: usize,
    length: usize,
    scope: &ScopeStack,
    config: &ValidationConfig,
) -> Option<Diagnostic> {
    if name.is_empty() || scope.resolve_table(name) {
        return None;
    }

    let suggestion = best_suggestion(name, &scope.all_table_names(), config.suggestion_threshold);
    Some(make_diagnostic(
        offset,
        length,
        format!("unknown table '{name}'"),
        suggestion.map(|s| format!("did you mean '{s}'?")),
        config,
    ))
}

pub fn check_column_ref(
    table: Option<&str>,
    column: &str,
    offset: usize,
    length: usize,
    scope: &ScopeStack,
    config: &ValidationConfig,
) -> Option<Diagnostic> {
    if column.is_empty() {
        return None;
    }

    match scope.resolve_column(table, column) {
        ColumnResolution::Found => None,
        // Table qualifier itself doesn't resolve — the table check already reported this.
        ColumnResolution::TableNotFound => None,
        ColumnResolution::TableFoundColumnMissing => {
            let tbl = table.unwrap();
            let candidates = scope.all_column_names(Some(tbl));
            let suggestion = best_suggestion(column, &candidates, config.suggestion_threshold);
            Some(make_diagnostic(
                offset,
                length,
                format!("unknown column '{column}' in table '{tbl}'"),
                suggestion.map(|s| format!("did you mean '{s}'?")),
                config,
            ))
        }
        ColumnResolution::NotFound => {
            let suggestion = best_suggestion(
                column,
                &scope.all_column_names(None),
                config.suggestion_threshold,
            );
            Some(make_diagnostic(
                offset,
                length,
                format!("unknown column '{column}'"),
                suggestion.map(|s| format!("did you mean '{s}'?")),
                config,
            ))
        }
    }
}

pub fn check_function_call(
    name: &str,
    arg_count: usize,
    offset: usize,
    length: usize,
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Option<Diagnostic> {
    if name.is_empty() {
        return None;
    }

    let mut by_name = functions
        .iter()
        .filter(|f| f.name.eq_ignore_ascii_case(name));

    let Some(first_match) = by_name.next() else {
        let mut all_names: Vec<String> = functions.iter().map(|f| f.name.clone()).collect();
        all_names.sort_unstable();
        all_names.dedup();
        let suggestion = best_suggestion(name, &all_names, config.suggestion_threshold);
        return Some(make_diagnostic(
            offset,
            length,
            format!("unknown function '{name}'"),
            suggestion.map(|s| format!("did you mean '{s}'?")),
            config,
        ));
    };

    // If any definition accepts this arg count (or is variadic), it's OK.
    let arity_ok = std::iter::once(first_match)
        .chain(by_name)
        .any(|f| f.args.is_none_or(|n| n == arg_count));

    if !arity_ok {
        let expected: Vec<String> = functions
            .iter()
            .filter(|f| f.name.eq_ignore_ascii_case(name))
            .filter_map(|f| f.args.map(|n| n.to_string()))
            .collect();
        return Some(make_diagnostic(
            offset,
            length,
            format!(
                "function '{name}' expects {} argument(s), got {arg_count}",
                expected.join(" or ")
            ),
            None,
            config,
        ));
    }

    None
}

fn make_diagnostic(
    offset: usize,
    length: usize,
    message: String,
    help: Option<String>,
    config: &ValidationConfig,
) -> Diagnostic {
    Diagnostic {
        start_offset: offset,
        end_offset: offset + length,
        message,
        severity: config.severity(),
        help,
    }
}
