// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashMap;

use syntaqlite_runtime::dialect::TokenCategory;
use syntaqlite_runtime::fmt::{FormatConfig, Formatter};
use syntaqlite_runtime::parser::Tokenizer;
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
            if doc.state.is_none() {
                let diagnostics = compute_diagnostics(&self.dialect, &doc.source);
                doc.state = Some(DocumentState { diagnostics });
            }
            &doc.state.as_ref().unwrap().diagnostics
        } else {
            &[]
        }
    }

    /// Get the source text for a document, if it exists.
    pub fn document_source(&self, uri: &str) -> Option<&str> {
        self.documents.get(uri).map(|doc| doc.source.as_str())
    }

    /// Get semantic tokens for a document.
    pub fn semantic_tokens(&self, uri: &str) -> Vec<SemanticToken> {
        let doc = match self.documents.get(uri) {
            Some(d) => d,
            None => return Vec::new(),
        };
        let source = &doc.source;
        let mut tokenizer = Tokenizer::new(self.dialect);
        let tk_space = self.dialect.tk_space();
        let cursor = tokenizer.tokenize(source);
        let source_base = source.as_ptr() as usize;

        cursor
            .filter_map(|raw| {
                if raw.token_type == tk_space {
                    return None;
                }
                let cat = self.dialect.token_category(raw.token_type);
                if cat == TokenCategory::Other {
                    return None;
                }
                let offset = raw.text.as_ptr() as usize - source_base;
                Some(SemanticToken {
                    offset,
                    length: raw.text.len(),
                    category: cat,
                })
            })
            .collect()
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
}

fn compute_diagnostics(dialect: &Dialect, source: &str) -> Vec<Diagnostic> {
    let mut parser = match Parser::try_new(dialect) {
        Some(p) => p,
        None => {
            return vec![Diagnostic {
                start_offset: 0,
                end_offset: 0,
                message: "parser allocation failed".to_string(),
                severity: Severity::Error,
            }];
        }
    };
    let mut cursor = parser.parse(source);
    let mut diagnostics = Vec::new();

    while let Some(result) = cursor.next_statement() {
        match result {
            Ok(_) => {}
            Err(err) => {
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
    }

    diagnostics
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
