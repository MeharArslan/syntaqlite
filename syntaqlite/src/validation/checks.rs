// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use super::ValidationConfig;
use super::fuzzy::best_suggestion;
use super::scope::{ColumnResolution, ScopeStack};
use super::types::{Diagnostic, DiagnosticMessage, Help};

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
