// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub mod types;

mod checks;
mod fuzzy;
mod scope;
mod walker;

use crate::parser::{FromArena, NodeId, NodeReader};
use crate::sqlite::ast::Stmt;

use scope::ScopeStack;
pub use types::{
    AmbientContext, ColumnDef, Diagnostic, FunctionDef, SessionContext, Severity, TableDef, ViewDef,
};

/// Configuration for semantic validation.
pub struct ValidationConfig {
    /// When `true`, unresolved names are reported as errors.
    /// When `false`, they are reported as warnings.
    pub strict_schema: bool,
    /// Maximum Levenshtein distance for "did you mean?" suggestions.
    pub suggestion_threshold: usize,
}

impl Default for ValidationConfig {
    fn default() -> Self {
        ValidationConfig {
            strict_schema: false,
            suggestion_threshold: 2,
        }
    }
}

impl ValidationConfig {
    fn severity(&self) -> Severity {
        if self.strict_schema {
            Severity::Error
        } else {
            Severity::Warning
        }
    }
}

/// Validate a single parsed statement against a schema and function catalog.
///
/// Walks the AST and checks that table names, column references, and
/// function calls resolve against the provided context.
pub fn validate_statement<'a>(
    reader: &'a NodeReader<'a>,
    stmt_id: NodeId,
    context: Option<&SessionContext>,
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let stmt: Option<Stmt<'a>> = FromArena::from_arena(reader, stmt_id);
    let Some(stmt) = stmt else {
        return Vec::new();
    };

    let mut scope = ScopeStack::new(context);

    walker::Walker::run(reader, stmt, &mut scope, functions, config)
}
