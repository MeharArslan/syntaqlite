// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use crate::dialect::TokenCategory;
use crate::fmt::FormatConfig;
use crate::fmt::formatter::Formatter;
use crate::semantic::DatabaseCatalog;
use crate::semantic::SemanticModel;
use crate::semantic::ValidationConfig;
use crate::semantic::analyzer::SemanticAnalyzer;
use crate::semantic::catalog::{CatalogStack, DocumentCatalog, StaticCatalog};
use crate::semantic::diagnostics::Diagnostic;
use crate::semantic::model::SemanticToken;
use syntaqlite_parser::DialectEnv;
use syntaqlite_parser::ParseError;

use super::{CompletionEntry, CompletionInfo, CompletionKind};

// ── Document store ────────────────────────────────────────────────────────

struct Document<'d> {
    version: i32,
    source: String,
    model: Option<SemanticModel<'d>>,
    /// Lazy cache — computed on first access, invalidated with model.
    cached_parse_diags: Option<Vec<Diagnostic>>,
    /// Lazy cache — computed on first access, invalidated with model.
    cached_semantic_tokens: Option<Vec<SemanticToken>>,
}

/// Ensure the document has a prepared `SemanticModel`, creating one if needed.
fn ensure_model<'d>(doc: &mut Document<'d>, analyzer: &mut SemanticAnalyzer<'d>) {
    if doc.model.is_none() {
        let model = analyzer.prepare(&doc.source);
        doc.model = Some(model);
    }
}

// ── LspHost ──────────────────────────────────────────────────────────────

/// Manages open documents and answers analysis queries.
///
/// Stores documents by URI and lazily computes per-document analysis
/// (diagnostics, semantic tokens, completion tokens) on first access after
/// each edit. Semantic validation delegates to [`SemanticAnalyzer`].
pub(crate) struct LspHost<'d> {
    dialect: DialectEnv<'d>,
    documents: HashMap<String, Document<'d>>,
    context: Option<DatabaseCatalog>,
    analyzer: SemanticAnalyzer<'d>,
}

impl<'d> LspHost<'d> {
    /// Create a host bound to `dialect`.
    pub(crate) fn with_dialect(dialect: impl Into<DialectEnv<'d>>) -> Self {
        let dialect = dialect.into();
        let analyzer = SemanticAnalyzer::with_dialect(dialect);
        LspHost {
            dialect,
            documents: HashMap::new(),
            context: None,
            analyzer,
        }
    }

    /// Create a host for the built-in SQLite dialect.
    #[cfg(feature = "sqlite")]
    pub(crate) fn new() -> LspHost<'static> {
        LspHost::with_dialect(crate::dialect::sqlite())
    }

    // ── Configuration ─────────────────────────────────────────────────────

    /// Set the session context (user-provided schema and functions).
    pub(crate) fn set_session_context(&mut self, ctx: DatabaseCatalog) {
        self.context = Some(ctx);
    }

    /// Access the current session context.
    pub(crate) fn session_context(&self) -> Option<&DatabaseCatalog> {
        self.context.as_ref()
    }

    /// Update the dialect environment (version/cflags). Rebuilds the analyzer.
    pub(crate) fn set_dialect_env(&mut self, env: DialectEnv<'d>) {
        self.dialect = env;
        self.analyzer = SemanticAnalyzer::with_dialect(env);
    }

    // ── Document lifecycle ─────────────────────────────────────────────────

    /// Register a newly opened document.
    pub(crate) fn open_document(&mut self, uri: &str, version: i32, text: String) {
        self.documents.insert(
            uri.to_string(),
            Document {
                version,
                source: text,
                model: None,
                cached_parse_diags: None,
                cached_semantic_tokens: None,
            },
        );
    }

    /// Update a document's content, invalidating cached analysis.
    pub(crate) fn update_document(&mut self, uri: &str, version: i32, text: String) {
        if let Some(doc) = self.documents.get_mut(uri) {
            doc.version = version;
            doc.source = text;
            doc.model = None;
            doc.cached_parse_diags = None;
            doc.cached_semantic_tokens = None;
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

    // ── Analysis queries ───────────────────────────────────────────────────

    /// Parse-error diagnostics for a document, lazily computed.
    pub(crate) fn diagnostics(&mut self, uri: &str) -> &[Diagnostic] {
        let Some(doc) = self.documents.get_mut(uri) else {
            return &[];
        };
        ensure_model(doc, &mut self.analyzer);
        if doc.cached_parse_diags.is_none() {
            let model = doc.model.as_ref().unwrap();
            let diags = self.analyzer.parse_diagnostics(model);
            doc.cached_parse_diags = Some(diags);
        }
        doc.cached_parse_diags.as_deref().unwrap()
    }

    /// Version, source text, and parse-error diagnostics in one borrow.
    pub(crate) fn document_diagnostics(&mut self, uri: &str) -> Option<(i32, &str, &[Diagnostic])> {
        let doc = self.documents.get_mut(uri)?;
        ensure_model(doc, &mut self.analyzer);
        if doc.cached_parse_diags.is_none() {
            let model = doc.model.as_ref().unwrap();
            let diags = self.analyzer.parse_diagnostics(model);
            doc.cached_parse_diags = Some(diags);
        }
        let version = doc.version;
        let source = doc.source.as_str();
        let diagnostics = doc.cached_parse_diags.as_deref().unwrap();
        Some((version, source, diagnostics))
    }

    /// Semantic tokens for syntax highlighting, lazily computed.
    pub(crate) fn semantic_tokens(&mut self, uri: &str) -> &[SemanticToken] {
        let Some(doc) = self.documents.get_mut(uri) else {
            return &[];
        };
        ensure_model(doc, &mut self.analyzer);
        if doc.cached_semantic_tokens.is_none() {
            let model = doc.model.as_ref().unwrap();
            let tokens = self.analyzer.semantic_tokens(model);
            doc.cached_semantic_tokens = Some(tokens);
        }
        doc.cached_semantic_tokens.as_deref().unwrap()
    }

    /// Semantic tokens delta-encoded for LSP `textDocument/semanticTokens/full`.
    pub(crate) fn semantic_tokens_encoded(
        &mut self,
        uri: &str,
        range: Option<(usize, usize)>,
    ) -> Vec<u32> {
        let Some(doc) = self.documents.get_mut(uri) else {
            return Vec::new();
        };
        ensure_model(doc, &mut self.analyzer);
        if doc.cached_semantic_tokens.is_none() {
            let model = doc.model.as_ref().unwrap();
            let tokens = self.analyzer.semantic_tokens(model);
            doc.cached_semantic_tokens = Some(tokens);
        }
        let tokens = doc.cached_semantic_tokens.as_deref().unwrap();
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
        ensure_model(doc, &mut self.analyzer);
        let model = doc.model.as_ref().unwrap();
        self.analyzer.completion_info(model, offset)
    }

    /// Expected terminal token IDs at a byte offset.
    pub(crate) fn expected_tokens_at_offset(&mut self, uri: &str, offset: usize) -> Vec<u32> {
        self.completion_info_at_offset(uri, offset).tokens
    }

    /// Completion items (keywords + functions) at a byte offset.
    pub(crate) fn completion_items(&mut self, uri: &str, offset: usize) -> Vec<CompletionEntry> {
        use crate::dialect::DialectExt;
        use std::collections::HashSet;

        let info = self.completion_info_at_offset(uri, offset);
        let expected_set: HashSet<u32> = info.tokens.iter().copied().collect();

        let mut seen: HashSet<String> = HashSet::new();
        let mut items: Vec<CompletionEntry> = Vec::new();

        let expects_identifier = expected_set
            .iter()
            .any(|&tok| self.dialect.token_category(tok) == TokenCategory::Identifier);

        for i in 0..self.dialect.keyword_count() {
            let Some((code, name)) = self.dialect.keyword_entry(i) else {
                continue;
            };
            if !expected_set.contains(&code) || !DialectEnv::is_suggestable_keyword(name) {
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
            for name in self.available_function_names() {
                if seen.insert(name.to_string()) {
                    items.push(CompletionEntry {
                        label: name,
                        kind: CompletionKind::Function,
                    });
                }
            }
        }

        items
    }

    /// Format a document's source text.
    pub(crate) fn format(&self, uri: &str, config: &FormatConfig) -> Result<String, FormatError> {
        let doc = self
            .documents
            .get(uri)
            .ok_or(FormatError::UnknownDocument)?;
        let mut formatter = Formatter::with_dialect_config(self.dialect, config);
        formatter.format(&doc.source).map_err(FormatError::Parse)
    }

    // ── Semantic validation ────────────────────────────────────────────────

    /// Semantic validation diagnostics for a document.
    ///
    /// Returns only semantic diagnostics (unknown tables, columns, functions,
    /// wrong arity). Parse-error diagnostics come from [`diagnostics()`](Self::diagnostics).
    #[cfg(feature = "sqlite")]
    pub(crate) fn validate(&mut self, uri: &str, config: &ValidationConfig) -> Vec<Diagnostic> {
        let Some(doc) = self.documents.get_mut(uri) else {
            return Vec::new();
        };
        ensure_model(doc, &mut self.analyzer);
        let model = doc.model.as_ref().unwrap();
        let empty_db = DatabaseCatalog::default();
        let catalog = self.context.as_ref().unwrap_or(&empty_db);
        self.analyzer
            .diagnostics_prepared_with_config_dialect::<syntaqlite_parser_sqlite::ast::SqliteAst>(
                model, catalog, config,
            )
            .into_iter()
            .filter(|d| !d.message.is_parse_error())
            .collect()
    }

    /// Parse + semantic diagnostics combined.
    #[cfg(feature = "sqlite")]
    pub(crate) fn all_diagnostics(
        &mut self,
        uri: &str,
        config: &ValidationConfig,
    ) -> Vec<Diagnostic> {
        let mut result = self.diagnostics(uri).to_vec();
        result.extend(self.validate(uri, config));
        result
    }

    /// Unique function names for the current configuration.
    pub(crate) fn available_function_names(&self) -> Vec<String> {
        let static_catalog = StaticCatalog::for_dialect(&self.dialect);
        let empty_db = DatabaseCatalog::default();
        let database = self.context.as_ref().unwrap_or(&empty_db);
        let document = DocumentCatalog::new();
        let stack = CatalogStack {
            static_: &static_catalog,
            database,
            document: &document,
        };
        stack.all_function_names()
    }
}

// ── Semantic tokens encoding ──────────────────────────────────────────────

/// Delta-encode semantic tokens as a flat `u32` array (5 values per token:
/// `deltaLine`, `deltaStartChar`, `length`, `legendIndex`, `modifiers`).
///
/// This is the format expected by Monaco/LSP `textDocument/semanticTokens/full`.
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

        if tok.offset < range_start {
            continue;
        }
        if tok.offset >= range_end {
            break;
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

// ── FormatError ───────────────────────────────────────────────────────────

/// Errors that can occur during formatting.
#[derive(Debug)]
pub(crate) enum FormatError {
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
    use super::LspHost;
    use crate::semantic::DatabaseCatalog;
    use crate::semantic::ValidationConfig;
    use crate::semantic::diagnostics::{DiagnosticMessage, Severity};
    use crate::semantic::schema::FunctionDef;
    use syntaqlite_parser::Parser;
    use syntaqlite_parser_sqlite::tokens::TokenType;

    #[test]
    fn completions_fall_back_to_last_good_state_on_parse_error() {
        let mut host = LspHost::new();
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
        let mut host = LspHost::new();
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
        let mut host = LspHost::new();
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
        let mut host = LspHost::new();
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
        let host = LspHost::new();
        let names = host.available_function_names();
        assert!(names.iter().any(|n| n == "abs"));
        assert!(names.iter().any(|n| n == "count"));
        assert!(
            !names.iter().any(|n| n == "acos"),
            "acos requires ENABLE_MATH_FUNCTIONS"
        );
    }

    #[test]
    fn available_functions_with_config_filters_by_cflags() {
        let mut host = LspHost::new();
        let env = crate::dialect::sqlite().with_cflag(34);
        host.set_dialect_env(env);
        let names = host.available_function_names();
        assert!(
            names.iter().any(|n| n == "acos"),
            "acos should appear with ENABLE_MATH_FUNCTIONS"
        );
    }

    #[test]
    fn available_functions_merges_ambient_context() {
        let mut host = LspHost::new();
        host.set_session_context(DatabaseCatalog {
            relations: vec![],
            functions: vec![FunctionDef {
                name: "my_custom_func".to_string(),
                args: Some(2),
            }],
        });
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
        assert!(expected.contains(&(TokenType::JOIN as u32)));
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
    fn syntax_error_offset_points_at_error_token_not_following_token() {
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
    fn syntax_error_offset_via_parser_directly() {
        let sql = "select 1 from slice where foo = where x = y;";
        let parser = Parser::new(crate::dialect::sqlite());
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
