// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite_syntax::any::MacroRegion;
use syntaqlite_syntax::{CommentKind, ParseOutcome, Parser, ParserConfig};

use super::FormatConfig;
use super::FormatError;
use super::comment::{CommentCtx, CommentEntry, TokenEntry};
use super::doc::{DocArena, DocId, NIL_DOC, RenderBuffers};
use super::interpret::{FmtCtx, InterpretScratch};
use crate::dialect::Dialect;

/// High-level SQL formatter. Created from a `Dialect`, reusable across inputs.
pub struct Formatter {
    pub(super) dialect: Dialect,
    pub(super) parser: Parser,
    pub(super) config: FormatConfig,
    // Statement-scoped state cached on the formatter to avoid per-statement allocations.
    pub(super) arena: DocArena<'static>,
    pub(super) interpret_scratch: InterpretScratch,
    pub(super) render_bufs: RenderBuffers,
    pub(super) macro_regions: Vec<MacroRegion>,
    pub(super) comment_entries: Vec<CommentEntry>,
    pub(super) token_entries: Vec<TokenEntry>,
    pub(super) parts: Vec<DocId>,
    pub(super) consumed_regions: Vec<bool>,
}

#[cfg(feature = "sqlite")]
impl Default for Formatter {
    fn default() -> Self {
        Self::new()
    }
}

impl Formatter {
    /// Create a formatter for the built-in `SQLite` dialect with default configuration.
    #[cfg(feature = "sqlite")]
    pub fn new() -> Formatter {
        Formatter::with_config(&FormatConfig::default())
    }

    /// Create a formatter for the built-in `SQLite` dialect with custom configuration.
    #[cfg(feature = "sqlite")]
    pub fn with_config(format_config: &FormatConfig) -> Formatter {
        Formatter::with_dialect_config(crate::sqlite::dialect::dialect(), format_config)
    }

    /// Create a formatter bound to the given dialect with custom configuration.
    pub(crate) fn with_dialect_config(dialect: Dialect, format_config: &FormatConfig) -> Self {
        assert!(
            dialect.has_fmt_data(),
            "dialect has no formatter bytecode — ensure .synq definitions include fmt blocks",
        );
        let parser = Parser::with_config(&ParserConfig::default().with_collect_tokens(true));
        Formatter {
            dialect,
            parser,
            config: format_config.clone(),
            arena: DocArena::with_capacity(256),
            interpret_scratch: InterpretScratch::new(),
            render_bufs: RenderBuffers::new(),
            macro_regions: Vec::with_capacity(32),
            comment_entries: Vec::with_capacity(64),
            token_entries: Vec::with_capacity(256),
            parts: Vec::with_capacity(64),
            consumed_regions: Vec::with_capacity(32),
        }
    }

    /// Format SQL source text. Handles multiple statements and preserves comments.
    ///
    /// Pipeline overview per statement:
    /// 1. Parse and collect token/comment/macro metadata.
    /// 2. Interpret formatter bytecode into Doc fragments.
    /// 3. Render Doc fragments with a Wadler-style pretty-printer (`DocArena`).
    /// 4. Recycle temporary buffers for the next statement.
    ///
    /// # Errors
    /// Returns [`FormatError`] when parsing fails for any statement in `source`.
    pub fn format(&mut self, source: &str) -> Result<String, FormatError> {
        let mut session = self.parser.parse(source);
        let mut result = String::new();
        let mut stmt_num: usize = 0;

        loop {
            let stmt = match session.next() {
                ParseOutcome::Done => break,
                ParseOutcome::Ok(stmt) => stmt,
                ParseOutcome::Err(e) => {
                    return Err(FormatError {
                        message: e.message().to_owned(),
                        offset: e.offset(),
                        length: e.length(),
                    });
                }
            };

            let erased = stmt.erase();
            let stmt_source = erased.source();

            // Stage 1: Collect parser side-channels the interpreter needs.
            self.macro_regions.clear();
            self.macro_regions.extend(erased.macro_regions());

            self.comment_entries.clear();
            self.comment_entries
                .extend(erased.comments().map(|c| CommentEntry {
                    offset: byte_offset_in(source, c.text.as_ptr()),
                    length: usize_to_u32(c.text.len()),
                    kind: c.kind,
                }));

            self.token_entries.clear();
            self.token_entries
                .extend(erased.tokens().map(|t| TokenEntry {
                    offset: byte_offset_in(source, t.text().as_ptr()),
                    length: usize_to_u32(t.text().len()),
                }));

            let root_id = erased.root_id();
            let semicolons = self.config.semicolons;
            let has_comments = !self.comment_entries.is_empty();

            let comment_ctx = if has_comments {
                // Move buffers into CommentCtx for this statement, then reclaim them after render.
                Some(CommentCtx::new(
                    std::mem::take(&mut self.comment_entries),
                    std::mem::take(&mut self.token_entries),
                ))
            } else {
                None
            };

            // Fresh arena for this statement — drops borrows from the previous iteration.
            let prev_arena = std::mem::replace(&mut self.arena, DocArena::new());
            let mut arena = DocArena::recycle(prev_arena);
            self.parts.clear();

            if stmt_num > 0 {
                emit_stmt_separator(
                    comment_ctx.as_ref(),
                    semicolons,
                    stmt_source,
                    &mut arena,
                    &mut self.parts,
                );
            } else if let Some(cctx) = comment_ctx.as_ref()
                && let Some((next_offset, _)) = cctx.peek_next_token()
            {
                drain_gap_comments(cctx, next_offset, stmt_source, &mut arena, &mut self.parts);
            }

            // Stage 2: Interpret bytecode for this AST into Doc fragments.
            let mut ctx = FmtCtx {
                dialect: self.dialect,
                reader: erased,
                comment_ctx,
                macro_regions: std::mem::take(&mut self.macro_regions),
            };
            let interpreted = self.interpret_node(&ctx, root_id, &mut arena);
            self.parts.push(interpreted);

            if let Some(cctx) = ctx.comment_ctx.as_ref() {
                self.parts
                    .push(cctx.drain_remaining(stmt_source, &mut arena));
            }

            // Stage 3: Render Docs via the Wadler-style group/flat/break algorithm.
            // Rendering happens here while `erased`/`ctx` still borrow parser session data.
            let doc = arena.cats(&self.parts);
            let mut bufs = std::mem::take(&mut self.render_bufs);
            bufs.clear();
            arena.render_into(
                doc,
                self.config.line_width,
                self.config.keyword_case,
                &mut bufs,
            );
            result.push_str(&bufs.out);
            self.render_bufs = bufs;

            // Stage 4: Recover and recycle statement-scoped buffers.
            if let Some(cctx) = ctx.comment_ctx.take() {
                let (comments, tokens) = cctx.into_parts();
                self.comment_entries = comments;
                self.token_entries = tokens;
            }
            self.macro_regions = ctx.macro_regions;

            // Recycle the arena, releasing all Doc borrows from this iteration.
            // `ctx`, `erased`, and `stmt` drop at end of loop body, releasing session borrow.
            self.arena = DocArena::recycle(arena);

            stmt_num += 1;
        }

        if stmt_num == 0 {
            return Ok(String::new());
        }

        if self.config.semicolons {
            result.push(';');
        }
        result.push('\n');

        Ok(result)
    }
}

// ── Multi-statement helpers ─────────────────────────────────────────────

fn emit_stmt_separator<'a>(
    comment_ctx: Option<&CommentCtx>,
    semicolons: bool,
    source: &'a str,
    arena: &mut DocArena<'a>,
    parts: &mut Vec<DocId>,
) {
    if let Some(cctx) = comment_ctx
        && let Some((next_offset, _)) = cctx.peek_next_token()
    {
        if semicolons {
            parts.push(arena.text(";"));
        }
        drain_trailing_gap(cctx, next_offset, source, arena, parts);
        parts.push(arena.hardline());
        parts.push(arena.hardline());
        drain_gap_comments(cctx, next_offset, source, arena, parts);
        return;
    }
    if semicolons {
        parts.push(arena.text(";"));
    }
    parts.push(arena.hardline());
    parts.push(arena.hardline());
}

#[inline]
fn usize_to_u32(value: usize) -> u32 {
    u32::try_from(value).expect("value must fit in u32")
}

#[inline]
fn byte_offset_in(source: &str, ptr: *const u8) -> u32 {
    let base = source.as_ptr() as usize;
    let start = ptr as usize;
    let offset = start
        .checked_sub(base)
        .expect("span pointer must be within source");
    usize_to_u32(offset)
}

fn drain_trailing_gap<'a>(
    ctx: &CommentCtx,
    before: u32,
    source: &'a str,
    arena: &mut DocArena<'a>,
    parts: &mut Vec<DocId>,
) {
    let mut last_end = ctx.prev_token_end();
    while let Some(c) = ctx.peek_comment() {
        if c.offset >= before {
            break;
        }
        let gap_start = (last_end as usize).min(source.len());
        let gap_end = (c.offset as usize).min(source.len());
        if gap_start < gap_end && source[gap_start..gap_end].contains('\n') {
            break;
        }
        let text = &source[c.offset as usize..(c.offset + c.length) as usize];
        match c.kind {
            CommentKind::Line => {
                let space = arena.text(" ");
                let comment = arena.text(text);
                let inner = arena.cat(space, comment);
                parts.push(arena.line_suffix(inner));
                parts.push(arena.break_parent());
            }
            CommentKind::Block => {
                parts.push(arena.text(" "));
                parts.push(arena.text(text));
            }
        }
        last_end = c.offset + c.length;
        ctx.advance_comment();
    }
}

fn drain_gap_comments<'a>(
    ctx: &CommentCtx,
    before: u32,
    source: &'a str,
    arena: &mut DocArena<'a>,
    parts: &mut Vec<DocId>,
) {
    while let Some(c) = ctx.peek_comment() {
        if c.offset >= before {
            break;
        }
        let text = &source[c.offset as usize..(c.offset + c.length) as usize];
        parts.push(arena.text(text));
        parts.push(arena.hardline());
        ctx.advance_comment();
    }
}

// ── Single-node formatting ──────────────────────────────────────────────

/// Check if the next token falls within a macro region.
pub(crate) fn try_macro_verbatim<'a>(
    ctx: &FmtCtx<'a>,
    regions: &[MacroRegion],
    arena: &mut DocArena<'a>,
    consumed: &mut [bool],
) -> Option<DocId> {
    let cctx = ctx.comment_ctx.as_ref()?;
    let (tok_offset, _) = cctx.peek_next_token()?;
    let source = ctx.source();

    for (i, r) in regions.iter().enumerate() {
        let r_start = r.call_offset;
        let r_end = r_start + r.call_length;

        if tok_offset >= r_start && tok_offset < r_end {
            if consumed[i] {
                return Some(NIL_DOC);
            }
            consumed[i] = true;
            return Some(arena.text(&source[r_start as usize..r_end as usize]));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fmt::KeywordCase;

    fn render_parts<'a>(arena: &mut DocArena<'a>, parts: &[DocId]) -> String {
        let root = arena.cats(parts);
        arena.render(root, 80, KeywordCase::Preserve)
    }

    #[test]
    fn byte_offset_in_returns_expected_offsets() {
        let src = "SELECT 1";
        assert_eq!(byte_offset_in(src, src.as_ptr()), 0);
        assert_eq!(byte_offset_in(src, src[7..].as_ptr()), 7);
    }

    #[test]
    fn emit_stmt_separator_without_comments_with_semicolon() {
        let source = "SELECT 1";
        let mut arena = DocArena::new();
        let mut parts = Vec::new();
        emit_stmt_separator(None, true, source, &mut arena, &mut parts);
        assert_eq!(render_parts(&mut arena, &parts), ";\n\n");
    }

    #[test]
    fn emit_stmt_separator_without_comments_without_semicolon() {
        let source = "SELECT 1";
        let mut arena = DocArena::new();
        let mut parts = Vec::new();
        emit_stmt_separator(None, false, source, &mut arena, &mut parts);
        assert_eq!(render_parts(&mut arena, &parts), "\n\n");
    }

    #[test]
    fn emit_stmt_separator_emits_inline_block_comment_before_break() {
        let source = "/*x*/SELECT";
        let ctx = CommentCtx::new(
            vec![CommentEntry {
                offset: 0,
                length: 5,
                kind: CommentKind::Block,
            }],
            vec![TokenEntry {
                offset: 5,
                length: 6,
            }],
        );
        let mut arena = DocArena::new();
        let mut parts = Vec::new();
        emit_stmt_separator(Some(&ctx), true, source, &mut arena, &mut parts);
        assert_eq!(render_parts(&mut arena, &parts), "; /*x*/\n\n");
    }

    #[test]
    fn drain_gap_comments_writes_each_comment_on_own_line() {
        let source = "--a\n/*b*/SELECT";
        let ctx = CommentCtx::new(
            vec![
                CommentEntry {
                    offset: 0,
                    length: 3,
                    kind: CommentKind::Line,
                },
                CommentEntry {
                    offset: 4,
                    length: 5,
                    kind: CommentKind::Block,
                },
            ],
            vec![TokenEntry {
                offset: 9,
                length: 6,
            }],
        );
        let mut arena = DocArena::new();
        let mut parts = Vec::new();
        drain_gap_comments(&ctx, 9, source, &mut arena, &mut parts);
        assert_eq!(render_parts(&mut arena, &parts), "--a\n/*b*/\n");
    }
}
