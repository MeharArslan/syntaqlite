// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Core analysis engine.
//!
//! [`SemanticAnalyzer`] is the single entry point for all semantic analysis —
//! diagnostics, semantic tokens, completions. It replaces the old `Validator`,
//! `EmbeddedAnalyzer`, and `AnalysisHost`.

use syntaqlite_parser::ast_traits::AstTypes;
use syntaqlite_parser::{
    DialectNodeType, NodeId, ParseError, ParserConfig, RawDialect, RawParseResult, RawParser,
};

use super::ValidationConfig;
use super::catalog::{CatalogStack, DatabaseCatalog, DocumentCatalog, StaticCatalog};
use super::diagnostics::{Diagnostic, DiagnosticMessage, Severity};
use super::model::SemanticModel;
use super::scope::ScopeStack;
use super::walker::Walker;

/// Analysis engine. Long-lived, reuses scratch buffers internally.
///
/// Created once for a dialect, reused across inputs. Holds the static
/// catalog (dialect builtins) and reusable scratch space.
///
/// # Example
///
/// ```
/// use syntaqlite::semantic::{SemanticAnalyzer, DatabaseCatalog};
///
/// let catalog = DatabaseCatalog::default();
/// let mut analyzer = SemanticAnalyzer::new();
/// let diags = analyzer.diagnostics("SELECT 1", &catalog);
/// assert!(diags.is_empty());
/// ```
pub struct SemanticAnalyzer<'d> {
    dialect: RawDialect<'d>,
    dialect_config: Option<syntaqlite_parser::DialectConfig>,

    /// Built once from dialect at construction — dialect builtins.
    static_catalog: StaticCatalog,

    /// Reusable scratch buffers — cleared, not reallocated.
    diag_buf: Vec<Diagnostic>,
    doc_catalog: DocumentCatalog,
}

impl<'d> SemanticAnalyzer<'d> {
    /// Create an analyzer for the built-in SQLite dialect.
    #[cfg(feature = "sqlite")]
    pub fn new() -> SemanticAnalyzer<'static> {
        let dialect = crate::dialect::sqlite();
        SemanticAnalyzer::with_dialect(dialect)
    }

    /// Create an analyzer bound to a specific dialect.
    pub fn with_dialect(dialect: impl Into<RawDialect<'d>>) -> Self {
        let dialect = dialect.into();
        let static_catalog =
            StaticCatalog::for_dialect(&dialect, &syntaqlite_parser::DialectConfig::default());
        SemanticAnalyzer {
            dialect,
            dialect_config: None,
            static_catalog,
            diag_buf: Vec::new(),
            doc_catalog: DocumentCatalog::new(),
        }
    }

    /// Create an analyzer with a specific dialect configuration.
    pub fn with_dialect_config(
        dialect: impl Into<RawDialect<'d>>,
        config: &syntaqlite_parser::DialectConfig,
    ) -> Self {
        let dialect = dialect.into();
        let static_catalog = StaticCatalog::for_dialect(&dialect, config);
        SemanticAnalyzer {
            dialect,
            dialect_config: Some(*config),
            static_catalog,
            diag_buf: Vec::new(),
            doc_catalog: DocumentCatalog::new(),
        }
    }

    /// Access the underlying dialect.
    pub fn dialect(&self) -> RawDialect<'d> {
        self.dialect
    }

    // ── Primary API — string in, results out ─────────────────────────

    /// Parse and validate SQL, returning all diagnostics (parse + semantic).
    pub fn diagnostics(&mut self, source: &str, catalog: &DatabaseCatalog) -> Vec<Diagnostic> {
        let model = self.prepare(source);
        self.diagnostics_prepared(&model, catalog)
    }

    /// Parse and validate SQL with explicit config, returning all diagnostics.
    pub fn diagnostics_with_config(
        &mut self,
        source: &str,
        catalog: &DatabaseCatalog,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        let model = self.prepare(source);
        self.diagnostics_prepared_with_config_dialect::<syntaqlite_parser_sqlite::ast::SqliteAst>(
            &model, catalog, config,
        )
    }

    // ── Advanced API — prepare once, query many ──────────────────────

    /// Parse SQL and produce an opaque model for repeated queries.
    pub fn prepare<'a>(&mut self, source: &'a str) -> SemanticModel<'a, 'd>
    where
        'd: 'a,
    {
        let parser = RawParser::with_config(
            self.dialect,
            &ParserConfig {
                collect_tokens: true,
                dialect_config: self.dialect_config,
                ..ParserConfig::default()
            },
        );
        let mut cursor = parser.parse(source);

        let stmts: Vec<Result<NodeId, ParseError>> =
            (&mut cursor).map(|r| r.map(|nr| nr.id())).collect();

        SemanticModel::new(parser, cursor, stmts)
    }

    /// Diagnostics from a prepared model (no re-parsing).
    pub fn diagnostics_prepared(
        &mut self,
        model: &SemanticModel<'_, 'd>,
        catalog: &DatabaseCatalog,
    ) -> Vec<Diagnostic> {
        self.diagnostics_prepared_dialect::<syntaqlite_parser_sqlite::ast::SqliteAst>(
            model, catalog,
        )
    }

    /// Diagnostics from a prepared model, generic over dialect AST types.
    pub fn diagnostics_prepared_dialect<A: for<'a> AstTypes<'a>>(
        &mut self,
        model: &SemanticModel<'_, 'd>,
        catalog: &DatabaseCatalog,
    ) -> Vec<Diagnostic> {
        let config = ValidationConfig::default();
        self.diagnostics_prepared_with_config_dialect::<A>(model, catalog, &config)
    }

    /// Diagnostics with explicit config, generic over dialect AST types.
    pub(crate) fn diagnostics_prepared_with_config_dialect<A: for<'a> AstTypes<'a>>(
        &mut self,
        model: &SemanticModel<'_, 'd>,
        catalog: &DatabaseCatalog,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        self.diag_buf.clear();
        self.doc_catalog.clear();

        let reader = model.reader();

        // Collect parse errors and valid statement roots.
        let mut stmt_ids = Vec::new();
        for result in &model.stmts {
            match result {
                Ok(id) => stmt_ids.push(*id),
                Err(err) => {
                    if let Some(root) = err.root {
                        stmt_ids.push(root);
                    }
                    let (start_offset, end_offset) = parse_error_span(err, model.source());
                    self.diag_buf.push(Diagnostic {
                        start_offset,
                        end_offset,
                        message: DiagnosticMessage::Other(err.message.clone()),
                        severity: Severity::Error,
                        help: None,
                    });
                }
            }
        }

        // Validate each statement with incremental document catalog.
        for &stmt_id in &stmt_ids {
            let catalog_stack = CatalogStack {
                static_: &self.static_catalog,
                database: catalog,
                document: &self.doc_catalog,
            };
            let diags = validate_statement_dialect::<A>(
                reader,
                stmt_id,
                self.dialect,
                &catalog_stack,
                config,
            );
            self.diag_buf.extend(diags);

            #[cfg(feature = "sqlite")]
            self.doc_catalog
                .accumulate(reader, stmt_id, self.dialect, Some(catalog));
        }

        self.diag_buf.clone()
    }
}

#[cfg(feature = "sqlite")]
impl Default for SemanticAnalyzer<'static> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Internal validation functions ────────────────────────────────────

/// Validate a single parsed statement against the catalog stack.
pub(crate) fn validate_statement_dialect<'a, A: AstTypes<'a>>(
    reader: RawParseResult<'a>,
    stmt_id: NodeId,
    dialect: RawDialect<'_>,
    catalog: &'a CatalogStack<'a>,
    config: &'a ValidationConfig,
) -> Vec<Diagnostic> {
    let stmt: Option<A::Stmt> = DialectNodeType::from_arena(reader, stmt_id);
    let Some(stmt) = stmt else {
        return Vec::new();
    };

    let mut scope = ScopeStack::new(CatalogStack {
        static_: catalog.static_,
        database: catalog.database,
        document: catalog.document,
    });

    Walker::<A>::run(reader, stmt, dialect, &mut scope, catalog, config)
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
