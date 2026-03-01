// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use super::FormatConfig;
use super::comment::CommentCtx;
use super::doc::{DocArena, DocId, NIL_DOC, RenderBuffers};
use super::interpret::{FmtCtx, InterpretScratch, interpret_node};
use crate::dialect::Dialect;
use crate::parser::ffi::CommentKind;
use crate::parser::ffi::MacroRegion;
use crate::parser::nodes::Fields;
use crate::parser::session::RawParser;

/// High-level SQL formatter. Created from a `Dialect`, reusable across inputs.
pub struct Formatter<'d> {
    dialect: Dialect<'d>,
    parser: RawParser<'d>,
    config: FormatConfig,
    /// Reusable scratch arena — cleared between format calls to avoid
    /// re-allocating the backing Vec.
    arena: DocArena<'static>,
    /// Reusable scratch buffers for the bytecode interpreter, shared across
    /// all recursive `interpret` calls within a single `format()` invocation.
    interpret_scratch: InterpretScratch,
    /// Reusable render buffers — recycled between format calls.
    render_bufs: RenderBuffers,
}

// SAFETY: Dialect is Send+Sync, Parser is Send.
unsafe impl Send for Formatter<'_> {}

#[cfg(feature = "sqlite")]
impl Default for Formatter<'static> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'d> Formatter<'d> {
    /// Create a formatter for the built-in SQLite dialect with default configuration.
    #[cfg(feature = "sqlite")]
    pub fn new() -> Formatter<'static> {
        Formatter::builder(&crate::sqlite::DIALECT).build()
    }

    /// Create a builder for a formatter bound to the given dialect.
    pub fn builder(dialect: &'d Dialect<'d>) -> FormatterBuilder<'d> {
        FormatterBuilder {
            dialect,
            format_config: FormatConfig::default(),
            dialect_config: None,
        }
    }

    /// Access the current configuration.
    pub fn config(&self) -> &FormatConfig {
        &self.config
    }

    /// Format SQL source text. Handles multiple statements and preserves comments.
    pub fn format(&mut self, source: &str) -> Result<String, crate::parser::session::ParseError> {
        let mut cursor = self.parser.parse(source);

        let mut roots = Vec::new();
        while let Some(result) = cursor.next_statement() {
            roots.push(result?.id());
        }

        let state = cursor.state();
        let comments = state.comments();
        let tokens = state.tokens();
        let source = state.source();

        if roots.is_empty() {
            return Ok(String::new());
        }

        // Recycle the arena from the previous call (reuses the Vec allocation).
        let prev_arena = std::mem::replace(&mut self.arena, DocArena::new());
        let mut arena = DocArena::recycle(prev_arena);

        let comment_ctx = if comments.is_empty() {
            None
        } else {
            Some(CommentCtx::new(comments, tokens))
        };
        let comment_ctx_ref = comment_ctx.as_ref();
        let semicolons = self.config.semicolons;

        let mut parts: Vec<DocId> = Vec::new();
        for (i, &root_id) in roots.iter().enumerate() {
            if i > 0 {
                emit_stmt_separator(&comment_ctx, semicolons, source, &mut arena, &mut parts);
            } else if let Some(cctx) = &comment_ctx
                && let Some((next_offset, _)) = cctx.peek_next_token()
            {
                drain_gap_comments(cctx, next_offset, source, &mut arena, &mut parts);
            }

            let ctx = FmtCtx {
                dialect: self.dialect,
                reader: state.reader,
                comment_ctx: comment_ctx_ref,
            };
            let mut consumed = 0u64;
            parts.push(interpret_node(
                &ctx,
                root_id,
                &mut consumed,
                &mut arena,
                &mut self.interpret_scratch,
            ));
        }

        if let Some(cctx) = &comment_ctx {
            parts.push(cctx.drain_remaining(source, &mut arena));
        }

        let doc = arena.cats(&parts);

        // Reuse the render buffers from the previous call.
        let mut bufs = std::mem::take(&mut self.render_bufs);
        bufs.clear();

        arena.render_into(
            doc,
            self.config.line_width,
            self.config.keyword_case,
            &mut bufs,
        );

        if semicolons {
            bufs.out.push(';');
        }
        bufs.out.push('\n');

        // Recycle arena and render buffers back for next call.
        self.arena = DocArena::recycle(arena);
        let result = std::mem::take(&mut bufs.out);
        self.render_bufs = bufs;

        Ok(result)
    }

    /// Format a single pre-parsed AST node. This is the low-level entry point
    /// for cases where the caller controls parsing (e.g. macro expansion).
    pub fn format_node(&self, node: crate::parser::session::NodeRef<'_>) -> String {
        let reader = node.reader();
        let mut arena = DocArena::new();
        let tokens = reader.tokens();
        let comment_ctx = CommentCtx::new(&[], tokens);
        let ctx = FmtCtx {
            dialect: self.dialect,
            reader,
            comment_ctx: Some(&comment_ctx),
        };
        let mut consumed = 0u64;
        let mut scratch = InterpretScratch::new();
        let doc = interpret_node(&ctx, node.id(), &mut consumed, &mut arena, &mut scratch);
        arena.render(doc, self.config.line_width, self.config.keyword_case)
    }
}

// ── FormatterBuilder ────────────────────────────────────────────────────

/// Builder for configuring a [`Formatter`] before construction.
pub struct FormatterBuilder<'d> {
    dialect: &'d Dialect<'d>,
    format_config: FormatConfig,
    dialect_config: Option<syntaqlite_parser::dialect::ffi::DialectConfig>,
}

impl<'d> FormatterBuilder<'d> {
    /// Set the format configuration (line width, keyword case, etc.).
    pub fn format_config(mut self, config: FormatConfig) -> Self {
        self.format_config = config;
        self
    }

    /// Set dialect config for version/cflag-gated tokenization.
    pub fn dialect_config(mut self, config: syntaqlite_parser::dialect::ffi::DialectConfig) -> Self {
        self.dialect_config = Some(config);
        self
    }

    /// Build the formatter.
    ///
    /// # Panics
    ///
    /// Panics if the dialect was not compiled with formatter bytecode
    /// (i.e. the `.synq` definitions have no `fmt` blocks).
    pub fn build(self) -> Formatter<'d> {
        assert!(
            self.dialect.has_fmt_data(),
            "dialect has no formatter bytecode — ensure .synq definitions include fmt blocks",
        );
        let mut parser_builder = RawParser::builder(self.dialect).collect_tokens(true);
        if let Some(dc) = self.dialect_config {
            parser_builder = parser_builder.dialect_config(dc);
        }
        let parser = parser_builder.build();
        Formatter {
            dialect: *self.dialect,
            parser,
            config: self.format_config,
            arena: DocArena::with_capacity(256),
            interpret_scratch: InterpretScratch::new(),
            render_bufs: RenderBuffers::new(),
        }
    }
}

// ── Multi-statement helpers ─────────────────────────────────────────────

fn emit_stmt_separator<'a>(
    comment_ctx: &Option<CommentCtx<'a>>,
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

/// Drain trailing comments from the inter-statement gap (same line as the
/// previous statement's end). Stops at the first newline.
fn drain_trailing_gap<'a>(
    ctx: &CommentCtx<'a>,
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
            CommentKind::LineComment => {
                let space = arena.text(" ");
                let comment = arena.text(text);
                let inner = arena.cat(space, comment);
                parts.push(arena.line_suffix(inner));
                parts.push(arena.break_parent());
            }
            CommentKind::BlockComment => {
                parts.push(arena.text(" "));
                parts.push(arena.text(text));
            }
        }
        last_end = c.offset + c.length;
        ctx.advance_comment();
    }
}

/// Drain leading comments from the inter-statement gap (each on its own line).
fn drain_gap_comments<'a>(
    ctx: &CommentCtx<'a>,
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
///
/// Returns:
/// - `Some(doc)` with verbatim text on first encounter (consumed bit gets set).
/// - `Some(NIL_DOC)` if the region was already consumed (child should be suppressed).
/// - `None` if the next token is not inside any macro region.
///
/// Does NOT advance the token cursor — the caller is responsible for
/// advancing it (typically by "calling" into the child node).
pub(crate) fn try_macro_verbatim<'a>(
    ctx: &FmtCtx<'a>,
    regions: &[MacroRegion],
    arena: &mut DocArena<'a>,
    consumed: &mut u64,
) -> Option<DocId> {
    let cctx = ctx.comment_ctx?;
    let (tok_offset, _) = cctx.peek_next_token()?;
    let source = ctx.source();

    for (i, r) in regions.iter().enumerate() {
        if i >= 64 {
            break;
        }
        let r_start = r.call_offset;
        let r_end = r_start + r.call_length;

        if tok_offset >= r_start && tok_offset < r_end {
            let bit = 1u64 << i;
            if *consumed & bit != 0 {
                // Already consumed — suppress this child.
                return Some(NIL_DOC);
            }
            *consumed |= bit;
            return Some(arena.text(&source[r_start as usize..r_end as usize]));
        }
    }
    None
}

// ── Field extraction ────────────────────────────────────────────────────

/// Extract typed field values from a raw node pointer.
#[inline]
pub(crate) fn extract_fields<'a>(
    dialect: &Dialect<'_>,
    ptr: *const u8,
    tag: u32,
    source: &'a str,
) -> Fields<'a> {
    // SAFETY: caller guarantees ptr is a valid arena pointer for a node with
    // the given tag; delegating to the shared implementation.
    unsafe { crate::dialect::extract_fields(dialect, ptr, tag, source) }
}
