// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Semantic validation of parsed SQL.
//!
//! Walks the AST and checks that table names, column references, and
//! function calls resolve against a provided schema context. Produces
//! [`Diagnostic`] values with byte-offset spans and optional "did you
//! mean?" suggestions.
//!
//! The entry point is [`Validator`], which owns a parser and validates SQL
//! in a single call.

pub use catalog::FunctionCatalog;
pub use render::SourceContext;
pub use types::ColumnDef;
pub use types::Diagnostic;
pub use types::DiagnosticMessage;
pub use types::DocumentContext;
pub use types::FunctionDef;
pub use types::Help;
pub use types::RelationDef;
pub use types::RelationKind;
pub use types::SessionContext;
pub use types::Severity;

pub(crate) mod types;

mod catalog;
mod checks;
mod fuzzy;
mod render;
mod scope;
mod walker;

use syntaqlite_parser::DialectConfig;
use syntaqlite_parser::DialectNodeType;
use syntaqlite_parser::NodeId;
use syntaqlite_parser::RawDialect;
use syntaqlite_parser::RawParser;
use syntaqlite_parser::ast_traits::AstTypes;
use syntaqlite_parser::{ParseError, RawNodeReader};

use scope::ScopeStack;
use types::expand_function_info;

/// Configuration for semantic validation.
#[derive(Clone, Copy)]
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
pub(crate) fn validate_statement_dialect<'a, A: AstTypes<'a>>(
    reader: RawNodeReader<'a>,
    stmt_id: NodeId,
    dialect: RawDialect<'_>,
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

/// Validate all statements in a document incrementally.
///
/// Each statement is validated against the schema accumulated from prior
/// statements, then contributes its own DDL to the document context.
pub(crate) fn validate_document(
    reader: RawNodeReader<'_>,
    stmt_ids: &[NodeId],
    dialect: RawDialect<'_>,
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
            dialect,
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
/// Owns a [`RawParser`] internally and builds the function catalog
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
    parser: RawParser<'d>,
    dialect: RawDialect<'d>,
    functions: Vec<FunctionDef>,
}

impl<'d> Validator<'d> {
    /// Create a validator for the built-in SQLite dialect with default configuration.
    ///
    /// Pre-populates the function catalog with all SQLite built-in functions
    /// available under the default [`DialectConfig`].
    #[cfg(feature = "sqlite")]
    pub fn new() -> Validator<'static> {
        let dc = DialectConfig::default();
        let functions: Vec<FunctionDef> = syntaqlite_parser::available_functions(&dc)
            .into_iter()
            .flat_map(|info| expand_function_info(info))
            .collect();
        Validator::with_config(crate::dialect::sqlite(), functions, None)
    }

    /// Create a validator bound to the given dialect with custom configuration.
    pub fn with_config(
        dialect: impl Into<RawDialect<'d>>,
        functions: Vec<FunctionDef>,
        dialect_config: Option<syntaqlite_parser::DialectConfig>,
    ) -> Self {
        let dialect = dialect.into();
        let parser = RawParser::with_config(
            dialect,
            &syntaqlite_parser::ParserConfig {
                dialect_config,
                ..syntaqlite_parser::ParserConfig::default()
            },
        );
        Validator {
            parser,
            dialect,
            functions,
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
            self.dialect,
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

/// Validate a mix of successful and failed parse results.
///
/// Converts each `Err` into a parse-error diagnostic, collects all valid
/// roots (from `Ok` values and from recovered roots in `Err` values),
/// then runs [`validate_document`] on the collected roots.
pub(crate) fn validate_parse_results(
    reader: RawNodeReader<'_>,
    results: &[Result<NodeId, ParseError>],
    source: &str,
    dialect: RawDialect<'_>,
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
