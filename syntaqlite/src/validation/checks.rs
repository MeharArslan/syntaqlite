// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use super::ValidationConfig;
use super::fuzzy::best_suggestion;
use super::scope::{ColumnResolution, ScopeStack};
use super::types::{Diagnostic, DiagnosticMessage, FunctionDef, Help};

pub(super) fn check_table_ref(
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
        DiagnosticMessage::UnknownTable {
            name: name.to_string(),
        },
        suggestion.map(Help::Suggestion),
        config,
    ))
}

pub(super) fn check_column_ref(
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
                DiagnosticMessage::UnknownColumn {
                    column: column.to_string(),
                    table: Some(tbl.to_string()),
                },
                suggestion.map(Help::Suggestion),
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
                DiagnosticMessage::UnknownColumn {
                    column: column.to_string(),
                    table: None,
                },
                suggestion.map(Help::Suggestion),
                config,
            ))
        }
    }
}

pub(super) fn check_function_call(
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
            DiagnosticMessage::UnknownFunction {
                name: name.to_string(),
            },
            suggestion.map(Help::Suggestion),
            config,
        ));
    };

    // If any definition accepts this arg count (or is variadic), it's OK.
    let arity_ok = std::iter::once(first_match)
        .chain(by_name)
        .any(|f| f.args.is_none_or(|n| n == arg_count));

    if !arity_ok {
        let expected: Vec<usize> = functions
            .iter()
            .filter(|f| f.name.eq_ignore_ascii_case(name))
            .filter_map(|f| f.args)
            .collect();
        return Some(make_diagnostic(
            offset,
            length,
            DiagnosticMessage::FunctionArity {
                name: name.to_string(),
                expected,
                got: arg_count,
            },
            None,
            config,
        ));
    }

    None
}

fn make_diagnostic(
    offset: usize,
    length: usize,
    message: DiagnosticMessage,
    help: Option<Help>,
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
