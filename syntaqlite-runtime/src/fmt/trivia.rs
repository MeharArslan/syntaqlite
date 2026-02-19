use std::cell::Cell;

use crate::parser::{Trivia, TriviaKind};

use super::doc::{DocArena, DocId, NIL_DOC};

/// Result of draining trivia items. Trailing docs (e.g. LineSuffix for
/// end-of-line comments) go BEFORE any pending line break. Leading docs
/// (comments on their own line) go AFTER any pending line break.
pub struct DrainResult {
    pub trailing: DocId,
    pub leading: DocId,
}

/// Tracks position through a sorted trivia array during Doc tree construction.
pub struct TriviaCtx<'a> {
    trivia: &'a [Trivia],
    source: &'a str,
    cursor: Cell<usize>,
    last_source_end: Cell<u32>,
}

impl<'a> TriviaCtx<'a> {
    pub fn new(trivia: &'a [Trivia], source: &'a str) -> Self {
        TriviaCtx {
            trivia,
            source,
            cursor: Cell::new(0),
            last_source_end: Cell::new(0),
        }
    }

    /// Update the tracked source position (call after processing a Span).
    pub fn set_source_end(&self, offset: u32) {
        if offset > self.last_source_end.get() {
            self.last_source_end.set(offset);
        }
    }

    /// Drain all trivia items with offset < `before`.
    pub fn drain_before(&self, before: u32, arena: &mut DocArena<'a>) -> DrainResult {
        let mut trailing = Vec::new();
        let mut leading = Vec::new();
        let mut cursor = self.cursor.get();
        while cursor < self.trivia.len() && self.trivia[cursor].offset < before {
            let t = &self.trivia[cursor];
            let text = &self.source[t.offset as usize..(t.offset + t.length) as usize];

            let last_end = self.last_source_end.get();
            let gap_start = (last_end as usize).min(self.source.len());
            let gap_end = (t.offset as usize).min(self.source.len());
            let has_newline = gap_start < gap_end && self.source[gap_start..gap_end].contains('\n');

            match t.kind {
                TriviaKind::LineComment => {
                    if has_newline {
                        // Leading: comment on its own line.
                        leading.push(arena.hardline());
                        leading.push(arena.text(text));

                    } else {
                        // Trailing: comment at end of current line
                        let space = arena.text(" ");
                        let comment = arena.text(text);
                        let inner = arena.cat(space, comment);
                        trailing.push(arena.line_suffix(inner));
                        trailing.push(arena.break_parent());
                    }
                }
                TriviaKind::BlockComment => {
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

    /// Flush all remaining trivia (for end-of-statement trailing comments).
    pub fn drain_remaining(&self, arena: &mut DocArena<'a>) -> DocId {
        let drain = self.drain_before(u32::MAX, arena);
        arena.cat(drain.trailing, drain.leading)
    }
}

/// Insert drained trivia into the parts list, respecting pending line breaks.
///
/// - Trailing trivia (LineSuffix) goes before any pending lines
/// - Pending lines are flushed
/// - Leading trivia (own-line comments) goes after pending lines
pub fn flush_trivia(
    drain: DrainResult,
    pending_lines: &mut Vec<DocId>,
    parts: &mut Vec<DocId>,
) {
    if drain.trailing != NIL_DOC {
        parts.push(drain.trailing);
    }
    parts.extend(pending_lines.drain(..));
    if drain.leading != NIL_DOC {
        parts.push(drain.leading);
    }
}
