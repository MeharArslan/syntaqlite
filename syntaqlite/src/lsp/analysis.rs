// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Per-document analysis: parse, diagnostics, semantic tokens, completions.

use crate::dialect::{Dialect, TokenCategory};
use crate::lsp::{CompletionContext, CompletionInfo, SemanticToken};
use crate::parser::incremental::RawIncrementalParser;
use crate::parser::session::RawParser;
use crate::parser::tokenizer::RawTokenizer;
use crate::validation::types::{Diagnostic, DiagnosticMessage, Severity};

/// A raw token position cached for completion replay.
pub(crate) struct CachedToken {
    pub(crate) type_: u32,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

/// The result of analyzing a single document: parse diagnostics,
/// semantic tokens for highlighting, and raw token positions for
/// completion replay.
///
/// Computed once per document edit via [`DocumentAnalysis::compute`] and
/// cached until the document changes.
pub struct DocumentAnalysis {
    diagnostics: Vec<Diagnostic>,
    semantic_tokens: Vec<SemanticToken>,
    /// Raw token boundaries for incremental completion replay.
    tokens: Vec<CachedToken>,
}

impl DocumentAnalysis {
    /// Parse `source` against `dialect` and collect all analysis results.
    pub fn compute(dialect: Dialect<'_>, source: &str) -> Self {
        let mut parser = RawParser::builder(dialect).collect_tokens(true).build();
        let mut cursor = parser.parse(source);
        let mut diagnostics = Vec::new();

        while let Some(result) = cursor.next_statement() {
            if let Err(err) = result.map(|nr| nr.id()) {
                let (start_offset, end_offset) = crate::validation::parse_error_span(&err, source);
                diagnostics.push(Diagnostic {
                    start_offset,
                    end_offset,
                    message: DiagnosticMessage::Other(err.message),
                    severity: Severity::Error,
                    help: None,
                });
            }
        }

        let mut semantic_tokens = Vec::new();

        for tp in cursor.state().tokens() {
            let cat = TokenCategory::from_u8(dialect.classify_token_raw(tp.type_, tp.flags));
            if cat == TokenCategory::Other {
                continue;
            }
            semantic_tokens.push(SemanticToken {
                offset: tp.offset as usize,
                length: tp.length as usize,
                category: cat,
            });
        }

        for c in cursor.state().comments() {
            semantic_tokens.push(SemanticToken {
                offset: c.offset as usize,
                length: c.length as usize,
                category: TokenCategory::Comment,
            });
        }
        semantic_tokens.sort_by_key(|t| t.offset);

        let mut tokens = Vec::new();
        let mut tokenizer = RawTokenizer::builder(dialect).build();
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

        DocumentAnalysis {
            diagnostics,
            semantic_tokens,
            tokens,
        }
    }

    /// Parse-error diagnostics for this document.
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Semantic tokens for syntax highlighting, sorted by byte offset.
    pub fn semantic_tokens(&self) -> &[SemanticToken] {
        &self.semantic_tokens
    }

    /// Semantic tokens delta-encoded as a flat `u32` array (5 values per token:
    /// `deltaLine`, `deltaStartChar`, `length`, `legendIndex`, `modifiers`).
    ///
    /// This is the format expected by Monaco/LSP `textDocument/semanticTokens/full`.
    /// When `range` is `Some((start, end))` only tokens within that byte range
    /// are emitted; the document is still fully parsed for correct fallback
    /// resolution.
    pub fn semantic_tokens_encoded(&self, source: &str, range: Option<(usize, usize)>) -> Vec<u32> {
        let src = source.as_bytes();
        let (range_start, range_end) = range.unwrap_or((0, src.len()));

        let mut result = Vec::with_capacity(self.semantic_tokens.len() * 5);
        let mut prev_line: u32 = 0;
        let mut prev_col: u32 = 0;
        let mut cur_line: u32 = 0;
        let mut cur_col: u32 = 0;
        let mut src_pos: usize = 0;

        for tok in &self.semantic_tokens {
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

    /// Expected parser tokens and semantic context at `offset`.
    ///
    /// Replays raw tokens up to the cursor position through an incremental
    /// parser to determine what terminal tokens are valid at that point.
    pub fn completion_info_at(
        &self,
        dialect: Dialect<'_>,
        source: &str,
        offset: usize,
    ) -> CompletionInfo {
        let cursor_offset = offset.min(source.len());
        let mut boundary = self.tokens.partition_point(|t| t.end <= cursor_offset);

        // Skip zero-width tokens at cursor.
        while boundary > 0 && {
            let t = &self.tokens[boundary - 1];
            t.start == t.end && t.end == cursor_offset
        } {
            boundary -= 1;
        }

        // Backtrack if cursor is mid-identifier so we still suggest completions.
        let mut backtracked = false;
        if boundary > 0 && self.tokens[boundary - 1].end == cursor_offset && cursor_offset > 0 {
            let b = source.as_bytes()[cursor_offset - 1];
            if b.is_ascii_alphanumeric() || b == b'_' {
                boundary -= 1;
                backtracked = true;
            }
        }

        let tk_semi = dialect.tk_semi();
        let start = self.tokens[..boundary]
            .iter()
            .rposition(|t| t.type_ == tk_semi)
            .map_or(0, |idx| idx + 1);

        let stmt_tokens = &self.tokens[start..boundary];

        let mut parser = RawIncrementalParser::builder(dialect).build();
        let mut cursor = parser.feed(source);
        let mut last_expected = cursor.expected_tokens();

        for tok in stmt_tokens {
            if cursor.feed_token(tok.type_, tok.start..tok.end).is_err() {
                return CompletionInfo {
                    tokens: last_expected,
                    context: CompletionContext::from_raw(cursor.completion_context()),
                };
            }
            last_expected = cursor.expected_tokens();
        }

        let context = CompletionContext::from_raw(cursor.completion_context());

        if backtracked {
            let extra = &self.tokens[boundary];
            if cursor
                .feed_token(extra.type_, extra.start..extra.end)
                .is_ok()
            {
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

        CompletionInfo {
            tokens: last_expected,
            context,
        }
    }
}
