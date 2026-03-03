// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Core analysis engine.
//!
//! [`SemanticAnalyzer`] is the single entry point for all semantic analysis —
//! diagnostics, semantic tokens, completions. It replaces the old `Validator`,
//! `EmbeddedAnalyzer`, and `AnalysisHost`.

use syntaqlite_parser::ast_traits::AstTypes;
use syntaqlite_parser::{
    DialectEnv, DialectNodeType, ParseError, ParserConfig, RawIncrementalParser, RawNodeId,
    RawParseResult, RawParser,
};

use crate::dialect::{DialectExt, TokenCategory};

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
    dialect: DialectEnv<'d>,

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
    pub fn with_dialect(dialect: impl Into<DialectEnv<'d>>) -> Self {
        let dialect = dialect.into();
        let static_catalog = StaticCatalog::for_dialect(&dialect);
        SemanticAnalyzer {
            dialect,
            static_catalog,
            diag_buf: Vec::new(),
            doc_catalog: DocumentCatalog::new(),
        }
    }

    /// Access the underlying dialect.
    pub fn dialect(&self) -> DialectEnv<'d> {
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
    pub fn prepare(&mut self, source: &str) -> SemanticModel<'d> {
        let parser = RawParser::with_config(
            self.dialect,
            &ParserConfig {
                collect_tokens: true,
                ..ParserConfig::default()
            },
        );
        let mut cursor = parser.parse(source);

        let mut stmts = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmts.push(result.map(|node_ref| node_ref.id()));
        }

        SemanticModel::new(parser, cursor, stmts)
    }

    /// Diagnostics from a prepared model (no re-parsing).
    pub fn diagnostics_prepared(
        &mut self,
        model: &SemanticModel<'d>,
        catalog: &DatabaseCatalog,
    ) -> Vec<Diagnostic> {
        self.diagnostics_prepared_dialect::<syntaqlite_parser_sqlite::ast::SqliteAst>(
            model, catalog,
        )
    }

    /// Diagnostics from a prepared model, generic over dialect AST types.
    pub fn diagnostics_prepared_dialect<A: for<'a> AstTypes<'a>>(
        &mut self,
        model: &SemanticModel<'d>,
        catalog: &DatabaseCatalog,
    ) -> Vec<Diagnostic> {
        let config = ValidationConfig::default();
        self.diagnostics_prepared_with_config_dialect::<A>(model, catalog, &config)
    }

    // ── Lazy query methods — compute from model on demand ────────────

    /// Parse-error diagnostics extracted from a prepared model.
    ///
    /// Iterates `model.stmts` and converts `Err(ParseError)` entries to
    /// `Diagnostic`. Does not include semantic diagnostics.
    pub fn parse_diagnostics(&self, model: &SemanticModel<'d>) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        for result in &model.stmts {
            if let Err(err) = result {
                let (start_offset, end_offset) = parse_error_span(err, model.source());
                diagnostics.push(Diagnostic {
                    start_offset,
                    end_offset,
                    message: DiagnosticMessage::Other(err.message.clone()),
                    severity: Severity::Error,
                    help: None,
                });
            }
        }
        diagnostics
    }

    /// Semantic tokens for syntax highlighting, computed from the model's
    /// parser token and comment data.
    pub fn semantic_tokens(&self, model: &SemanticModel<'d>) -> Vec<super::model::SemanticToken> {
        let reader = model.reader();
        let mut tokens = Vec::new();

        for tp in reader.tokens() {
            let cat = self.dialect.classify_token(tp.type_, tp.flags);
            if cat == TokenCategory::Other {
                continue;
            }
            tokens.push(super::model::SemanticToken {
                offset: tp.offset as usize,
                length: tp.length as usize,
                category: cat,
            });
        }

        for c in reader.comments() {
            tokens.push(super::model::SemanticToken {
                offset: c.offset as usize,
                length: c.length as usize,
                category: TokenCategory::Comment,
            });
        }
        tokens.sort_by_key(|t| t.offset);
        tokens
    }

    /// Expected tokens and semantic context at `offset`, computed by
    /// replaying the model's token stream through an incremental parser.
    pub fn completion_info(
        &self,
        model: &SemanticModel<'d>,
        offset: usize,
    ) -> super::model::CompletionInfo {
        let source = model.source();
        let reader = model.reader();
        let tokens = reader.tokens();
        let cursor_offset = offset.min(source.len());

        let mut boundary =
            tokens.partition_point(|t| (t.offset + t.length) as usize <= cursor_offset);

        // Skip zero-width tokens at cursor.
        while boundary > 0 && {
            let t = &tokens[boundary - 1];
            t.length == 0 && t.offset as usize == cursor_offset
        } {
            boundary -= 1;
        }

        // Backtrack if cursor is mid-identifier so we still suggest completions.
        let mut backtracked = false;
        if boundary > 0
            && (tokens[boundary - 1].offset + tokens[boundary - 1].length) as usize == cursor_offset
            && cursor_offset > 0
        {
            let b = source.as_bytes()[cursor_offset - 1];
            if b.is_ascii_alphanumeric() || b == b'_' {
                boundary -= 1;
                backtracked = true;
            }
        }

        let tk_semi = self.dialect.tk_semi();
        let start = tokens[..boundary]
            .iter()
            .rposition(|t| t.type_ == tk_semi)
            .map_or(0, |idx| idx + 1);

        let stmt_tokens = &tokens[start..boundary];

        let inc_parser = RawIncrementalParser::new(self.dialect);
        let mut cursor = inc_parser.feed(source);
        let mut last_expected = cursor.expected_tokens();

        for tok in stmt_tokens {
            let span = tok.offset as usize..(tok.offset + tok.length) as usize;
            if cursor.feed_token(tok.type_, span).is_err() {
                return super::model::CompletionInfo {
                    tokens: last_expected,
                    context: super::model::CompletionContext::from_raw(cursor.completion_context()),
                };
            }
            last_expected = cursor.expected_tokens();
        }

        let context = super::model::CompletionContext::from_raw(cursor.completion_context());

        if backtracked {
            let extra = &tokens[boundary];
            let span = extra.offset as usize..(extra.offset + extra.length) as usize;
            if cursor.feed_token(extra.type_, span).is_ok() {
                let after = cursor.expected_tokens();
                let mut seen: std::collections::HashSet<u32> =
                    last_expected.iter().copied().collect();
                for tok in after {
                    if seen.insert(tok) {
                        last_expected.push(tok);
                    }
                }
            }
        }

        super::model::CompletionInfo {
            tokens: last_expected,
            context,
        }
    }

    // ── Validation methods ────────────────────────────────────────────

    /// Diagnostics with explicit config, generic over dialect AST types.
    pub(crate) fn diagnostics_prepared_with_config_dialect<A: for<'a> AstTypes<'a>>(
        &mut self,
        model: &SemanticModel<'d>,
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
    stmt_id: RawNodeId,
    dialect: DialectEnv<'_>,
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
