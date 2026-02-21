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

/// Tracks position through a sorted comment array during Doc tree construction.
pub struct CommentCtx<'a> {
    comments: &'a [Comment],
    tokens: &'a [TokenPos],
    source: &'a str,
    cursor: Cell<usize>,
    token_cursor: Cell<usize>,
    last_source_end: Cell<u32>,
}

impl<'a> CommentCtx<'a> {
    pub fn new(comments: &'a [Comment], tokens: &'a [TokenPos], source: &'a str) -> Self {
        CommentCtx {
            comments,
            tokens,
            source,
            cursor: Cell::new(0),
            token_cursor: Cell::new(0),
            last_source_end: Cell::new(0),
        }
    }

    /// Update the tracked source position (call after processing a Span).
    pub fn set_source_end(&self, offset: u32) {
        if offset > self.last_source_end.get() {
            self.last_source_end.set(offset);
        }
    }

    /// Drain all comments with offset < `before`.
    ///
    /// Stops early if there is non-whitespace source text (i.e. a keyword)
    /// between a comment and `before`. This prevents comments that precede
    /// an intervening keyword from being attributed to the child after the
    /// keyword.
    pub fn drain_before(&self, before: u32, arena: &mut DocArena<'a>) -> DrainResult {
        let mut trailing = Vec::new();
        let mut leading = Vec::new();
        let mut cursor = self.cursor.get();
        while cursor < self.comments.len() && self.comments[cursor].offset < before {
            let t = &self.comments[cursor];
            let comment_end = (t.offset + t.length) as usize;

            // Check if there is non-whitespace, non-comment source text
            // between the end of this comment and `before`. If so, there's
            // an intervening keyword — stop draining so this comment stays
            // with the keyword rather than being pulled past it.
            let before_usize = (before as usize).min(self.source.len());
            if comment_end < before_usize
                && has_non_comment_text(
                    self.source,
                    comment_end,
                    before_usize,
                    self.comments,
                    cursor + 1,
                )
            {
                break;
            }

            let text = &self.source[t.offset as usize..comment_end];

            let last_end = self.last_source_end.get();
            let gap_start = (last_end as usize).min(self.source.len());
            let gap_end = (t.offset as usize).min(self.source.len());
            let has_newline = gap_start < gap_end && self.source[gap_start..gap_end].contains('\n');

            match t.kind {
                CommentKind::LineComment => {
                    if has_newline {
                        // Leading: comment on its own line.
                        // HardLine before puts the comment on a new line,
                        // HardLine after ensures the next token doesn't
                        // concatenate with the comment text.
                        leading.push(arena.hardline());
                        leading.push(arena.text(text));
                        leading.push(arena.hardline());
                    } else {
                        // Trailing: comment at end of current line
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

            self.last_source_end.set(t.offset + t.length);
            cursor += 1;
        }

        self.cursor.set(cursor);

        DrainResult {
            trailing: arena.cats(&trailing),
            leading: arena.cats(&leading),
        }
    }

    /// Advance past N tokens (one per whitespace-separated word in the keyword).
    /// Returns (first_offset, last_end) for drain_before and set_source_end.
    /// The keyword is trimmed before counting words.
    pub fn next_keyword_tokens(&self, kw_text: &str) -> Option<(u32, u32)> {
        let word_count = kw_text.trim().split_whitespace().count();
        if word_count == 0 {
            return None;
        }
        let first_idx = self.token_cursor.get();
        if first_idx + word_count > self.tokens.len() {
            return None;
        }
        let first_offset = self.tokens[first_idx].offset;
        let last = &self.tokens[first_idx + word_count - 1];
        let last_end = last.offset + last.length;
        self.token_cursor.set(first_idx + word_count);
        Some((first_offset, last_end))
    }

    /// Advance the token cursor past all tokens whose offset < end_offset.
    /// Used after processing a span to keep the token cursor in sync.
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

    /// Advance the comment cursor by one (call after peek_comment).
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

    /// Return the current tracked source end position.
    pub fn last_source_end(&self) -> u32 {
        self.last_source_end.get()
    }

    /// Flush all remaining comments (for end-of-statement trailing comments).
    pub fn drain_remaining(&self, arena: &mut DocArena<'a>) -> DocId {
        let drain = self.drain_before(u32::MAX, arena);
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
