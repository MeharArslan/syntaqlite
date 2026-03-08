// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::Cell;

use syntaqlite_syntax::CommentKind;

use super::doc::{DocArena, DocId, NIL_DOC};

/// A collected comment entry with pre-computed byte offset and length.
#[derive(Clone, Copy)]
pub(crate) struct CommentEntry {
    pub offset: u32,
    pub length: u32,
    pub kind: CommentKind,
}

/// A collected token entry with pre-computed byte offset and length.
#[derive(Clone, Copy)]
pub(crate) struct TokenEntry {
    pub offset: u32,
    pub length: u32,
}

/// Result of draining comment items. Trailing docs (e.g. `LineSuffix` for
/// end-of-line comments) go BEFORE any pending line break. Leading docs
/// (comments on their own line) go AFTER any pending line break.
pub(crate) struct DrainResult {
    pub trailing: DocId,
    pub leading: DocId,
}

/// Two cursors advancing monotonically through sorted comment and token arrays.
/// Shared via `&` across iterative formatting traversal; interior mutability is
/// required because interpreter state carries a shared `&CommentCtx`.
///
/// Owns its comment and token data (no lifetime parameter).
pub(crate) struct CommentCtx {
    comments: Vec<CommentEntry>,
    tokens: Vec<TokenEntry>,
    cursor: Cell<usize>,
    token_cursor: Cell<usize>,
}

impl CommentCtx {
    pub(crate) fn new(comments: Vec<CommentEntry>, tokens: Vec<TokenEntry>) -> Self {
        CommentCtx {
            comments,
            tokens,
            cursor: Cell::new(0),
            token_cursor: Cell::new(0),
        }
    }

    /// Return owned storage so callers can recycle vector allocations.
    pub(crate) fn into_parts(self) -> (Vec<CommentEntry>, Vec<TokenEntry>) {
        (self.comments, self.tokens)
    }

    /// End offset of the token just before the current token cursor position.
    /// Returns 0 if the cursor is at the start.
    pub(crate) fn prev_token_end(&self) -> u32 {
        let idx = self.token_cursor.get();
        if idx > 0 {
            let tp = &self.tokens[idx - 1];
            tp.offset + tp.length
        } else {
            0
        }
    }

    /// Drain all comments with offset < `before`.
    ///
    /// Stops early if there is non-whitespace source text (i.e. a keyword)
    /// between a comment and `before`.
    pub(crate) fn drain_before<'a>(
        &self,
        before: u32,
        source: &'a str,
        arena: &mut DocArena<'a>,
    ) -> DrainResult {
        self.drain_impl(before, source, arena, false)
    }

    /// Drain comments between the tokens of a multi-word keyword (e.g.,
    /// between LEFT and JOIN in "LEFT JOIN"). Unlike `drain_before`, this
    /// skips the `has_non_comment_text` guard because the intervening tokens
    /// belong to the same keyword being output.
    pub(crate) fn drain_keyword_interior<'a>(
        &self,
        word_count: usize,
        source: &'a str,
        arena: &mut DocArena<'a>,
    ) -> DrainResult {
        let first_idx = self.token_cursor.get();
        let last_tok = &self.tokens[first_idx + word_count - 1];
        let end = last_tok.offset + last_tok.length;
        self.drain_impl(end, source, arena, true)
    }

    fn drain_impl<'a>(
        &self,
        before: u32,
        source: &'a str,
        arena: &mut DocArena<'a>,
        skip_text_check: bool,
    ) -> DrainResult {
        let mut trailing = NIL_DOC;
        let mut leading = NIL_DOC;
        let mut cursor = self.cursor.get();
        let mut last_end = self.prev_token_end();
        while cursor < self.comments.len() && self.comments[cursor].offset < before {
            let t = &self.comments[cursor];
            let comment_end = (t.offset + t.length) as usize;

            if !skip_text_check {
                let before_usize = (before as usize).min(source.len());
                if comment_end < before_usize
                    && has_non_comment_text(
                        source,
                        comment_end,
                        before_usize,
                        &self.comments,
                        cursor + 1,
                    )
                {
                    break;
                }
            }

            let text = &source[t.offset as usize..comment_end];

            let gap_start = (last_end as usize).min(source.len());
            let gap_end = (t.offset as usize).min(source.len());
            let has_newline = gap_start < gap_end && source[gap_start..gap_end].contains('\n');

            match t.kind {
                CommentKind::Line => {
                    if has_newline {
                        let hl1 = arena.hardline();
                        let comment_doc = arena.text(text);
                        let hl2 = arena.hardline();
                        let inner = arena.cat(comment_doc, hl2);
                        let chunk = arena.cat(hl1, inner);
                        leading = if leading == NIL_DOC {
                            chunk
                        } else {
                            arena.cat(leading, chunk)
                        };
                    } else {
                        let space = arena.text(" ");
                        let comment = arena.text(text);
                        let inner = arena.cat(space, comment);
                        let ls = arena.line_suffix(inner);
                        let bp = arena.break_parent();
                        let chunk = arena.cat(ls, bp);
                        trailing = if trailing == NIL_DOC {
                            chunk
                        } else {
                            arena.cat(trailing, chunk)
                        };
                    }
                }
                CommentKind::Block => {
                    if has_newline {
                        let hl = arena.hardline();
                        let comment_doc = arena.text(text);
                        let chunk = arena.cat(hl, comment_doc);
                        leading = if leading == NIL_DOC {
                            chunk
                        } else {
                            arena.cat(leading, chunk)
                        };
                    } else {
                        let sp = arena.text(" ");
                        let comment_doc = arena.text(text);
                        let chunk = arena.cat(sp, comment_doc);
                        trailing = if trailing == NIL_DOC {
                            chunk
                        } else {
                            arena.cat(trailing, chunk)
                        };
                    }
                }
            }

            last_end = t.offset + t.length;
            cursor += 1;
        }

        self.cursor.set(cursor);

        DrainResult { trailing, leading }
    }

    /// Peek at the next N tokens without advancing the token cursor.
    /// Verifies each token's text matches the corresponding keyword word
    /// (case-insensitive). Returns `None` if the keyword is not present in
    /// the source (e.g., an inserted `AS`).
    pub(crate) fn peek_keyword_tokens(&self, kw_text: &str, source: &str) -> Option<(u32, usize)> {
        let first_idx = self.token_cursor.get();
        let mut word_count = 0usize;
        for word in kw_text.split_whitespace() {
            let tok_idx = first_idx + word_count;
            if tok_idx >= self.tokens.len() {
                return None;
            }
            let tok = &self.tokens[tok_idx];
            let tok_text = &source[tok.offset as usize..(tok.offset + tok.length) as usize];
            if !tok_text.eq_ignore_ascii_case(word) {
                return None;
            }
            word_count += 1;
        }
        if word_count == 0 {
            return None;
        }
        let first_offset = self.tokens[first_idx].offset;
        Some((first_offset, word_count))
    }

    /// Advance the token cursor by `n` positions.
    pub(crate) fn advance_token_cursor(&self, n: usize) {
        self.token_cursor.set(self.token_cursor.get() + n);
    }

    /// Advance the token cursor past all tokens whose offset is `< end_offset`.
    pub(crate) fn advance_past(&self, end_offset: u32) {
        let mut idx = self.token_cursor.get();
        while idx < self.tokens.len() && self.tokens[idx].offset < end_offset {
            idx += 1;
        }
        self.token_cursor.set(idx);
    }

    /// Peek at the next undrained comment without advancing the cursor.
    pub(crate) fn peek_comment(&self) -> Option<&CommentEntry> {
        let idx = self.cursor.get();
        self.comments.get(idx)
    }

    /// Advance the comment cursor by one.
    pub(crate) fn advance_comment(&self) {
        let idx = self.cursor.get();
        if idx < self.comments.len() {
            self.cursor.set(idx + 1);
        }
    }

    /// Peek at the next token's offset and length without advancing.
    pub(crate) fn peek_next_token(&self) -> Option<(u32, u32)> {
        let idx = self.token_cursor.get();
        self.tokens.get(idx).map(|tp| (tp.offset, tp.length))
    }

    /// Flush all remaining comments.
    pub(crate) fn drain_remaining<'a>(&self, source: &'a str, arena: &mut DocArena<'a>) -> DocId {
        let drain = self.drain_before(u32::MAX, source, arena);
        arena.cat(drain.trailing, drain.leading)
    }
}

fn has_non_comment_text(
    source: &str,
    start: usize,
    end: usize,
    comments: &[CommentEntry],
    comment_start_idx: usize,
) -> bool {
    let src = source.as_bytes();
    let mut pos = start;
    let mut ci = comment_start_idx;

    while pos < end {
        while ci < comments.len() {
            let c_start = comments[ci].offset as usize;
            let c_end = (comments[ci].offset + comments[ci].length) as usize;
            if pos >= c_start && pos < c_end {
                pos = c_end;
                ci += 1;
                break;
            } else if c_start > pos {
                break;
            }
            ci += 1;
        }
        if pos >= end {
            break;
        }
        if !src[pos].is_ascii_whitespace() {
            return true;
        }
        pos += 1;
    }
    false
}
