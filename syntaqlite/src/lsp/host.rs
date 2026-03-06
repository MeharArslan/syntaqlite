// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use syntaqlite_syntax::any::{AnyTokenType, TokenCategory};
use syntaqlite_syntax::util::is_suggestable_keyword;

use crate::dialect::Dialect;
use crate::fmt::FormatConfig;
use crate::fmt::formatter::Formatter;
use crate::semantic::Catalog;
use crate::semantic::ValidationConfig;
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::diagnostics::Diagnostic;
use crate::semantic::model::{SemanticModel, SemanticToken};

use super::{CompletionEntry, CompletionInfo, CompletionKind};

// ── Document store ────────────────────────────────────────────────────────────

struct Document {
    version: i32,
    source: String,
    /// Cached analysis result. `None` when dirty (source changed or catalog changed).
    model: Option<SemanticModel>,
    /// Parse errors from the last analysis (derived from `model`).
    cached_parse_diags: Option<Vec<Diagnostic>>,
    /// Semantic tokens from the last analysis (derived from `model`).
    cached_sem_tokens: Option<Vec<SemanticToken>>,
}

/// Run analysis for `doc` if no model is cached yet.
fn ensure_model(doc: &mut Document, analyzer: &mut SemanticAnalyzer, user_catalog: &Catalog) {
    if doc.model.is_some() {
        return;
    }
    let model = analyzer.analyze(&doc.source, user_catalog, &ValidationConfig::default());
    let parse_diags = model
        .diagnostics()
        .iter()
        .filter(|d| d.message.is_parse_error())
        .cloned()
        .collect();
    doc.cached_parse_diags = Some(parse_diags);
    doc.model = Some(model);
}

// ── LspHost ───────────────────────────────────────────────────────────────────

/// Manages open documents and answers analysis queries.
///
/// Stores documents by URI and lazily computes per-document analysis
/// (diagnostics, semantic tokens, completions, formatting) on first access
/// after each edit. Semantic validation delegates to [`SemanticAnalyzer`].
pub struct LspHost {
    dialect: Dialect,
    /// User-provided schema (tables, views, functions).
    user_catalog: Catalog,
    analyzer: SemanticAnalyzer,
    documents: HashMap<String, Document>,
}

impl LspHost {
    /// Create a host for the built-in SQLite dialect.
    #[cfg(feature = "sqlite")]
    pub fn new() -> Self {
        let dialect = crate::sqlite::dialect::dialect();
        LspHost {
            user_catalog: Catalog::new(dialect.clone()),
            analyzer: SemanticAnalyzer::new(),
            dialect,
            documents: HashMap::new(),
        }
    }

    /// Create a host bound to `dialect`.
    pub(crate) fn with_dialect(dialect: Dialect) -> Self {
        LspHost {
            user_catalog: Catalog::new(dialect.clone()),
            analyzer: SemanticAnalyzer::with_dialect(dialect.clone()),
            dialect,
            documents: HashMap::new(),
        }
    }

    // ── Configuration ─────────────────────────────────────────────────────────

    /// Set the session context (user-provided schema and functions).
    /// Invalidates all cached analysis.
    pub fn set_session_context(&mut self, ctx: Catalog) {
        self.user_catalog = ctx;
        for doc in self.documents.values_mut() {
            doc.model = None;
            doc.cached_parse_diags = None;
            doc.cached_sem_tokens = None;
        }
    }

    /// Access the current user catalog.
    pub(crate) fn session_context(&self) -> &Catalog {
        &self.user_catalog
    }

    // ── Document lifecycle ─────────────────────────────────────────────────────

    /// Register a newly opened document.
    pub(crate) fn open_document(&mut self, uri: &str, version: i32, text: String) {
        self.documents.insert(
            uri.to_string(),
            Document {
                version,
                source: text,
                model: None,
                cached_parse_diags: None,
                cached_sem_tokens: None,
            },
        );
    }

    /// Update a document's content, invalidating cached analysis.
    pub fn update_document(&mut self, uri: &str, version: i32, text: String) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.version = version;
            doc.source = text;
            doc.model = None;
            doc.cached_parse_diags = None;
            doc.cached_sem_tokens = None;
        } else {
            self.open_document(uri, version, text);
        }
    }

    /// Remove a document from the host.
    pub(crate) fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Source text for a document.
    pub(crate) fn document_source(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|doc| doc.source.as_str())
    }

    // ── Analysis queries ───────────────────────────────────────────────────────

    /// Parse-error diagnostics for a document, lazily computed.
    pub(crate) fn diagnostics(&mut self, uri: &str) -> &[Diagnostic] {
        let Some(doc) = self.documents.get_mut(uri) else {
            return &[];
        };
        ensure_model(doc, &mut self.analyzer, &self.user_catalog);
        doc.cached_parse_diags.as_deref().unwrap()
    }

    /// Version, source text, and parse-error diagnostics in one borrow.
    pub(crate) fn document_diagnostics(&mut self, uri: &str) -> Option<(i32, &str, &[Diagnostic])> {
        let doc = self.documents.get_mut(uri)?;
        ensure_model(doc, &mut self.analyzer, &self.user_catalog);
        let version = doc.version;
        let source = doc.source.as_str();
        let diags = doc.cached_parse_diags.as_deref().unwrap();
        Some((version, source, diags))
    }

    /// Semantic tokens for syntax highlighting, lazily computed.
    pub(crate) fn semantic_tokens(&mut self, uri: &str) -> &[SemanticToken] {
        let Some(doc) = self.documents.get_mut(uri) else {
            return &[];
        };
        ensure_model(doc, &mut self.analyzer, &self.user_catalog);
        if doc.cached_sem_tokens.is_none() {
            let tokens = self.analyzer.semantic_tokens(doc.model.as_ref().unwrap());
            doc.cached_sem_tokens = Some(tokens);
        }
        doc.cached_sem_tokens.as_deref().unwrap()
    }

    /// Semantic tokens delta-encoded for LSP `textDocument/semanticTokens/full`.
    pub fn semantic_tokens_encoded(
        &mut self,
        uri: &str,
        range: Option<(usize, usize)>,
    ) -> Vec<u32> {
        let Some(doc) = self.documents.get_mut(uri) else {
            return Vec::new();
        };
        ensure_model(doc, &mut self.analyzer, &self.user_catalog);
        if doc.cached_sem_tokens.is_none() {
            let tokens = self.analyzer.semantic_tokens(doc.model.as_ref().unwrap());
            doc.cached_sem_tokens = Some(tokens);
        }
        let tokens = doc.cached_sem_tokens.as_deref().unwrap();
        encode_semantic_tokens(&doc.source, tokens, range)
    }

    /// Expected parser tokens and semantic context at a byte offset.
    pub(crate) fn completion_info_at_offset(&mut self, uri: &str, offset: usize) -> CompletionInfo {
        let Some(doc) = self.documents.get_mut(uri) else {
            return CompletionInfo {
                tokens: Vec::new(),
                context: super::CompletionContext::Unknown,
            };
        };
        ensure_model(doc, &mut self.analyzer, &self.user_catalog);
        self.analyzer
            .completion_info(doc.model.as_ref().unwrap(), offset)
    }

    /// Expected terminal token IDs (as `u32` ordinals) at a byte offset.
    pub(crate) fn expected_tokens_at_offset(&mut self, uri: &str, offset: usize) -> Vec<u32> {
        self.completion_info_at_offset(uri, offset)
            .tokens
            .iter()
            .map(|&t| t as u32)
            .collect()
    }

    /// Completion items (keywords + functions) at a byte offset.
    pub fn completion_items(&mut self, uri: &str, offset: usize) -> Vec<CompletionEntry> {
        use std::collections::HashSet;

        let info = self.completion_info_at_offset(uri, offset);
        let expected_set: HashSet<u32> = info.tokens.iter().map(|&t| t as u32).collect();

        let mut seen: HashSet<String> = HashSet::new();
        let mut items: Vec<CompletionEntry> = Vec::new();

        let expects_identifier = expected_set.iter().any(|&tok| {
            self.dialect.token_category(AnyTokenType::from_raw(tok)) == TokenCategory::Identifier
        });

        for entry in self.dialect.keywords() {
            let code = u32::from(entry.token_type);
            if !expected_set.contains(&code) || !is_suggestable_keyword(entry.keyword) {
                continue;
            }
            if seen.insert(entry.keyword.to_string()) {
                items.push(CompletionEntry::new(entry.keyword.to_string(), CompletionKind::Keyword));
            }
        }

        let show_functions = expects_identifier
            && matches!(
                info.context,
                super::CompletionContext::Expression | super::CompletionContext::Unknown
            );

        if show_functions {
            for name in self.available_function_names() {
                if seen.insert(name.clone()) {
                    items.push(CompletionEntry::new(name, CompletionKind::Function));
                }
            }
        }

        items
    }

    // ── Semantic validation ────────────────────────────────────────────────────

    /// Semantic validation diagnostics for a document (non-parse-error issues only).
    ///
    /// Always re-analyzes with `user_catalog` and `config`; use
    /// [`diagnostics`](Self::diagnostics) for the cheaper cached parse-error path.
    #[cfg(feature = "validation")]
    pub(crate) fn validate(&mut self, uri: &str, config: &ValidationConfig) -> Vec<Diagnostic> {
        let Some(source) = self.documents.get(uri).map(|d| d.source.as_str()) else {
            return Vec::new();
        };
        let model = self.analyzer.analyze(source, &self.user_catalog, config);
        model
            .diagnostics()
            .iter()
            .filter(|d| !d.message.is_parse_error())
            .cloned()
            .collect()
    }

    /// Parse + semantic diagnostics combined.
    #[cfg(feature = "validation")]
    pub fn all_diagnostics(
        &mut self,
        uri: &str,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        let mut result = self.diagnostics(uri).to_vec();
        result.extend(self.validate(uri, config));
        result
    }

    // ── Formatting ────────────────────────────────────────────────────────────

    /// Format a document's source text.
    #[cfg(feature = "fmt")]
    pub(crate) fn format(&self, uri: &str, config: &FormatConfig) -> Result<String, FormatError> {
        let doc = self
            .documents
            .get(uri)
            .ok_or(FormatError::UnknownDocument)?;
        let mut formatter = Formatter::with_dialect_config(self.dialect.clone(), config);
        formatter.format(&doc.source).map_err(FormatError::Format)
    }

    // ── Schema helpers ────────────────────────────────────────────────────────

    /// All function names available given the current dialect and user catalog.
    pub fn available_function_names(&self) -> Vec<String> {
        let mut cat = Catalog::new(self.dialect.clone());
        cat.database = self.user_catalog.database.clone();
        cat.all_function_names()
    }

    /// Parse a JSON schema blob and use it as the session context.
    ///
    /// Convenience wrapper over [`set_session_context`] that constructs a
    /// [`Catalog`] using the host's dialect, avoiding the need for callers to
    /// handle `Dialect` directly.
    ///
    /// # Errors
    ///
    /// Returns an error string if `json` is not a valid schema JSON blob.
    #[cfg(feature = "json")]
    pub fn set_session_context_from_json(&mut self, json: &str) -> Result<(), String> {
        let catalog = Catalog::from_json(self.dialect.clone(), json)?;
        self.set_session_context(catalog);
        Ok(())
    }

    /// Parse DDL statements and use the resulting schema as the session context.
    ///
    /// Convenience wrapper over [`set_session_context`] that constructs a
    /// [`Catalog`] using the host's dialect and DDL source, avoiding the need
    /// for callers to handle `Dialect` directly.
    pub fn set_session_context_from_ddl(&mut self, ddl: &str) {
        let catalog = Catalog::from_ddl(self.dialect.clone(), ddl);
        self.set_session_context(catalog);
    }
}

// ── Semantic tokens encoding ───────────────────────────────────────────────────

/// Delta-encode semantic tokens as a flat `u32` array (5 values per token:
/// `deltaLine`, `deltaStartChar`, `length`, `legendIndex`, `modifiers`).
fn encode_semantic_tokens(
    source: &str,
    semantic_tokens: &[SemanticToken],
    range: Option<(usize, usize)>,
) -> Vec<u32> {
    let src = source.as_bytes();
    let (range_start, range_end) = range.unwrap_or((0, src.len()));

    let mut result = Vec::with_capacity(semantic_tokens.len() * 5);
    let mut prev_line: u32 = 0;
    let mut prev_col: u32 = 0;
    let mut cur_line: u32 = 0;
    let mut cur_col: u32 = 0;
    let mut src_pos: usize = 0;

    for tok in semantic_tokens {
        while src_pos < tok.offset && src_pos < src.len() {
            if src[src_pos] == b'\n' {
                cur_line += 1;
                cur_col = 0;
            } else {
                cur_col += 1;
            }
            src_pos += 1;
        }

        if tok.offset < range_start || tok.offset >= range_end {
            continue;
        }
        if tok.category == TokenCategory::Other {
            continue;
        }

        let legend_idx = tok.category as u32;
        let delta_line = cur_line - prev_line;
        let delta_start = if delta_line == 0 {
            cur_col - prev_col
        } else {
            cur_col
        };

        result.push(delta_line);
        result.push(delta_start);
        result.push(tok.length as u32);
        result.push(legend_idx);
        result.push(0);

        prev_line = cur_line;
        prev_col = cur_col;
    }

    result
}

// ── FormatError ───────────────────────────────────────────────────────────────

/// Errors that can occur during formatting.
#[derive(Debug)]
pub(crate) enum FormatError {
    /// The document URI was not found.
    UnknownDocument,
    /// The formatter returned an error.
    Format(crate::fmt::FormatError),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::UnknownDocument => write!(f, "unknown document"),
            FormatError::Format(err) => write!(f, "format error: {err}"),
        }
    }
}

impl std::error::Error for FormatError {}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use syntaqlite_syntax::TokenType;

    use super::LspHost;
    use crate::semantic::Catalog;
    use crate::semantic::ValidationConfig;
    use crate::semantic::diagnostics::{DiagnosticMessage, Severity};

    #[test]
    fn completions_fall_back_to_last_good_state_on_parse_error() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FR";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::From as u32)),
            "expected From after SELECT *, got {:?}",
            expected
        );
    }

    #[test]
    fn completions_ignore_prior_statement_errors_after_semicolon() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "SELEC 1; SELECT * FR";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::From as u32)),
            "expected From in second statement context, got {:?}",
            expected
        );
    }

    #[test]
    fn completions_include_join_after_from_alias_with_partial_next_token() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM s AS x J";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::JoinKw as u32)),
            "expected JoinKw after FROM alias, got {:?}",
            expected
        );
    }

    #[test]
    fn completions_include_join_after_from_table_with_trailing_space() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM slice ";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::Join as u32)),
            "expected Join"
        );
        assert!(
            !expected.contains(&(TokenType::Create as u32)),
            "Create should not appear"
        );
        assert!(
            !expected.contains(&(TokenType::Select as u32)),
            "Select should not appear"
        );
        assert!(
            !expected.contains(&(TokenType::Virtual as u32)),
            "Virtual should not appear"
        );
    }

    #[test]
    fn available_functions_default_config_includes_baseline() {
        let host = LspHost::new();
        let names = host.available_function_names();
        assert!(names.iter().any(|n| n == "abs"));
        assert!(names.iter().any(|n| n == "count"));
    }

    #[test]
    fn available_functions_merges_user_context() {
        let mut host = LspHost::new();
        let dialect = crate::sqlite::dialect::dialect();
        let mut ctx = Catalog::new(dialect);
        ctx.add_function("my_custom_func", Some(2));
        host.set_session_context(ctx);
        let names = host.available_function_names();
        assert!(names.iter().any(|n| n == "my_custom_func"));
        assert!(names.iter().any(|n| n == "abs"));
    }

    #[test]
    fn completion_context_after_from_is_table_ref() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT acos() as foo FROM ";
        host.open_document(uri, 1, sql.to_string());
        let info = host.completion_info_at_offset(uri, sql.len());
        assert_eq!(info.context, super::super::CompletionContext::TableRef);
    }

    #[test]
    fn completion_context_after_select_is_not_table_ref() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT ";
        host.open_document(uri, 1, sql.to_string());
        let info = host.completion_info_at_offset(uri, sql.len());
        assert_ne!(info.context, super::super::CompletionContext::TableRef);
    }

    #[test]
    fn completion_context_after_where_is_expression() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM t WHERE ";
        host.open_document(uri, 1, sql.to_string());
        let info = host.completion_info_at_offset(uri, sql.len());
        assert_eq!(info.context, super::super::CompletionContext::Expression);
    }

    #[test]
    fn completions_include_join_after_from_table_no_trailing_space() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM slice";
        host.open_document(uri, 1, sql.to_string());
        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(expected.contains(&(TokenType::Join as u32)));
    }

    #[test]
    fn validate_select_after_create_table_as_select_no_diags() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(
            uri,
            1,
            "CREATE TABLE orders AS SELECT 1 AS order_id;\nSELECT o.order_id FROM orders o;"
                .to_string(),
        );
        let diags = host.validate(uri, &ValidationConfig::default());
        assert!(diags.is_empty(), "unexpected diagnostics: {:?}", diags);
    }

    #[test]
    fn validate_select_from_unknown_table_still_warns() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT * FROM nonexistent;".to_string());
        let diags = host.validate(uri, &ValidationConfig::default());
        assert!(!diags.is_empty());
    }

    #[test]
    fn validate_forward_reference_warns() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(
            uri,
            1,
            "SELECT * FROM t;\nCREATE TABLE t (id INTEGER);".to_string(),
        );
        let diags = host.validate(uri, &ValidationConfig::default());
        assert!(!diags.is_empty());
    }

    #[test]
    fn syntax_error_produces_diagnostic_for_bare_select() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT ".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert!(!diags.is_empty());
        assert_eq!(diags[0].severity, Severity::Error);
    }

    #[test]
    fn syntax_error_produces_diagnostic_for_incomplete_from() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT * FROM".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert!(!diags.is_empty());
    }

    #[test]
    fn validation_returns_error_for_syntax_invalid_sql() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "NOT VALID SQL;".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert!(!diags.is_empty());
    }

    #[test]
    fn multiple_syntax_errors_all_reported() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "include ;\ninclude ;\nSELECT 1;".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        let errors: Vec<_> = diags
            .iter()
            .filter(|d| d.severity == Severity::Error)
            .collect();
        assert_eq!(errors.len(), 2, "got {}: {:?}", errors.len(), errors);
    }

    #[test]
    fn syntax_errors_do_not_suppress_later_valid_statements() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "NOT VALID;\nSELECT 1;".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert_eq!(diags.len(), 1, "got {}: {:?}", diags.len(), diags);
    }

    #[test]
    fn syntax_error_after_valid_statement_is_reported() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT 1;\nNOT VALID;".to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert_eq!(diags.len(), 1, "got {}: {:?}", diags.len(), diags);
    }

    #[test]
    fn validate_does_not_duplicate_parse_error_diagnostics() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT ;\nSELECT 1;".to_string());
        let diags = host.validate(uri, &ValidationConfig::default());
        assert_eq!(diags.len(), 0, "got: {:?}", diags);
    }

    #[test]
    fn validate_continues_past_errors_to_check_later_statements() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(
            uri,
            1,
            "SELECT ;\nSELECT ;\nSELECT * FROM no_such_table;".to_string(),
        );
        let diags = host.validate(uri, &ValidationConfig::default());
        let table_diags: Vec<_> = diags
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert_eq!(table_diags.len(), 1, "got: {:?}", diags);
    }

    #[test]
    fn syntax_error_offset_points_at_error_token() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        let sql = "select 1 from slice where foo = where x = y;";
        host.open_document(uri, 1, sql.to_string());
        let (_, _, diags) = host.document_diagnostics(uri).unwrap();
        assert!(!diags.is_empty());
        let diag = &diags[0];
        assert_eq!(diag.severity, Severity::Error);
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
    fn parse_and_validate_combined_no_duplicates() {
        let mut host = LspHost::new();
        let uri = "file:///test.sql";
        host.open_document(uri, 1, "SELECT ;\nSELECT * FROM no_such_table;".to_string());
        let parse_diags = host.diagnostics(uri).to_vec();
        let val_diags = host.validate(uri, &ValidationConfig::default());
        let all: Vec<_> = parse_diags.iter().chain(val_diags.iter()).collect();
        let errors = all.iter().filter(|d| d.severity == Severity::Error).count();
        let warnings = all
            .iter()
            .filter(|d| d.severity == Severity::Warning)
            .count();
        assert_eq!(errors, 1, "got {}: {:?}", errors, all);
        assert_eq!(warnings, 1, "got {}: {:?}", warnings, all);
    }
}
