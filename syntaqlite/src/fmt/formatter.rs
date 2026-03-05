// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite_syntax::any::MacroRegion;
use syntaqlite_syntax::{CommentKind, Parser, ParserConfig};

use super::FormatConfig;
use super::FormatError;
use super::comment::{CommentCtx, CommentEntry, TokenEntry};
use super::doc::{DocArena, DocId, NIL_DOC, RenderBuffers};
use super::interpret::{FmtCtx, InterpretScratch, interpret_node};
use crate::dialect::Dialect;

/// High-level SQL formatter. Created from a `Dialect`, reusable across inputs.
pub struct Formatter {
    dialect: Dialect,
    parser: Parser,
    config: FormatConfig,
    arena: DocArena<'static>,
    interpret_scratch: InterpretScratch,
    render_bufs: RenderBuffers,
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
        }
    }

    /// Format SQL source text. Handles multiple statements and preserves comments.
    ///
    /// # Errors
    /// Returns [`FormatError`] when parsing fails for any statement in `source`.
    pub fn format(&mut self, source: &str) -> Result<String, FormatError> {
        let mut session = self.parser.parse(source);
        let mut result = String::new();
        let mut stmt_num: usize = 0;

        while let Some(stmt) = session.next() {
            let stmt = match stmt {
                Ok(stmt) => stmt,
                Err(e) => {
                    return Err(FormatError {
                        message: e.message().to_owned(),
                        offset: e.offset(),
                        length: e.length(),
                    });
                }
            };

            let erased = stmt.erase();
            let stmt_source = erased.source();

            let macro_regions: Vec<MacroRegion> = erased.macro_regions().collect();

            let comments: Vec<CommentEntry> = erased
                .comments()
                .map(|c| CommentEntry {
                    offset: byte_offset_in(source, c.text.as_ptr()),
                    length: usize_to_u32(c.text.len()),
                    kind: c.kind,
                })
                .collect();

            let tokens: Vec<TokenEntry> = erased
                .tokens()
                .map(|t| TokenEntry {
                    offset: byte_offset_in(source, t.text().as_ptr()),
                    length: usize_to_u32(t.text().len()),
                })
                .collect();

            let root_id = erased.root_id();
            let semicolons = self.config.semicolons;
            let has_comments = !comments.is_empty();

            let comment_ctx = if has_comments {
                Some(CommentCtx::new(comments, tokens))
            } else {
                None
            };

            // Fresh arena for this statement — drops borrows from the previous iteration.
            let prev_arena = std::mem::replace(&mut self.arena, DocArena::new());
            let mut arena = DocArena::recycle(prev_arena);
            let mut parts: Vec<DocId> = Vec::new();

            if stmt_num > 0 {
                emit_stmt_separator(
                    comment_ctx.as_ref(),
                    semicolons,
                    stmt_source,
                    &mut arena,
                    &mut parts,
                );
            } else if let Some(cctx) = comment_ctx.as_ref()
                && let Some((next_offset, _)) = cctx.peek_next_token()
            {
                drain_gap_comments(cctx, next_offset, stmt_source, &mut arena, &mut parts);
            }

            let ctx = FmtCtx {
                dialect: self.dialect,
                reader: erased,
                comment_ctx,
                macro_regions,
            };
            let mut consumed = vec![false; ctx.macro_regions.len()];
            parts.push(interpret_node(
                &ctx,
                root_id,
                &mut consumed,
                &mut arena,
                &mut self.interpret_scratch,
            ));

            if let Some(cctx) = ctx.comment_ctx.as_ref() {
                parts.push(cctx.drain_remaining(stmt_source, &mut arena));
            }

            // Render this statement immediately while `erased`/`ctx` still borrow session.
            let doc = arena.cats(&parts);
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
