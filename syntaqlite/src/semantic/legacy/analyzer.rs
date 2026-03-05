// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Core analysis engine.
//!
//! [`SemanticAnalyzer`] is the single entry point for all semantic analysis -
//! diagnostics, semantic tokens, completions.

use std::collections::HashSet;

use syntaqlite_syntax::any::{AnyNodeId, AnyParsedStatement};
use syntaqlite_syntax::ast_traits::AstTypes;
use syntaqlite_syntax::typed::{GrammarNodeType, TypedParser};
use syntaqlite_syntax::ParserTokenFlags;
use syntaqlite_syntax::TokenType;

use crate::dialect::{Dialect, TokenCategory};

use super::catalog::{CatalogStack, DatabaseCatalog, DocumentCatalog, StaticCatalog};
use super::diagnostics::{Diagnostic, DiagnosticMessage, Severity};
use super::model::{SemanticModel, StoredComment, StoredParseError, StoredToken};
use super::scope::ScopeStack;
use super::walker::Walker;
use super::ValidationConfig;

/// Analysis engine. Long-lived, reuses scratch buffers internally.
///
/// Created once for a dialect, reused across inputs. Holds the static
/// catalog (dialect builtins) and reusable scratch space.
pub(crate) struct SemanticAnalyzer<'d> {
    dialect: Dialect,

    /// Built once from dialect at construction - dialect builtins.
    static_catalog: StaticCatalog,

    /// Reusable scratch buffers - cleared, not reallocated.
    diag_buf: Vec<Diagnostic>,
    doc_catalog: DocumentCatalog,
}

impl<'d> SemanticAnalyzer<'d> {
    /// Create an analyzer for the built-in `SQLite` dialect.
    #[cfg(feature = "sqlite")]
    pub(crate) fn new() -> SemanticAnalyzer<'static> {
        let dialect = crate::dialect::sqlite();
        SemanticAnalyzer::with_dialect(dialect)
    }

    /// Create an analyzer bound to a specific dialect.
    pub(crate) fn with_dialect(dialect: impl Into<Dialect>) -> Self {
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
    pub(crate) fn dialect(&self) -> Dialect {
        self.dialect
    }

    fn push_statement_diagnostics<'a, A: AstTypes<'a>>(
        &mut self,
        stmt_result: AnyParsedStatement<'a>,
        stmt_id: AnyNodeId,
        catalog: &DatabaseCatalog,
        config: &ValidationConfig,
    ) {
        let catalog_stack = CatalogStack {
            static_: &self.static_catalog,
            database: catalog,
            document: &self.doc_catalog,
        };
        self.diag_buf.extend(validate_statement_dialect::<A>(
            stmt_result,
            stmt_id,
            &catalog_stack,
            config,
        ));

        #[cfg(feature = "sqlite")]
        self.doc_catalog
            .accumulate(stmt_result, stmt_id, self.dialect, Some(catalog));
    }

    // -- Primary API -------------------------------------------------------

    /// Parse and validate SQL, returning all diagnostics (parse + semantic).
    pub(crate) fn diagnostics(
        &mut self,
        source: &str,
        catalog: &DatabaseCatalog,
    ) -> Vec<Diagnostic> {
        let model = self.prepare(source);
        self.diagnostics_prepared(&model, catalog)
    }

    /// Parse and validate SQL with explicit config, returning all diagnostics.
    pub(crate) fn diagnostics_with_config(
        &mut self,
        source: &str,
        catalog: &DatabaseCatalog,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        let model = self.prepare(source);
        self.diagnostics_prepared_with_config_default_ast(&model, catalog, config)
    }

    // -- Advanced API ------------------------------------------------------

    /// Parse SQL and produce an opaque model for repeated queries.
    pub(crate) fn prepare(&mut self, source: &str) -> SemanticModel {
        let parser = syntaqlite_syntax::Parser::with_config(
            &syntaqlite_syntax::ParserConfig::default().with_collect_tokens(true),
        );
        let mut session = parser.parse(source);

        let mut tokens = Vec::new();
        let mut comments = Vec::new();
        let mut parse_errors = Vec::new();

        while let Some(stmt) = session.next() {
            match stmt {
                Ok(stmt) => {
                    collect_stmt_positions(
                        source,
                        stmt.tokens().map(|t| (t.text(), t.token_type(), t.flags())),
                        stmt.comments().map(|c| c.text),
                        &mut tokens,
                        &mut comments,
                    );
                }
                Err(err) => {
                    parse_errors.push(StoredParseError {
                        message: err.message().to_string(),
                        offset: err.offset(),
                        length: err.length(),
                    });
                }
            }
        }

        SemanticModel::new(source.to_string(), tokens, comments, parse_errors)
    }

    /// Diagnostics from a prepared model.
    pub(crate) fn diagnostics_prepared(
        &mut self,
        model: &SemanticModel,
        catalog: &DatabaseCatalog,
    ) -> Vec<Diagnostic> {
        self.diagnostics_prepared_with_config_default_ast(
            model,
            catalog,
            &ValidationConfig::default(),
        )
    }

    /// Diagnostics from a prepared model, generic over dialect AST types.
    pub(crate) fn diagnostics_prepared_dialect<A: for<'a> AstTypes<'a>>(
        &mut self,
        model: &SemanticModel,
        catalog: &DatabaseCatalog,
    ) -> Vec<Diagnostic> {
        let config = ValidationConfig::default();
        self.diagnostics_prepared_with_config_dialect::<A>(model, catalog, &config)
    }

    #[cfg(feature = "sqlite")]
    fn diagnostics_prepared_with_config_default_ast(
        &mut self,
        model: &SemanticModel,
        catalog: &DatabaseCatalog,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        self.diagnostics_prepared_with_config_dialect::<syntaqlite_syntax::nodes::SqliteAstMarker>(
            model, catalog, config,
        )
    }

    #[cfg(not(feature = "sqlite"))]
    fn diagnostics_prepared_with_config_default_ast(
        &mut self,
        model: &SemanticModel,
        _catalog: &DatabaseCatalog,
        _config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        self.parse_diagnostics(model)
    }
    // -- Lazy query methods ------------------------------------------------

    /// Parse-error diagnostics extracted from a prepared model.
    pub(crate) fn parse_diagnostics(&self, model: &SemanticModel) -> Vec<Diagnostic> {
        model
            .parse_errors
            .iter()
            .map(|err| parse_error_diagnostic(err, model.source()))
            .collect()
    }

    /// Semantic tokens for syntax highlighting.
    pub(crate) fn semantic_tokens(
        &self,
        model: &SemanticModel,
    ) -> Vec<super::model::SemanticToken> {
        let mut tokens = Vec::new();

        for tp in &model.tokens {
            let cat = self.dialect.classify_token(tp.token_type.into(), tp.flags);
            if cat == TokenCategory::Other {
                continue;
            }
            tokens.push(super::model::SemanticToken {
                offset: tp.offset,
                length: tp.length,
                category: cat,
            });
        }

        for c in &model.comments {
            tokens.push(super::model::SemanticToken {
                offset: c.offset,
                length: c.length,
                category: TokenCategory::Comment,
            });
        }

        tokens.sort_by_key(|t| t.offset);
        tokens
    }

    /// Expected tokens and semantic context at `offset`.
    pub(crate) fn completion_info(
        &self,
        model: &SemanticModel,
        offset: usize,
    ) -> super::model::CompletionInfo {
        let source = model.source();
        let tokens = &model.tokens;
        let cursor_offset = offset.min(source.len());
        let (boundary, backtracked) = completion_boundary(source, tokens, cursor_offset);
        let start = statement_token_start(tokens, boundary);

        let stmt_tokens = &tokens[start..boundary];

        let parser = TypedParser::new(syntaqlite_syntax::typed::grammar());
        let mut cursor = parser.incremental_parse(source);
        let mut last_expected: Vec<TokenType> = cursor.expected_tokens().collect();

        for tok in stmt_tokens {
            let span = tok.offset..(tok.offset + tok.length);
            if cursor.feed_token(tok.token_type, span).is_some() {
                return super::model::CompletionInfo {
                    tokens: last_expected,
                    context: super::model::CompletionContext::from_parser(
                        cursor.completion_context(),
                    ),
                };
            }
            last_expected = cursor.expected_tokens().collect();
        }

        let context = super::model::CompletionContext::from_parser(cursor.completion_context());

        if backtracked {
            let extra = &tokens[boundary];
            let span = extra.offset..(extra.offset + extra.length);
            if cursor.feed_token(extra.token_type, span).is_none() {
                merge_expected_tokens(&mut last_expected, cursor.expected_tokens().collect());
            }
        }

        super::model::CompletionInfo {
            tokens: last_expected,
            context,
        }
    }

    // -- Validation methods ------------------------------------------------

    /// Diagnostics with explicit config, generic over dialect AST types.
    pub(crate) fn diagnostics_prepared_with_config_dialect<A: for<'a> AstTypes<'a>>(
        &mut self,
        model: &SemanticModel,
        catalog: &DatabaseCatalog,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        self.diag_buf.clear();
        self.doc_catalog.clear();

        self.diag_buf.extend(self.parse_diagnostics(model));

        // Re-parse to obtain statement arenas for semantic walking.
        let parser = syntaqlite_syntax::Parser::new();
        let mut session = parser.parse(model.source());

        while let Some(stmt) = session.next() {
            let stmt = match stmt {
                Ok(stmt) => stmt,
                Err(_) => continue,
            };

            let Some(root) = stmt.root() else {
                continue;
            };

            let stmt_id: AnyNodeId = root.node_id().into();
            self.push_statement_diagnostics::<A>(stmt.erase(), stmt_id, catalog, config);
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

fn collect_stmt_positions<'a>(
    source: &'a str,
    tokens: impl Iterator<Item = (&'a str, TokenType, ParserTokenFlags)>,
    comments: impl Iterator<Item = &'a str>,
    out_tokens: &mut Vec<StoredToken>,
    out_comments: &mut Vec<StoredComment>,
) {
    for (text, token_type, flags) in tokens {
        out_tokens.push(StoredToken {
            offset: str_offset(source, text),
            length: text.len(),
            token_type,
            flags,
        });
    }

    for text in comments {
        out_comments.push(StoredComment {
            offset: str_offset(source, text),
            length: text.len(),
        });
    }
}

fn str_offset(source: &str, part: &str) -> usize {
    part.as_ptr() as usize - source.as_ptr() as usize
}

fn parse_error_diagnostic(err: &StoredParseError, source: &str) -> Diagnostic {
    let (start_offset, end_offset) = parse_error_span(err, source);
    Diagnostic {
        start_offset,
        end_offset,
        message: DiagnosticMessage::Other(err.message.clone()),
        severity: Severity::Error,
        help: None,
    }
}

fn completion_boundary(
    source: &str,
    tokens: &[StoredToken],
    cursor_offset: usize,
) -> (usize, bool) {
    let mut boundary = tokens.partition_point(|t| t.offset + t.length <= cursor_offset);

    while boundary > 0 {
        let token = &tokens[boundary - 1];
        if token.length == 0 && token.offset == cursor_offset {
            boundary -= 1;
        } else {
            break;
        }
    }

    let mut backtracked = false;
    if boundary > 0
        && tokens[boundary - 1].offset + tokens[boundary - 1].length == cursor_offset
        && cursor_offset > 0
    {
        let prev = source.as_bytes()[cursor_offset - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            boundary -= 1;
            backtracked = true;
        }
    }

    (boundary, backtracked)
}

fn statement_token_start(tokens: &[StoredToken], boundary: usize) -> usize {
    tokens[..boundary]
        .iter()
        .rposition(|t| t.token_type == TokenType::Semi)
        .map_or(0, |idx| idx + 1)
}

fn merge_expected_tokens(into: &mut Vec<TokenType>, extra: Vec<TokenType>) {
    let mut seen: HashSet<TokenType> = into.iter().copied().collect();
    for token in extra {
        if seen.insert(token) {
            into.push(token);
        }
    }
}

// -- Internal validation functions ------------------------------------------

/// Validate a single parsed statement against the catalog stack.
pub(crate) fn validate_statement_dialect<'a, A: AstTypes<'a>>(
    stmt_result: AnyParsedStatement<'a>,
    stmt_id: AnyNodeId,
    catalog: &'a CatalogStack<'a>,
    config: &'a ValidationConfig,
) -> Vec<Diagnostic> {
    let Some(stmt) = A::Stmt::from_result(stmt_result, stmt_id) else {
        return Vec::new();
    };

    let mut scope = ScopeStack::new(CatalogStack {
        static_: catalog.static_,
        database: catalog.database,
        document: catalog.document,
    });

    Walker::<A>::run(stmt_result, stmt, &mut scope, catalog, config)
}

/// Compute the byte range for a parse error in the source text.
pub(crate) fn parse_error_span(err: &StoredParseError, source: &str) -> (usize, usize) {
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
