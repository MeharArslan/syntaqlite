// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use crate::dialect::TokenCategory;
use crate::fmt::{FormatConfig, Formatter};
use crate::parser::{
    LowLevelParser, ParserConfig, TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID, TOKEN_FLAG_AS_TYPE,
    Tokenizer,
};
use crate::{Dialect, ParseError, Parser};

use crate::lsp::{Diagnostic, SemanticToken, Severity};
use crate::validation::types::{FunctionDef, SessionContext};

/// Semantic completion context derived from parser stack state.
#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionContext {
    /// Could not determine context.
    Unknown = 0,
    /// Cursor is in an expression position (functions/values expected).
    Expression = 1,
    /// Cursor is in a table-reference position (table/view names expected).
    TableRef = 2,
}

impl CompletionContext {
    fn from_raw(v: u32) -> Self {
        match v {
            1 => Self::Expression,
            2 => Self::TableRef,
            _ => Self::Unknown,
        }
    }
}

/// Expected tokens and semantic context at a cursor position.
#[derive(Debug)]
pub struct CompletionInfo {
    /// Terminal token IDs valid at the cursor.
    pub tokens: Vec<u32>,
    /// Semantic context (expression vs table-ref).
    pub context: CompletionContext,
}

/// Manages open documents and runs analysis queries.
pub struct AnalysisHost<'d> {
    dialect: Dialect<'d>,
    documents: HashMap<String, Document>,
    context: Option<SessionContext>,
    dialect_config: Option<crate::dialect::ffi::DialectConfig>,
}

struct Document {
    version: i32,
    source: String,
    state: Option<DocumentState>,
}

struct DocumentState {
    diagnostics: Vec<Diagnostic>,
    semantic_tokens: Vec<SemanticToken>,
    tokens: Vec<CachedToken>,
}

struct CachedToken {
    type_: u32,
    start: usize,
    end: usize,
}

impl<'d> AnalysisHost<'d> {
    pub fn with_dialect(dialect: Dialect<'d>) -> Self {
        AnalysisHost {
            dialect,
            documents: HashMap::new(),
            context: None,
            dialect_config: None,
        }
    }

    /// Create an analysis host for the built-in SQLite dialect.
    #[cfg(feature = "sqlite")]
    pub fn new() -> AnalysisHost<'static> {
        AnalysisHost::with_dialect(*crate::sqlite::DIALECT)
    }

    /// Set the session context (user-provided schema and functions).
    pub fn set_session_context(&mut self, ctx: SessionContext) {
        self.context = Some(ctx);
    }

    /// Access the current session context.
    pub fn session_context(&self) -> Option<&SessionContext> {
        self.context.as_ref()
    }

    /// Run semantic validation on a document, generic over the dialect's AST types.
    ///
    /// Parses the document, walks each statement through
    /// [`validate_statement_dialect`](crate::validation::validate_statement_dialect),
    /// and returns diagnostics for unresolved table names, column references,
    /// and function calls.
    pub fn validate_dialect<A: for<'a> crate::ast_traits::AstTypes<'a>>(
        &self,
        uri: &str,
        config: &crate::validation::ValidationConfig,
    ) -> Vec<Diagnostic> {
        let Some(doc) = self.documents.get(uri) else {
            return Vec::new();
        };

        let functions = self.available_functions();
        let mut parser = crate::Parser::with_dialect(&self.dialect);
        let mut cursor = parser.parse(&doc.source);

        // Collect all statement IDs first, stopping at the first parse error.
        let stmt_ids: Vec<_> = (&mut cursor).map_while(|r| r.ok()).collect();

        // Single-pass incremental validation: each statement is validated
        // against only the DDL that precedes it in the document, then its
        // own definitions are accumulated for subsequent statements.
        let mut doc_ctx = crate::validation::DocumentContext::new();
        let reader = cursor.reader();
        let mut diagnostics = Vec::new();

        for &stmt_id in &stmt_ids {
            // Validate BEFORE accumulating — statement cannot see its own definitions.
            let stmt_diags = crate::validation::validate_statement_dialect::<A>(
                reader,
                stmt_id,
                self.dialect,
                self.context.as_ref(),
                Some(&doc_ctx),
                &functions,
                config,
            );
            diagnostics.extend(stmt_diags);

            // Now add any DDL this statement defined to the document schema.
            #[cfg(feature = "sqlite")]
            doc_ctx.accumulate(reader, stmt_id, self.context.as_ref());
        }

        diagnostics
    }

    /// Run semantic validation using the built-in SQLite dialect.
    ///
    /// Convenience wrapper around [`validate_dialect`](Self::validate_dialect).
    #[cfg(feature = "sqlite")]
    pub fn validate(
        &self,
        uri: &str,
        config: &crate::validation::ValidationConfig,
    ) -> Vec<Diagnostic> {
        self.validate_dialect::<crate::sqlite::ast::SqliteAst>(uri, config)
    }

    /// Set the dialect configuration for filtering built-in functions.
    pub fn set_dialect_config(&mut self, config: crate::dialect::ffi::DialectConfig) {
        self.dialect_config = Some(config);
    }

    /// Returns function definitions available in the current configuration.
    ///
    /// Three-layer resolution:
    /// 1. SQLite base catalog (filtered by `DialectConfig`)
    /// 2. Dialect extensions from the C vtable (filtered by `DialectConfig`)
    /// 3. Session context user functions
    pub fn available_functions(&self) -> Vec<FunctionDef> {
        let default_config = crate::dialect::ffi::DialectConfig::default();
        let config = self.dialect_config.as_ref().unwrap_or(&default_config);

        // Layer 1: SQLite base catalog (filtered by config)
        #[cfg(feature = "sqlite")]
        let mut result = catalog_to_function_defs(config);
        #[cfg(not(feature = "sqlite"))]
        let mut result = Vec::new();

        // Layer 2: Dialect extensions (filtered by config)
        for ext in self.dialect.function_extensions() {
            if crate::catalog::is_available(&ext, config) {
                result.extend(expand_function_info(&ext.info));
            }
        }

        // Layer 3: Session context user functions
        if let Some(ref ctx) = self.context {
            result.extend(ctx.functions.iter().cloned());
        }

        result
    }

    /// Register a newly opened document.
    pub fn open_document(&mut self, uri: &str, version: i32, text: String) {
        self.documents.insert(
            uri.to_string(),
            Document {
                version,
                source: text,
                state: None,
            },
        );
    }

    /// Update a document's content, invalidating cached state.
    pub fn update_document(&mut self, uri: &str, version: i32, text: String) {
        if let Some(doc) = self.documents.get_mut(uri) {
            if doc.version == version && doc.source == text {
                return;
            }
            doc.version = version;
            doc.source = text;
            doc.state = None;
        } else {
            self.open_document(uri, version, text);
        }
    }

    /// Remove a document from the host.
    pub fn close_document(&mut self, uri: &str) {
        self.documents.remove(uri);
    }

    /// Get diagnostics for a document, lazily parsing if needed.
    pub fn diagnostics(&mut self, uri: &str) -> &[Diagnostic] {
        if let Some(doc) = self.documents.get_mut(uri) {
            ensure_document_state(&self.dialect, doc);
            &doc.state.as_ref().unwrap().diagnostics
        } else {
            &[]
        }
    }

    /// Borrow document source + diagnostics + version in one host borrow.
    pub fn document_diagnostics(&mut self, uri: &str) -> Option<(i32, &str, &[Diagnostic])> {
        let doc = self.documents.get_mut(uri)?;
        ensure_document_state(&self.dialect, doc);
        let state = doc.state.as_ref().unwrap();
        Some((
            doc.version,
            doc.source.as_str(),
            state.diagnostics.as_slice(),
        ))
    }

    /// Get the source text for a document, if it exists.
    pub fn document_source(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|doc| doc.source.as_str())
    }

    /// Get semantic tokens for a document.
    ///
    /// Uses the parser with `collect_tokens` to resolve keyword/identifier
    /// fallback via grammar actions (tokens marked with `SYNQ_TOKEN_FLAG_AS_ID`
    /// are classified as `Identifier` regardless of their original token type).
    /// Function callee names marked with `SYNQ_TOKEN_FLAG_AS_FUNCTION` are
    /// classified as `Function`.
    /// Tokens marked with `SYNQ_TOKEN_FLAG_AS_TYPE` are classified as `Type`.
    pub fn semantic_tokens(&mut self, uri: &str) -> &[SemanticToken] {
        if let Some(doc) = self.documents.get_mut(uri) {
            ensure_document_state(&self.dialect, doc);
            &doc.state.as_ref().unwrap().semantic_tokens
        } else {
            &[]
        }
    }

    /// Get semantic tokens as a delta-encoded `Uint32Array`-compatible vector.
    ///
    /// Each token is 5 u32s: `[deltaLine, deltaStartChar, length, legendIndex, 0]`.
    /// This is the format Monaco/LSP expects, computed in a single O(n) pass
    /// over the source.
    ///
    /// When `range` is `Some((start_offset, end_offset))`, only tokens whose
    /// offset falls within the byte range are emitted (the full document is
    /// still parsed for correct fallback resolution).
    pub fn semantic_tokens_encoded(
        &mut self,
        uri: &str,
        range: Option<(usize, usize)>,
    ) -> Vec<u32> {
        let doc = match self.documents.get_mut(uri) {
            Some(d) => d,
            None => return Vec::new(),
        };
        ensure_document_state(&self.dialect, doc);
        let source = doc.source.as_bytes();
        let tokens = &doc.state.as_ref().unwrap().semantic_tokens;

        let (range_start, range_end) = range.unwrap_or((0, source.len()));

        let mut result = Vec::with_capacity(tokens.len() * 5);
        let mut prev_line: u32 = 0;
        let mut prev_col: u32 = 0;
        // Walk source bytes in lockstep with tokens (both sorted by offset).
        let mut cur_line: u32 = 0;
        let mut cur_col: u32 = 0;
        let mut src_pos: usize = 0;

        for tok in tokens {
            // Advance src_pos to tok.offset, tracking line/col.
            while src_pos < tok.offset && src_pos < source.len() {
                if source[src_pos] == b'\n' {
                    cur_line += 1;
                    cur_col = 0;
                } else {
                    cur_col += 1;
                }
                src_pos += 1;
            }

            // Range filter: skip tokens before range, stop after range.
            if tok.offset < range_start {
                continue;
            }
            if tok.offset >= range_end {
                break;
            }

            let legend_idx = match tok.category.legend_index() {
                Some(idx) => idx,
                None => continue, // Skip Other tokens
            };

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
            result.push(0); // modifiers bitset

            prev_line = cur_line;
            prev_col = cur_col;
        }

        result
    }

    /// Format a document's source text.
    pub fn format(&self, uri: &str, config: &FormatConfig) -> Result<String, FormatError> {
        let doc = self
            .documents
            .get(uri)
            .ok_or(FormatError::UnknownDocument)?;
        let mut formatter = Formatter::with_dialect_config(&self.dialect, config.clone())
            .map_err(FormatError::Setup)?;
        formatter.format(&doc.source).map_err(FormatError::Parse)
    }

    /// Return parser-expected terminal token IDs at a byte offset.
    ///
    /// Replays tokens up to the cursor on demand — O(k) where k is the
    /// number of tokens before the cursor. This avoids precomputing expected
    /// sets for every token boundary.
    pub fn expected_tokens_at_offset(&mut self, uri: &str, offset: usize) -> Vec<u32> {
        self.completion_info_at_offset(uri, offset).tokens
    }

    /// Return expected tokens and semantic completion context at a byte offset.
    pub fn completion_info_at_offset(&mut self, uri: &str, offset: usize) -> CompletionInfo {
        let Some(doc) = self.documents.get_mut(uri) else {
            return CompletionInfo {
                tokens: Vec::new(),
                context: CompletionContext::Unknown,
            };
        };
        ensure_document_state(&self.dialect, doc);
        let state = doc.state.as_ref().unwrap();
        replay_completion_info(&self.dialect, &doc.source, &state.tokens, offset)
    }
}

/// Expand a `FunctionInfo` into one `FunctionDef` per arity.
fn expand_function_info(info: &crate::catalog::FunctionInfo<'_>) -> Vec<FunctionDef> {
    if info.arities.is_empty() {
        vec![FunctionDef {
            name: info.name.to_string(),
            args: None,
            description: None,
        }]
    } else {
        info.arities
            .iter()
            .map(|&arity| FunctionDef {
                name: info.name.to_string(),
                args: if arity < 0 {
                    None
                } else {
                    Some(arity as usize)
                },
                description: None,
            })
            .collect()
    }
}

/// Convert the SQLite function catalog into `FunctionDef` values filtered by config.
#[cfg(feature = "sqlite")]
fn catalog_to_function_defs(config: &crate::dialect::ffi::DialectConfig) -> Vec<FunctionDef> {
    crate::sqlite::functions::available_functions(config)
        .into_iter()
        .flat_map(|info| expand_function_info(info))
        .collect()
}

fn compute_document_state(dialect: &Dialect, source: &str) -> DocumentState {
    let config = ParserConfig {
        collect_tokens: true,
        ..Default::default()
    };
    let mut parser = Parser::with_dialect_config(dialect, &config);
    let mut cursor = parser.parse(source);
    let mut diagnostics = Vec::new();

    while let Some(result) = cursor.next_statement() {
        if let Err(err) = result {
            let (start_offset, end_offset) = error_span(&err, source);
            diagnostics.push(Diagnostic {
                start_offset,
                end_offset,
                message: err.message,
                severity: Severity::Error,
            });
            break;
        }
    }

    let mut semantic_tokens = Vec::new();

    for tp in cursor.base().tokens() {
        let cat = if tp.flags & TOKEN_FLAG_AS_FUNCTION != 0 {
            TokenCategory::Function
        } else if tp.flags & TOKEN_FLAG_AS_TYPE != 0 {
            TokenCategory::Type
        } else if tp.flags & TOKEN_FLAG_AS_ID != 0 {
            TokenCategory::Identifier
        } else {
            dialect.token_category(tp.type_)
        };
        if cat == TokenCategory::Other {
            continue;
        }
        semantic_tokens.push(SemanticToken {
            offset: tp.offset as usize,
            length: tp.length as usize,
            category: cat,
        });
    }

    for c in cursor.base().comments() {
        semantic_tokens.push(SemanticToken {
            offset: c.offset as usize,
            length: c.length as usize,
            category: TokenCategory::Comment,
        });
    }
    semantic_tokens.sort_by_key(|t| t.offset);

    let mut tokens = Vec::new();
    let mut tokenizer = Tokenizer::with_dialect(*dialect);
    let source_base = source.as_ptr() as usize;
    for tok in tokenizer.tokenize(source) {
        let start = tok.text.as_ptr() as usize - source_base;
        let end = start + tok.text.len();

        tokens.push(CachedToken {
            type_: tok.token_type,
            start,
            end,
        });
    }

    DocumentState {
        diagnostics,
        semantic_tokens,
        tokens,
    }
}

fn ensure_document_state(dialect: &Dialect, doc: &mut Document) {
    if doc.state.is_none() {
        doc.state = Some(compute_document_state(dialect, &doc.source));
    }
}

fn replay_completion_info(
    dialect: &Dialect,
    source: &str,
    tokens: &[CachedToken],
    offset: usize,
) -> CompletionInfo {
    let cursor_offset = offset.min(source.len());
    let mut boundary = tokens.partition_point(|t| t.end <= cursor_offset);
    // Skip zero-width tokens at cursor, then backtrack if cursor is mid-identifier.
    while boundary > 0 && {
        let t = &tokens[boundary - 1];
        t.start == t.end && t.end == cursor_offset
    } {
        boundary -= 1;
    }
    let mut backtracked = false;
    if boundary > 0 && tokens[boundary - 1].end == cursor_offset && cursor_offset > 0 {
        let b = source.as_bytes()[cursor_offset - 1];
        if b.is_ascii_alphanumeric() || b == b'_' {
            boundary -= 1;
            backtracked = true;
        }
    }
    let tk_semi = dialect.tk_semi();
    let start = tokens[..boundary]
        .iter()
        .rposition(|t| t.type_ == tk_semi)
        .map_or(0, |idx| idx + 1);

    let stmt_tokens = &tokens[start..boundary];

    let mut parser = LowLevelParser::with_dialect(dialect);
    let mut cursor = parser.feed(source);
    let mut last_expected = cursor.expected_tokens();

    for tok in stmt_tokens {
        if cursor.feed_token(tok.type_, tok.start..tok.end).is_err() {
            let ctx = CompletionContext::from_raw(cursor.completion_context());
            return CompletionInfo {
                tokens: last_expected,
                context: ctx,
            };
        }
        last_expected = cursor.expected_tokens();
    }

    let context = CompletionContext::from_raw(cursor.completion_context());

    // When the cursor is at the end of an identifier token, we backtracked past it
    // to offer identifier completions. Also feed it and merge the expected tokens
    // that follow, so keywords like JOIN are suggested too.
    if backtracked {
        let extra_tok = &tokens[boundary];
        if cursor
            .feed_token(extra_tok.type_, extra_tok.start..extra_tok.end)
            .is_ok()
        {
            let after = cursor.expected_tokens();
            let mut seen: std::collections::HashSet<u32> = last_expected.iter().copied().collect();
            for tok in after {
                if seen.insert(tok) {
                    last_expected.push(tok);
                }
            }
        }
    }

    CompletionInfo {
        tokens: last_expected,
        context,
    }
}

fn error_span(err: &ParseError, source: &str) -> (usize, usize) {
    match (err.offset, err.length) {
        (Some(offset), Some(length)) if length > 0 => (offset, offset + length),
        (Some(offset), _) => {
            // Point at the error offset; if at end of input, highlight last char.
            if offset >= source.len() && !source.is_empty() {
                (source.len() - 1, source.len())
            } else {
                (offset, (offset + 1).min(source.len()))
            }
        }
        _ => {
            // No offset info — highlight end of source.
            let end = source.len();
            let start = if end > 0 { end - 1 } else { 0 };
            (start, end)
        }
    }
}

/// Errors that can occur during formatting.
#[derive(Debug)]
pub enum FormatError {
    /// The document URI was not found.
    UnknownDocument,
    /// Formatter setup failed (e.g., dialect has no fmt data).
    Setup(&'static str),
    /// Parse error during formatting.
    Parse(ParseError),
}

impl std::fmt::Display for FormatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatError::UnknownDocument => write!(f, "unknown document"),
            FormatError::Setup(msg) => write!(f, "formatter setup: {msg}"),
            FormatError::Parse(err) => write!(f, "parse error: {err}"),
        }
    }
}

impl std::error::Error for FormatError {}

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use super::AnalysisHost;
    use crate::lsp::FunctionDef;
    use crate::sqlite::low_level::TokenType;
    use crate::validation::SessionContext;

    #[test]
    fn completions_fall_back_to_last_good_state_on_parse_error() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FR";
        host.open_document(uri, 1, sql.to_string());

        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::From as u32)),
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
            expected.contains(&(TokenType::From as u32)),
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
            expected.contains(&(TokenType::JoinKw as u32)),
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
            expected.contains(&(TokenType::Join as u32)),
            "expected TK_JOIN after FROM table with trailing space, got {:?}",
            expected
        );
        assert!(
            !expected.contains(&(TokenType::Create as u32)),
            "TK_CREATE should not appear after FROM table, got {:?}",
            expected
        );
        assert!(
            !expected.contains(&(TokenType::Select as u32)),
            "TK_SELECT should not appear after FROM table, got {:?}",
            expected
        );
        assert!(
            !expected.contains(&(TokenType::Virtual as u32)),
            "TK_VIRTUAL (fallback to ID) should not appear after FROM table, got {:?}",
            expected
        );
    }

    #[test]
    fn available_functions_default_config_includes_baseline() {
        let host = AnalysisHost::new();
        let funcs = host.available_functions();
        let names: Vec<&str> = funcs.iter().map(|f| f.name.as_str()).collect();
        assert!(names.contains(&"abs"), "abs should be in default config");
        assert!(
            names.contains(&"count"),
            "count should be in default config"
        );
        assert!(
            !names.contains(&"acos"),
            "acos requires ENABLE_MATH_FUNCTIONS"
        );
    }

    #[test]
    fn available_functions_with_config_filters_by_cflags() {
        let mut host = AnalysisHost::new();
        let mut config = crate::dialect::ffi::DialectConfig::default();
        // SQLITE_ENABLE_MATH_FUNCTIONS = cflag index 34
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
        assert!(
            names.contains(&"my_custom_func"),
            "user-provided function should be in results"
        );
        assert!(
            names.contains(&"abs"),
            "built-in abs should still be present"
        );
    }

    #[test]
    fn completion_context_after_from_is_table_ref() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT acos() as foo FROM ";
        host.open_document(uri, 1, sql.to_string());

        let info = host.completion_info_at_offset(uri, sql.len());
        assert_eq!(
            info.context,
            super::CompletionContext::TableRef,
            "context after FROM should be TableRef, got {:?}",
            info.context
        );
    }

    #[test]
    fn completion_context_after_select_is_not_table_ref() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT ";
        host.open_document(uri, 1, sql.to_string());

        let info = host.completion_info_at_offset(uri, sql.len());
        // After bare SELECT, the parser hasn't reached an expr goto state yet,
        // so context is Unknown — but importantly it is NOT TableRef, so
        // functions are still shown (Unknown and Expression both allow functions).
        assert_ne!(
            info.context,
            super::CompletionContext::TableRef,
            "context after SELECT should not be TableRef, got {:?}",
            info.context
        );
    }

    #[test]
    fn completion_context_after_where_is_expression() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM t WHERE ";
        host.open_document(uri, 1, sql.to_string());

        let info = host.completion_info_at_offset(uri, sql.len());
        assert_eq!(
            info.context,
            super::CompletionContext::Expression,
            "context after WHERE should be Expression, got {:?}",
            info.context
        );
    }

    #[test]
    fn completions_include_join_after_from_table_no_trailing_space() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM slice";
        host.open_document(uri, 1, sql.to_string());

        let expected = host.expected_tokens_at_offset(uri, sql.len());
        assert!(
            expected.contains(&(TokenType::Join as u32)),
            "expected TK_JOIN after FROM table without trailing space, got {:?}",
            expected
        );
    }

    #[test]
    fn validate_select_after_create_table_as_select_no_diags() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "CREATE TABLE orders AS SELECT 1 AS order_id;\nSELECT o.order_id FROM orders o;";
        host.open_document(uri, 1, sql.to_string());

        let diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        assert!(
            diags.is_empty(),
            "expected no diagnostics when selecting from a table defined earlier in the document: {:?}",
            diags
        );
    }

    #[test]
    fn validate_select_from_unknown_table_still_warns() {
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM nonexistent;";
        host.open_document(uri, 1, sql.to_string());

        let diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        assert!(
            !diags.is_empty(),
            "expected a diagnostic for an unknown table"
        );
    }

    #[test]
    fn validate_forward_reference_warns() {
        // A SELECT that references a table defined *after* it should produce a warning.
        let mut host = AnalysisHost::new();
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM t;\nCREATE TABLE t (id INTEGER);";
        host.open_document(uri, 1, sql.to_string());

        let diags = host.validate(uri, &crate::validation::ValidationConfig::default());
        assert!(
            !diags.is_empty(),
            "expected a diagnostic for forward reference to table t: {:?}",
            diags
        );
    }
}
