// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::Cell;

use crate::parser::{Comment, CommentKind, TokenPos};

use super::doc::{DocArena, DocId, NIL_DOC};

/// Result of draining comment items. Trailing docs (e.g. LineSuffix for
/// end-of-line comments) go BEFORE any pending line break. Leading docs
/// (comments on their own line) go AFTER any pending line break.
pub struct DrainResult {
    pub trailing: DocId,
    pub leading: DocId,
}

/// Two cursors advancing monotonically through sorted comment and token arrays.
/// Shared via `&` across recursive format calls; interior mutability is required
/// because the recursive `format_child` closure captures `&CommentCtx`.
pub struct CommentCtx<'a> {
    comments: &'a [Comment],
    tokens: &'a [TokenPos],
    cursor: Cell<usize>,
    token_cursor: Cell<usize>,
}

impl<'a> CommentCtx<'a> {
    pub fn new(comments: &'a [Comment], tokens: &'a [TokenPos]) -> Self {
        CommentCtx {
            comments,
            tokens,
            cursor: Cell::new(0),
            token_cursor: Cell::new(0),
        }
    }

    /// End offset of the token just before the current token cursor position.
    /// Returns 0 if the cursor is at the start.
    pub fn prev_token_end(&self) -> u32 {
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
    /// between a comment and `before`. This prevents comments that precede
    /// an intervening keyword from being attributed to the child after the
    /// keyword.
    pub fn drain_before(
        &self,
        before: u32,
        source: &'a str,
        arena: &mut DocArena<'a>,
    ) -> DrainResult {
        let mut trailing = Vec::new();
        let mut leading = Vec::new();
        let mut cursor = self.cursor.get();
        let mut last_end = self.prev_token_end();
        while cursor < self.comments.len() && self.comments[cursor].offset < before {
            let t = &self.comments[cursor];
            let comment_end = (t.offset + t.length) as usize;

            // Check if there is non-whitespace, non-comment source text
            // between the end of this comment and `before`. If so, there's
            // an intervening keyword — stop draining so this comment stays
            // with the keyword rather than being pulled past it.
            let before_usize = (before as usize).min(source.len());
            if comment_end < before_usize
                && has_non_comment_text(
                    source,
                    comment_end,
                    before_usize,
                    self.comments,
                    cursor + 1,
                )
            {
                break;
            }

            let text = &source[t.offset as usize..comment_end];

            let gap_start = (last_end as usize).min(source.len());
            let gap_end = (t.offset as usize).min(source.len());
            let has_newline = gap_start < gap_end && source[gap_start..gap_end].contains('\n');

            match t.kind {
                CommentKind::LineComment => {
                    if has_newline {
                        leading.push(arena.hardline());
                        leading.push(arena.text(text));
                        leading.push(arena.hardline());
                    } else {
                        let space = arena.text(" ");
                        let comment = arena.text(text);
                        let inner = arena.cat(space, comment);
                        trailing.push(arena.line_suffix(inner));
                        trailing.push(arena.break_parent());
                    }
                }
                CommentKind::BlockComment => {
                    if has_newline {
                        leading.push(arena.hardline());
                        leading.push(arena.text(text));
                    } else {
                        trailing.push(arena.text(" "));
                        trailing.push(arena.text(text));
                    }
                }
            }

            last_end = t.offset + t.length;
            cursor += 1;
        }

        self.cursor.set(cursor);

        DrainResult {
            trailing: arena.cats(&trailing),
            leading: arena.cats(&leading),
        }
    }

    /// Peek at the next N tokens (one per whitespace-separated word in the keyword)
    /// without advancing the token cursor.
    pub fn peek_keyword_tokens(&self, kw_text: &str) -> Option<(u32, usize)> {
        let word_count = kw_text.trim().split_whitespace().count();
        if word_count == 0 {
            return None;
        }
        let first_idx = self.token_cursor.get();
        if first_idx + word_count > self.tokens.len() {
            return None;
        }
        let first_offset = self.tokens[first_idx].offset;
        Some((first_offset, word_count))
    }

    /// Advance the token cursor by `n` positions.
    pub fn advance_token_cursor(&self, n: usize) {
        self.token_cursor.set(self.token_cursor.get() + n);
    }

    /// Advance the token cursor past all tokens whose offset < end_offset.
    pub fn advance_past(&self, end_offset: u32) {
        let mut idx = self.token_cursor.get();
        while idx < self.tokens.len() && self.tokens[idx].offset < end_offset {
            idx += 1;
        }
        self.token_cursor.set(idx);
    }

    /// Peek at the next undrained comment without advancing the cursor.
    pub fn peek_comment(&self) -> Option<&Comment> {
        let idx = self.cursor.get();
        if idx < self.comments.len() {
            Some(&self.comments[idx])
        } else {
            None
        }
    }

    /// Advance the comment cursor by one.
    pub fn advance_comment(&self) {
        let idx = self.cursor.get();
        if idx < self.comments.len() {
            self.cursor.set(idx + 1);
        }
    }

    /// Peek at the next token's offset without advancing the token cursor.
    pub fn peek_next_token(&self) -> Option<(u32, u32)> {
        let idx = self.token_cursor.get();
        if idx < self.tokens.len() {
            let tp = &self.tokens[idx];
            Some((tp.offset, tp.length))
        } else {
            None
        }
    }

    /// Flush all remaining comments.
    pub fn drain_remaining(&self, source: &'a str, arena: &mut DocArena<'a>) -> DocId {
        let drain = self.drain_before(u32::MAX, source, arena);
        arena.cat(drain.trailing, drain.leading)
    }
}

/// Check whether a byte range contains non-whitespace text that isn't covered
/// by a comment. `comments` must be sorted by offset; scanning starts at
/// `comment_start_idx`.
fn has_non_comment_text(
    source: &str,
    start: usize,
    end: usize,
    comments: &[Comment],
    comment_start_idx: usize,
) -> bool {
    let src = source.as_bytes();
    let mut pos = start;
    let mut ci = comment_start_idx;

    while pos < end {
        // Skip over any comment region that covers `pos`.
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

/// Insert drained comments into the parts list, respecting pending line breaks.
///
/// - Trailing comments (LineSuffix) go before any pending lines
/// - If there are leading comments, they already start with a HardLine,
///   so pending lines are dropped to avoid a double line break
/// - If there are no leading comments, pending lines are flushed normally
pub fn flush_comments(drain: DrainResult, pending_lines: &mut Vec<DocId>, parts: &mut Vec<DocId>) {
    if drain.trailing != NIL_DOC {
        parts.push(drain.trailing);
    }
    if drain.leading != NIL_DOC {
        // Leading comments already start with a HardLine — drop pending
        // lines to avoid an extra blank line.
        pending_lines.clear();
        parts.push(drain.leading);
    } else {
        parts.extend(pending_lines.drain(..));
    }
}
