// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use syntaqlite_runtime::dialect::TokenCategory;
use syntaqlite_runtime::fmt::{FormatConfig, Formatter};
use syntaqlite_runtime::parser::{
    LowLevelParser, ParserConfig, TOKEN_FLAG_AS_FUNCTION, TOKEN_FLAG_AS_ID, TOKEN_FLAG_AS_TYPE,
    Tokenizer,
};
use syntaqlite_runtime::{Dialect, ParseError, Parser};

use crate::context::AmbientContext;
use crate::types::{Diagnostic, SemanticToken, Severity};

/// Manages open documents and runs analysis queries.
pub struct AnalysisHost<'d> {
    dialect: Dialect<'d>,
    documents: HashMap<String, Document>,
    context: Option<AmbientContext>,
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
    pub fn new(dialect: Dialect<'d>) -> Self {
        AnalysisHost {
            dialect,
            documents: HashMap::new(),
            context: None,
        }
    }

    /// Set the ambient database schema context.
    pub fn set_ambient_context(&mut self, ctx: AmbientContext) {
        self.context = Some(ctx);
    }

    /// Access the current ambient context.
    pub fn ambient_context(&self) -> Option<&AmbientContext> {
        self.context.as_ref()
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
    pub fn semantic_tokens(&mut self, uri: &str) -> Vec<SemanticToken> {
        let doc = match self.documents.get_mut(uri) {
            Some(d) => d,
            None => return Vec::new(),
        };
        ensure_document_state(&self.dialect, doc);
        doc.state
            .as_ref()
            .unwrap()
            .semantic_tokens
            .as_slice()
            .to_vec()
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
        let mut formatter =
            Formatter::with_config(&self.dialect, config.clone()).map_err(FormatError::Setup)?;
        formatter.format(&doc.source).map_err(FormatError::Parse)
    }

    /// Return parser-expected terminal token IDs at a byte offset.
    ///
    /// Replays tokens up to the cursor on demand — O(k) where k is the
    /// number of tokens before the cursor. This avoids precomputing expected
    /// sets for every token boundary.
    pub fn expected_tokens_at_offset(&mut self, uri: &str, offset: usize) -> Vec<u32> {
        let Some(doc) = self.documents.get_mut(uri) else {
            return Vec::new();
        };
        ensure_document_state(&self.dialect, doc);
        let state = doc.state.as_ref().unwrap();
        replay_expected_tokens(&self.dialect, &doc.source, &state.tokens, offset)
    }
}

fn compute_document_state(dialect: &Dialect, source: &str) -> DocumentState {
    let config = ParserConfig {
        collect_tokens: true,
        ..Default::default()
    };
    let mut parser = Parser::with_config(dialect, &config);
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
    let mut tokenizer = Tokenizer::new(*dialect);
    let mut consumed = 0usize;
    for tok in tokenizer.tokenize(source) {
        let start = consumed;
        let end = consumed + tok.text.len();
        consumed = end;

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

fn replay_expected_tokens(
    dialect: &Dialect,
    source: &str,
    tokens: &[CachedToken],
    offset: usize,
) -> Vec<u32> {
    let cursor_offset = offset.min(source.len());
    let boundary = tokens.partition_point(|t| t.end <= cursor_offset);
    let mut parser = LowLevelParser::new(dialect);
    let mut cursor = parser.feed(source);

    for tok in &tokens[..boundary] {
        if cursor.feed_token(tok.type_, tok.start..tok.end).is_err() {
            return Vec::new();
        }
    }

    cursor.expected_tokens()
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
