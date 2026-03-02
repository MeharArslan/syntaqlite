// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use crate::dialect::Dialect;
use crate::fmt::FormatConfig;
use crate::fmt::formatter::Formatter;
use crate::lsp::analysis::DocumentAnalysis;
use crate::parser::session::ParseError;
use crate::validation::types::{Diagnostic, FunctionDef, SessionContext};
use crate::validation::{FunctionCatalog, ValidationConfig};

use super::{CompletionEntry, CompletionInfo, CompletionKind, SemanticToken};

// ── Document store ────────────────────────────────────────────────────────

struct Document {
    version: i32,
    source: String,
    analysis: Option<DocumentAnalysis>,
}

impl Document {
    fn analysis(&mut self, dialect: Dialect<'_>) -> &DocumentAnalysis {
        if self.analysis.is_none() {
            self.analysis = Some(DocumentAnalysis::compute(dialect, &self.source));
        }
        self.analysis.as_ref().expect("just populated")
    }
}

// ── AnalysisHost ──────────────────────────────────────────────────────────

/// Manages open documents and answers analysis queries.
///
/// The host stores documents by URI and lazily computes per-document
/// analysis (diagnostics, semantic tokens, completion tokens) on first
/// access after each edit.  Heavy analysis is delegated to
/// [`DocumentAnalysis`] and [`FunctionCatalog`].
pub struct AnalysisHost<'d> {
    dialect: Dialect<'d>,
    documents: HashMap<String, Document>,
    context: Option<SessionContext>,
    dialect_config: Option<syntaqlite_parser::dialect::ffi::DialectConfig>,
}

impl<'d> AnalysisHost<'d> {
    /// Create a host bound to `dialect`.
    pub fn with_dialect(dialect: Dialect<'d>) -> Self {
        AnalysisHost {
            dialect,
            documents: HashMap::new(),
            context: None,
            dialect_config: None,
        }
    }

    /// Create a host for the built-in SQLite dialect.
    #[cfg(feature = "sqlite")]
    pub fn new() -> AnalysisHost<'static> {
        AnalysisHost::with_dialect(*crate::sqlite::DIALECT)
    }

    // ── Configuration ─────────────────────────────────────────────────────

    /// Set the session context (user-provided schema and functions).
    pub fn set_session_context(&mut self, ctx: SessionContext) {
        self.context = Some(ctx);
    }

    /// Access the current session context.
    pub fn session_context(&self) -> Option<&SessionContext> {
        self.context.as_ref()
    }

    /// Set the dialect configuration for version/cflag-gated function filtering.
    pub fn set_dialect_config(&mut self, config: syntaqlite_parser::dialect::ffi::DialectConfig) {
        self.dialect_config = Some(config);
    }

    // ── Document lifecycle ─────────────────────────────────────────────────

    /// Register a newly opened document.
    pub fn open_document(&mut self, uri: &str, version: i32, text: String) {
        self.documents.insert(
            uri.to_string(),
            Document {
                version,
                source: text,
                analysis: None,
            },
        );
    }

    /// Update a document's content, invalidating cached analysis.
    pub fn update_document(&mut self, uri: &str, version: i32, text: String) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.version = version;
            doc.source = text;
            doc.analysis = None;
        } else {
            self.open_document(uri, version, text);
        }
    }

    /// Remove a document from the host.
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Source text for a document.
    pub fn document_source(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|doc| doc.source.as_str())
    }

    // ── Analysis queries ───────────────────────────────────────────────────

    /// Parse-error diagnostics for a document, lazily computed.
    pub fn diagnostics(&mut self, uri: &str) -> &[Diagnostic] {
        match self.documents.get_mut(uri) {
            Some(doc) => doc.analysis(self.dialect).diagnostics(),
            None => &[],
        }
    }

    /// Version, source text, and parse-error diagnostics in one borrow.
    pub fn document_diagnostics(&mut self, uri: &str) -> Option<(i32, &str, &[Diagnostic])> {
        let doc = self.documents.get_mut(uri)?;
        doc.analysis(self.dialect);
        let version = doc.version;
        let source = doc.source.as_str();
        let diagnostics = doc.analysis.as_ref().expect("just computed").diagnostics();
        Some((version, source, diagnostics))
    }

    /// Semantic tokens for syntax highlighting, lazily computed.
    pub fn semantic_tokens(&mut self, uri: &str) -> &[SemanticToken] {
        match self.documents.get_mut(uri) {
            Some(doc) => doc.analysis(self.dialect).semantic_tokens(),
            None => &[],
        }
    }

    /// Semantic tokens delta-encoded for LSP `textDocument/semanticTokens/full`.
    pub fn semantic_tokens_encoded(
        &mut self,
        uri: &str,
        range: Option<(usize, usize)>,
    ) -> Vec<u32> {
        match self.documents.get_mut(uri) {
            Some(doc) => {
                let source = doc.source.clone();
                doc.analysis(self.dialect)
                    .semantic_tokens_encoded(&source, range)
            }
            None => Vec::new(),
        }
    }

    /// Expected parser tokens and semantic context at a byte offset.
    pub fn completion_info_at_offset(&mut self, uri: &str, offset: usize) -> CompletionInfo {
        match self.documents.get_mut(uri) {
            Some(doc) => {
                let source = doc.source.clone();
                doc.analysis(self.dialect)
                    .completion_info_at(self.dialect, &source, offset)
            }
            None => CompletionInfo {
                tokens: Vec::new(),
                context: super::CompletionContext::Unknown,
            },
        }
    }

    /// Expected terminal token IDs at a byte offset.
    pub fn expected_tokens_at_offset(&mut self, uri: &str, offset: usize) -> Vec<u32> {
        self.completion_info_at_offset(uri, offset).tokens
    }

    /// Completion items (keywords + functions) at a byte offset.
    pub fn completion_items(&mut self, uri: &str, offset: usize) -> Vec<CompletionEntry> {
        use crate::dialect::TokenCategory;
        use std::collections::HashSet;

        let info = self.completion_info_at_offset(uri, offset);
        let expected_set: HashSet<u32> = info.tokens.iter().copied().collect();

        let mut seen: HashSet<String> = HashSet::new();
        let mut items: Vec<CompletionEntry> = Vec::new();

        let expects_identifier = expected_set.iter().any(|&tok| {
            TokenCategory::from_u8(self.dialect.token_category_raw(tok))
                == TokenCategory::Identifier
        });

        for i in 0..self.dialect.keyword_count() {
            let Some((code, name)) = self.dialect.keyword_entry(i) else {
                continue;
            };
            if !expected_set.contains(&code) || !Dialect::is_suggestable_keyword(name) {
                continue;
            }
            if seen.insert(name.to_string()) {
                items.push(CompletionEntry {
                    label: name.to_string(),
                    kind: CompletionKind::Keyword,
                });
            }
        }

        let show_functions = expects_identifier
            && matches!(
                info.context,
                super::CompletionContext::Expression | super::CompletionContext::Unknown
            );

        if show_functions {
            let catalog = self.function_catalog();
            for name in catalog.unique_names() {
                if seen.insert(name.to_string()) {
                    items.push(CompletionEntry {
                        label: name.to_string(),
                        kind: CompletionKind::Function,
                    });
                }
            }
        }

        items
    }

    /// Format a document's source text.
    pub fn format(&self, uri: &str, config: &FormatConfig) -> Result<String, FormatError> {
        let doc = self
            .documents
            .get(uri)
            .ok_or(FormatError::UnknownDocument)?;
        let mut formatter = Formatter::builder(self.dialect)
            .format_config(config.clone())
            .build();
        formatter.format(&doc.source).map_err(FormatError::Parse)
    }

    // ── Semantic validation ────────────────────────────────────────────────

    /// Semantic validation diagnostics for a document, generic over dialect AST types.
    pub fn validate_dialect<A: for<'a> crate::ast_traits::AstTypes<'a>>(
        &self,
        uri: &str,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        let Some(doc) = self.documents.get(uri) else {
            return Vec::new();
        };

        let catalog = self.function_catalog();
        let mut parser = crate::parser::session::RawParser::builder(self.dialect).build();
        let mut cursor = parser.parse(&doc.source);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            match result {
                Ok(node_ref) => stmt_ids.push(node_ref.id()),
                Err(err) => {
                    if let Some(id) = err.root {
                        stmt_ids.push(id);
                    }
                }
            }
        }

        let mut doc_ctx = crate::validation::DocumentContext::new();
        let reader = cursor.reader();
        let mut diagnostics = Vec::new();

        for &stmt_id in &stmt_ids {
            let stmt_diags = crate::validation::validate_statement_dialect::<A>(
                reader,
                stmt_id,
                self.dialect,
                self.context.as_ref(),
                Some(&doc_ctx),
                catalog.functions(),
                config,
            );
            diagnostics.extend(stmt_diags);

            #[cfg(feature = "sqlite")]
            doc_ctx.accumulate(reader, stmt_id, self.dialect, self.context.as_ref());
        }

        diagnostics
    }

    /// Semantic validation using the built-in SQLite dialect.
    #[cfg(feature = "sqlite")]
    pub fn validate(&self, uri: &str, config: &ValidationConfig) -> Vec<Diagnostic> {
        self.validate_dialect::<syntaqlite_parser_sqlite::ast::SqliteAst>(uri, config)
    }

    /// Parse + semantic diagnostics combined.
    #[cfg(feature = "sqlite")]
    pub fn all_diagnostics(&mut self, uri: &str, config: &ValidationConfig) -> Vec<Diagnostic> {
        let mut result = self.diagnostics(uri).to_vec();
        result.extend(self.validate(uri, config));
        result
    }

    // ── Function catalog ──────────────────────────────────────────────────

    /// Build the function catalog for the current dialect configuration and
    /// session context.
    pub fn function_catalog(&self) -> FunctionCatalog {
        let default_config = syntaqlite_parser::dialect::ffi::DialectConfig::default();
        let config = self.dialect_config.as_ref().unwrap_or(&default_config);
        let catalog = FunctionCatalog::for_dialect(&self.dialect, config);
        match self.context.as_ref() {
            Some(ctx) => catalog.with_session(ctx),
            None => catalog,
        }
    }

    /// All function definitions for the current configuration.
    pub fn available_functions(&self) -> Vec<FunctionDef> {
        self.function_catalog().functions().to_vec()
    }

    /// Unique function names for the current configuration.
    pub fn available_function_names(&self) -> Vec<String> {
        self.function_catalog()
            .unique_names()
            .map(|s| s.to_string())
            .collect()
    }
}

// ── FormatError ───────────────────────────────────────────────────────────

/// Errors that can occur during formatting.
#[derive(Debug)]
pub enum FormatError {
    /// The document URI was not found.
    UnknownDocument,
    /// Parse error during formatting.
    Parse(ParseError),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::UnknownDocument => write!(f, "unknown document"),
            FormatError::Parse(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl std::error::Error for FormatError {}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use super::AnalysisHost;
    use crate::parser::session::RawParser;
    use crate::validation::SessionContext;
    use crate::validation::types::FunctionDef;
    use syntaqlite_parser_sqlite::tokens::TokenType;

    #[test]
    fn completions_fall_back_to_last_good_state_on_parse_error() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FR";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::FROM as u32)),
            "expected TK_FROM after SELECT *, got {:?}",
            expected
        );
    }

    #[test]
    fn completions_ignore_prior_statement_errors_after_semicolon() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELEC 1; SELECT * FR";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::FROM as u32)),
            "expected TK_FROM in second statement context, got {:?}",
            expected
        );
    }

    #[test]
    fn completions_include_join_after_from_alias_with_partial_next_token() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM s AS x J";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::JOINKW as u32)),
            "expected TK_JOIN_KW after FROM alias, got {:?}",
            expected
        );
    }

    #[test]
    fn completions_include_join_after_from_table_with_trailing_space() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM slice ";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::JOIN as u32)),
            "expected TK_JOIN"
        );
        assert!(
            !expected.contains(&(TokenType::CREATE as u32)),
            "TK_CREATE should not appear"
        );
        assert!(
            !expected.contains(&(TokenType::SELECT as u32)),
            "TK_SELECT should not appear"
        );
        assert!(
            !expected.contains(&(TokenType::VIRTUAL as u32)),
            "TK_VIRTUAL should not appear"
        );
    }

    #[test]
    fn available_functions_default_config_includes_baseline() {
        let host = AnalysisHost::new();
        let funcs = host.available_functions();
        let names: Vec<&str> = funcs.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"abs"));
        assert!(names.contains(&"count"));
        assert!(
            !names.contains(&"acos"),
            "acos requires ENABLE_MATH_FUNCTIONS"
        );
    }

    #[test]
    fn available_functions_with_config_filters_by_cflags() {
        let mut host = AnalysisHost::new();
        let mut config = syntaqlite_parser::dialect::ffi::DialectConfig::default();
        config.cflags.set(34);
        host.set_dialect_config(config);
        let funcs = host.available_functions();
        let names: Vec<&str> = funcs.iter().map(|f| f.name.as_str()).collect();
        assert!(
            names.contains(&"acos"),
            "acos should appear with ENABLE_MATH_FUNCTIONS"
        );
    }

    #[test]
    fn available_functions_merges_ambient_context() {
        let mut host = AnalysisHost::new();
        host.set_session_context(SessionContext {
            relations: vec![],
            functions: vec![FunctionDef {
                name: "my_custom_func".to_string(),
                args: Some(2),
                description: None,
            }],
        });
        let funcs = host.available_functions();
        let names: Vec<&str> = funcs.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"my_custom_func"));
        assert!(names.contains(&"abs"));
    }

    #[test]
    fn completion_context_after_from_is_table_ref() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT acos() as foo FROM ";
        host.open_document(uri, 1, sql.to_string());
        let info = host.completion_info_at_offset(uri, sql.len());
        assert_eq!(info.context, super::super::CompletionContext::TableRef);
    }

    #[test]
    fn completion_context_after_select_is_not_table_ref() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT ";
        host.open_document(uri, 1, sql.to_string());
        let info = host.completion_info_at_offset(uri, sql.len());
        assert_ne!(info.context, super::super::CompletionContext::TableRef);
    }

    #[test]
    fn completion_context_after_where_is_expression() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM t WHERE ";
        host.open_document(uri, 1, sql.to_string());
        let info = host.completion_info_at_offset(uri, sql.len());
        assert_eq!(info.context, super::super::CompletionContext::Expression);
    }

    #[test]
    fn completions_include_join_after_from_table_no_trailing_space() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM slice";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(expected.contains(&(TokenType::JOIN as u32)));
    }

    #[test]
    fn validate_select_after_create_table_as_select_no_diags() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(
            uri,
            1,
            "CREATE TABLE orders AS SELECT 1 AS order_id;\nSELECT o.order_id FROM orders o;"
                .to_string(),
        );
        let diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
    }

    #[test]
    fn validate_select_from_unknown_table_still_warns() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT * FROM nonexistent;".to_string());
        let diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        assert!(!diags.is_empty());
    }

    #[test]
    fn validate_forward_reference_warns() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(
            uri,
            1,
            "SELECT * FROM t;\nCREATE TABLE t (id INTEGER);".to_string(),
        );
        let diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        assert!(!diags.is_empty());
    }

    #[test]
    fn syntax_error_produces_diagnostic_for_bare_select() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT ".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert!(!diags.is_empty());
        assert_eq!(diags[0].severity, crate::validation::Severity::Error);
    }

    #[test]
    fn syntax_error_produces_diagnostic_for_incomplete_from() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT * FROM".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert!(!diags.is_empty());
    }

    #[test]
    fn validation_returns_error_for_syntax_invalid_sql() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "NOT VALID SQL;".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert!(!diags.is_empty());
    }

    #[test]
    fn multiple_syntax_errors_all_reported() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "include ;\ninclude ;\nSELECT 1;".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == crate::validation::Severity::Error)
            .collect();
        assert_eq!(errors.len(), 2, "got {}: {:?}", errors.len(), errors);
    }

    #[test]
    fn syntax_errors_do_not_suppress_later_valid_statements() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "NOT VALID;\nSELECT 1;".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert_eq!(diags.len(), 1, "got {}: {:?}", diags.len(), diags);
    }

    #[test]
    fn syntax_error_after_valid_statement_is_reported() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT 1;\nNOT VALID;".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert_eq!(diags.len(), 1, "got {}: {:?}", diags.len(), diags);
    }

    #[test]
    fn validate_does_not_duplicate_parse_error_diagnostics() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT ;\nSELECT 1;".to_string());
        let diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        assert_eq!(diags.len(), 0, "got: {:?}", diags);
    }

    #[test]
    fn validate_continues_past_errors_to_check_later_statements() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(
            uri,
            1,
            "SELECT ;\nSELECT ;\nSELECT * FROM no_such_table;".to_string(),
        );
        let diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        let table_diags: Vec<_> = diags
            .iter()
            .filter(|d| {
                matches!(
                    &d.message,
                    crate::validation::DiagnosticMessage::UnknownTable { .. }
                )
            })
            .collect();
        assert_eq!(table_diags.len(), 1, "got: {:?}", diags);
    }

    #[test]
    fn syntax_error_offset_points_at_error_token_not_following_token() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "select 1 from slice where foo = where x = y;";
        host.open_document(uri, 1, sql.to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert!(!diags.is_empty());
        let diag = &diags[0];
        assert_eq!(diag.severity, crate::validation::Severity::Error);
        let second_where = sql[31..].find("where").map(|i| i + 31).unwrap();
        assert_eq!(
            diag.start_offset,
            second_where,
            "got '{}' at {}",
            &sql[diag.start_offset..diag.start_offset + 1],
            diag.start_offset
        );
    }

    #[test]
    fn syntax_error_offset_via_parser_directly() {
        let sql = "select 1 from slice where foo = where x = y;";
        let mut parser = RawParser::new();
        let mut cursor = parser.parse(sql);
        let err = cursor
            .next_statement()
            .expect("Some")
            .expect_err("parse error");
        assert!(err.message.contains("where"), "got: {}", err.message);
        let offset = err.offset.expect("has offset");
        let second_where = sql[31..].find("where").map(|i| i + 31).unwrap();
        assert_eq!(
            offset,
            second_where,
            "got '{}' at {}",
            &sql[offset..offset + 1],
            offset
        );
    }

    #[test]
    fn parse_and_validate_combined_no_duplicates() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT ;\nSELECT * FROM no_such_table;".to_string());
        let parse_diags = host.diagnostics(uri).to_vec();
        let val_diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        let all: Vec<_> = parse_diags.iter().chain(val_diags.iter()).collect();
        let errors = all
            .iter()
            .filter(|d| d.severity == crate::validation::Severity::Error)
            .count();
        let warnings = all
            .iter()
            .filter(|d| d.severity == crate::validation::Severity::Warning)
            .count();
        assert_eq!(errors, 1, "got {}: {:?}", errors, all);
        assert_eq!(warnings, 1, "got {}: {:?}", warnings, all);
    }
}
