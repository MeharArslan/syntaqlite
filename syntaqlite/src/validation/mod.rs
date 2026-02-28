// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub mod types;

mod checks;
mod fuzzy;
mod scope;
mod walker;

use crate::ast_traits::AstTypes;
use crate::parser::{FromArena, NodeId, NodeReader};

use scope::ScopeStack;
pub use types::{
    ColumnDef, Diagnostic, DiagnosticMessage, DocumentContext, FunctionDef, Help, RelationDef,
    RelationKind, SessionContext, Severity,
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

/// Validate a single parsed statement against a schema and function catalog,
/// generic over the dialect's AST types.
///
/// Walks the AST and checks that table names, column references, and
/// function calls resolve against the provided context.
///
/// Resolution order: SQL scope stack → `document` (DDL from earlier in the
/// document) → `session` (externally-provided ambient schema).
pub fn validate_statement_dialect<'a, A: AstTypes<'a>>(
    reader: &'a NodeReader<'a>,
    stmt_id: NodeId,
    dialect: crate::Dialect<'_>,
    session: Option<&SessionContext>,
    document: Option<&DocumentContext>,
    functions: &'a [FunctionDef],
    config: &'a ValidationConfig,
) -> Vec<Diagnostic> {
    let stmt: Option<A::Stmt> = FromArena::from_arena(reader, stmt_id);
    let Some(stmt) = stmt else {
        return Vec::new();
    };

    let mut scope = ScopeStack::new(session, document);

    walker::Walker::<A>::run(reader, stmt, dialect, &mut scope, functions, config)
}

/// Validate a single parsed statement using the built-in SQLite dialect.
///
/// Convenience wrapper around [`validate_statement_dialect`] that uses
/// the SQLite AST types and dialect.
#[cfg(feature = "sqlite")]
pub fn validate_statement<'a>(
    reader: &'a NodeReader<'a>,
    stmt_id: NodeId,
    session: Option<&SessionContext>,
    document: Option<&DocumentContext>,
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let dialect = crate::sqlite::low_level::dialect();
    validate_statement_dialect::<crate::sqlite::ast::SqliteAst>(
        reader, stmt_id, *dialect, session, document, functions, config,
    )
}

/// Validate all statements in a document incrementally.
///
/// Each statement is validated against the schema accumulated from prior
/// statements, then contributes its own DDL to the document context.
pub fn validate_document(
    reader: &NodeReader<'_>,
    stmt_ids: &[NodeId],
    dialect: &crate::Dialect<'_>,
    session: Option<&SessionContext>,
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let mut doc_ctx = DocumentContext::new();
    let mut all_diags = Vec::new();
    for &stmt_id in stmt_ids {
        let diags = validate_statement_dialect::<crate::sqlite::ast::SqliteAst>(
            reader, stmt_id, *dialect, session, Some(&doc_ctx), functions, config,
        );
        all_diags.extend(diags);
        doc_ctx.accumulate(reader, stmt_id, dialect, session);
    }
    all_diags
}
