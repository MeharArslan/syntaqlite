// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Semantic validation of parsed SQL.
//!
//! Walks the AST and checks that table names, column references, and
//! function calls resolve against a provided schema context. Produces
//! [`Diagnostic`] values with byte-offset spans and optional "did you
//! mean?" suggestions.
//!
//! The high-level entry point is [`Validator`], which owns a parser and
//! validates SQL in a single call. For finer control, use
//! [`validate_document`] or [`validate_statement`] directly.

pub(crate) mod types;

mod checks;
mod fuzzy;
mod scope;
mod walker;

use crate::ast_traits::AstTypes;
use crate::parser::nodes::NodeId;
use crate::parser::session::{ParseError, RawNodeReader};
use crate::parser::typed_list::DialectNodeType;

use scope::ScopeStack;

// ── Public re-exports ────────────────────────────────────────────────────

pub(crate) use types::expand_function_info;
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
    reader: &'a RawNodeReader<'a>,
    stmt_id: NodeId,
    dialect: crate::Dialect<'_>,
    session: Option<&SessionContext>,
    document: Option<&DocumentContext>,
    functions: &'a [FunctionDef],
    config: &'a ValidationConfig,
) -> Vec<Diagnostic> {
    let stmt: Option<A::Stmt> = DialectNodeType::from_arena(reader, stmt_id);
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
    reader: &'a RawNodeReader<'a>,
    stmt_id: NodeId,
    session: Option<&SessionContext>,
    document: Option<&DocumentContext>,
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let dialect = *crate::sqlite::DIALECT;
    validate_statement_dialect::<syntaqlite_parser_sqlite::ast::SqliteAst>(
        reader, stmt_id, dialect, session, document, functions, config,
    )
}

/// Validate all statements in a document incrementally.
///
/// Each statement is validated against the schema accumulated from prior
/// statements, then contributes its own DDL to the document context.
pub fn validate_document(
    reader: &RawNodeReader<'_>,
    stmt_ids: &[NodeId],
    dialect: &crate::Dialect<'_>,
    session: Option<&SessionContext>,
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let mut doc_ctx = DocumentContext::new();
    let mut all_diags = Vec::new();
    for &stmt_id in stmt_ids {
        let diags = validate_statement_dialect::<syntaqlite_parser_sqlite::ast::SqliteAst>(
            reader,
            stmt_id,
            *dialect,
            session,
            Some(&doc_ctx),
            functions,
            config,
        );
        all_diags.extend(diags);
        doc_ctx.accumulate(reader, stmt_id, dialect, session);
    }
    all_diags
}

/// Compute the byte range for a parse error in the source text.
pub(crate) fn parse_error_span(err: &ParseError, source: &str) -> (usize, usize) {
    match (err.offset, err.length) {
        (Some(offset), Some(length)) if length > 0 => (offset, offset + length),
        (Some(offset), _) => {
            if offset >= source.len() && !source.is_empty() {
                (source.len() - 1, source.len())
            } else {
                (offset, (offset + 1).min(source.len()))
            }
        }
        _ => {
            let end = source.len();
            let start = if end > 0 { end - 1 } else { 0 };
            (start, end)
        }
    }
}

/// Convert a [`ParseError`] into a [`Diagnostic`].
fn parse_error_to_diagnostic(err: &ParseError, source: &str) -> Diagnostic {
    let (start_offset, end_offset) = parse_error_span(err, source);
    Diagnostic {
        start_offset,
        end_offset,
        message: DiagnosticMessage::Other(err.message.clone()),
        severity: Severity::Error,
        help: None,
    }
}

// ── Validator (high-level, reusable) ─────────────────────────────────────

/// High-level SQL validator. Created from a `Dialect`, reusable across inputs.
///
/// Owns a [`RawParser`](crate::parser::session::RawParser) internally and builds the function catalog
/// once at construction. Call [`validate`](Validator::validate) to parse and
/// validate SQL in a single step.
///
/// # Example
///
/// ```
/// use syntaqlite::validation::{Validator, ValidationConfig};
///
/// let mut validator = Validator::new();
/// let diags = validator.validate("SELEC 1", None, &ValidationConfig::default());
/// assert!(!diags.is_empty());
/// ```
pub struct Validator<'d> {
    parser: crate::parser::session::RawParser<'d>,
    dialect: crate::Dialect<'d>,
    functions: Vec<FunctionDef>,
}

// SAFETY: Dialect is Send+Sync, Parser is Send.
unsafe impl Send for Validator<'_> {}

impl<'d> Validator<'d> {
    /// Create a validator for the built-in SQLite dialect with default configuration.
    ///
    /// Pre-populates the function catalog with all SQLite built-in functions
    /// available under the default [`DialectConfig`](syntaqlite_parser::dialect::ffi::DialectConfig).
    #[cfg(feature = "sqlite")]
    pub fn new() -> Validator<'static> {
        let dc = syntaqlite_parser::dialect::ffi::DialectConfig::default();
        let functions: Vec<FunctionDef> = syntaqlite_parser::sqlite::available_functions(&dc)
            .into_iter()
            .flat_map(|info| expand_function_info(info))
            .collect();
        Validator::builder(&crate::sqlite::DIALECT)
            .functions(functions)
            .build()
    }

    /// Create a builder for a validator bound to the given dialect.
    pub fn builder(dialect: &'d crate::Dialect<'d>) -> ValidatorBuilder<'d> {
        ValidatorBuilder {
            dialect,
            functions: Vec::new(),
            dialect_config: None,
        }
    }

    /// Validate SQL source text. Parses and returns all diagnostics
    /// (both parse errors and semantic issues).
    pub fn validate(
        &mut self,
        source: &str,
        session: Option<&SessionContext>,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        let mut cursor = self.parser.parse(source);
        // Collect NodeRef results and convert to NodeId results for validate_parse_results.
        let results: Vec<Result<NodeId, ParseError>> =
            (&mut cursor).map(|r| r.map(|nr| nr.id())).collect();
        validate_parse_results(
            cursor.reader(),
            &results,
            source,
            &self.dialect,
            session,
            &self.functions,
            config,
        )
    }
}

#[cfg(feature = "sqlite")]
impl Default for Validator<'static> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for configuring a [`Validator`] before construction.
pub struct ValidatorBuilder<'d> {
    dialect: &'d crate::Dialect<'d>,
    functions: Vec<FunctionDef>,
    dialect_config: Option<syntaqlite_parser::dialect::ffi::DialectConfig>,
}

impl<'d> ValidatorBuilder<'d> {
    /// Set the function catalog used for function-name/arity validation.
    ///
    /// By default the list is empty. Use
    /// [`sqlite_function_defs`](crate::embedded::sqlite_function_defs)
    /// to populate it with the SQLite built-in catalog.
    pub fn functions(mut self, functions: Vec<FunctionDef>) -> Self {
        self.functions = functions;
        self
    }

    /// Set dialect config for version/cflag-gated parsing.
    pub fn dialect_config(mut self, config: syntaqlite_parser::dialect::ffi::DialectConfig) -> Self {
        self.dialect_config = Some(config);
        self
    }

    /// Build the validator.
    pub fn build(self) -> Validator<'d> {
        let mut builder = crate::parser::session::RawParser::builder(self.dialect);
        if let Some(dc) = self.dialect_config {
            builder = builder.dialect_config(dc);
        }

        Validator {
            parser: builder.build(),
            dialect: *self.dialect,
            functions: self.functions,
        }
    }
}

/// Validate a mix of successful and failed parse results.
///
/// Converts each `Err` into a parse-error diagnostic, collects all valid
/// roots (from `Ok` values and from recovered roots in `Err` values),
/// then runs [`validate_document`] on the collected roots.
pub fn validate_parse_results(
    reader: &RawNodeReader<'_>,
    results: &[Result<NodeId, ParseError>],
    source: &str,
    dialect: &crate::Dialect<'_>,
    session: Option<&SessionContext>,
    functions: &[FunctionDef],
    config: &ValidationConfig,
) -> Vec<Diagnostic> {
    let mut diags = Vec::new();
    let mut stmt_ids = Vec::new();

    for result in results {
        match result {
            Ok(id) => stmt_ids.push(*id),
            Err(err) => {
                if let Some(root) = err.root {
                    stmt_ids.push(root);
                }
                diags.push(parse_error_to_diagnostic(err, source));
            }
        }
    }

    let semantic = validate_document(reader, &stmt_ids, dialect, session, functions, config);
    diags.extend(semantic);
    diags
}
