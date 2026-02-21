// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::Cell;

use crate::dialect::Dialect;
use crate::dialect::ffi::{
    FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN, FieldMeta,
};
use crate::parser::{
    Comment, CommentKind, CursorBase, FieldVal, Fields, MacroRegion, NodeId, Parser,
    ParserConfig, SourceSpan, TokenPos,
};
use super::interpret::CommentInfo;

use super::FormatConfig;
use super::comment::CommentCtx;
use super::doc::{DocArena, DocId, NIL_DOC};
use super::interpret::Interpreter;
use super::render::render;

// ── Formatter ───────────────────────────────────────────────────────────

/// High-level SQL formatter. Created from a `Dialect`, reusable across inputs.
pub struct Formatter<'d> {
    dialect: Dialect<'d>,
    parser: Parser,
    config: FormatConfig,
}

// SAFETY: Dialect is Send+Sync, Parser is Send.
unsafe impl Send for Formatter<'_> {}

impl<'d> Formatter<'d> {
    /// Create a formatter for the given dialect with default configuration.
    pub fn new(dialect: &Dialect<'d>) -> Result<Self, &'static str> {
        Self::with_config(dialect, FormatConfig::default())
    }

    /// Create a formatter with the given configuration.
    pub fn with_config(dialect: &Dialect<'d>, config: FormatConfig) -> Result<Self, &'static str> {
        if !dialect.has_fmt_data() {
            return Err("C dialect has no fmt data");
        }
        let parser_config = ParserConfig {
            collect_tokens: true,
            ..Default::default()
        };
        let parser = Parser::with_config(dialect, &parser_config);
        Ok(Formatter {
            dialect: *dialect,
            parser,
            config,
        })
    }

    /// Access the current configuration.
    pub fn config(&self) -> &FormatConfig {
        &self.config
    }

    /// Format SQL source text. Handles multiple statements and preserves comments.
    pub fn format(&mut self, source: &str) -> Result<String, crate::parser::ParseError> {
        let mut cursor = self.parser.parse(source);

        let mut roots = Vec::new();
        while let Some(result) = cursor.next_statement() {
            roots.push(result?);
        }

        let base = cursor.base();
        let comments = base.comments().to_vec();
        let tokens = base.tokens();
        Ok(format_stmts(
            self.dialect,
            &self.config,
            self.config.semicolons,
            base,
            &roots,
            &comments,
            tokens,
            source,
        ))
    }

    /// Format a single pre-parsed AST node. This is the low-level entry point
    /// for cases where the caller controls parsing (e.g. macro expansion).
    pub fn format_node<'a>(&self, cursor: &'a CursorBase<'a>, node_id: NodeId) -> String {
        let mut arena = DocArena::new();
        let tokens = cursor.tokens();
        let source = cursor.source();
        let ctx = CommentCtx::new(&[], tokens, source);
        let doc = format_node(self.dialect, cursor, node_id, &mut arena, Some(&ctx));
        render(&arena, doc, &self.config)
    }
}

// ── Multi-statement formatting ──────────────────────────────────────────

fn format_stmts<'a>(
    dialect: Dialect<'_>,
    config: &FormatConfig,
    semicolons: bool,
    cursor: &'a CursorBase<'a>,
    roots: &[NodeId],
    comments: &[Comment],
    tokens: &[TokenPos],
    source: &str,
) -> String {
    if roots.is_empty() {
        return String::new();
    }

    let mut arena = DocArena::new();

    let comment_ctx = if comments.is_empty() {
        None
    } else {
        Some(CommentCtx::new(comments, tokens, source))
    };
    let comment_ctx_ref = comment_ctx.as_ref();

    let mut parts: Vec<DocId> = Vec::new();
    for (i, &root_id) in roots.iter().enumerate() {
        if let Some(ctx) = &comment_ctx {
            if let Some((next_offset, _)) = ctx.peek_next_token() {
                if i == 0 {
                    // Drain comments before the first statement (e.g. file header).
                    drain_gap_comments(ctx, next_offset, source, &mut arena, &mut parts);
                } else {
                    // Emit statement separator.
                    if semicolons {
                        parts.push(arena.text(";"));
                    }
                    // Drain trailing comments (same line as end of previous stmt).
                    drain_trailing_gap(ctx, next_offset, source, &mut arena, &mut parts);
                    // Statement separator blank line.
                    parts.push(arena.hardline());
                    parts.push(arena.hardline());
                    // Drain leading comments (on their own lines before next stmt).
                    drain_gap_comments(ctx, next_offset, source, &mut arena, &mut parts);
                }
            } else if i > 0 {
                if semicolons {
                    parts.push(arena.text(";"));
                }
                parts.push(arena.hardline());
                parts.push(arena.hardline());
            }
        } else if i > 0 {
            if semicolons {
                parts.push(arena.text(";"));
            }
            parts.push(arena.hardline());
            parts.push(arena.hardline());
        }

        parts.push(format_node(
            dialect,
            cursor,
            root_id,
            &mut arena,
            comment_ctx_ref,
        ));
    }

    if let Some(ctx) = &comment_ctx {
        let trailing = ctx.drain_remaining(&mut arena);
        parts.push(trailing);
    }

    let doc = arena.cats(&parts);
    let mut out = render(&arena, doc, config);

    if semicolons {
        out.push(';');
    }
    out.push('\n');
    out
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
    while let Some(c) = ctx.peek_comment() {
        if c.offset >= before {
            break;
        }
        let gap_start = (ctx.last_source_end() as usize).min(source.len());
        let gap_end = (c.offset as usize).min(source.len());
        if gap_start < gap_end && source[gap_start..gap_end].contains('\n') {
            break; // This comment is on a new line — it's leading, not trailing.
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
        ctx.set_source_end(c.offset + c.length);
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
        ctx.set_source_end(c.offset + c.length);
        ctx.advance_comment();
    }
}

// ── Single-node formatting ──────────────────────────────────────────────

fn format_node<'a>(
    dialect: Dialect<'a>,
    cursor: &'a CursorBase<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    comment_ctx: Option<&'a CommentCtx<'a>>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(dialect, cursor, node_id, arena, comment_ctx, &consumed)
}

fn format_node_inner<'a>(
    dialect: Dialect<'a>,
    cursor: &'a CursorBase<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    comment_ctx: Option<&'a CommentCtx<'a>>,
    consumed_regions: &Cell<u64>,
) -> DocId {
    if node_id.is_null() {
        return NIL_DOC;
    }

    let Some((ptr, tag)) = cursor.node_ptr(node_id) else {
        return NIL_DOC;
    };
    let Some((ops_bytes, ops_len)) = dialect.fmt_dispatch(tag) else {
        return NIL_DOC;
    };
    let source = cursor.source();
    let macro_regions = cursor.macro_regions();

    let format_child = move |id: NodeId, arena: &mut DocArena<'a>| {
        if !macro_regions.is_empty() {
            if let Some(verbatim) = try_macro_verbatim(
                comment_ctx,
                macro_regions,
                source,
                arena,
                consumed_regions,
            ) {
                return verbatim;
            }
        }
        format_node_inner(dialect, cursor, id, arena, comment_ctx, consumed_regions)
    };
    let resolve_list = move |id: NodeId| -> Vec<NodeId> {
        cursor
            .list_children(id, &dialect)
            .map(|c| c.to_vec())
            .unwrap_or_default()
    };
    let children = cursor.list_children(node_id, &dialect);

    let fields = extract_fields(&dialect, ptr, tag, source);

    let comments = comment_ctx.map(|ctx| CommentInfo { ctx });

    Interpreter::new(
        dialect,
        ops_bytes,
        ops_len,
        &format_child,
        &resolve_list,
        comments,
    )
    .run(&fields, children, arena)
}

/// Check if the next token falls within a macro region. If so, emit the
/// entire macro call verbatim and advance the token cursor past it.
fn try_macro_verbatim<'a>(
    comment_ctx: Option<&CommentCtx<'a>>,
    regions: &[MacroRegion],
    source: &'a str,
    arena: &mut DocArena<'a>,
    consumed: &Cell<u64>,
) -> Option<DocId> {
    let ctx = comment_ctx?;
    let (tok_offset, _) = ctx.peek_next_token()?;

    for (i, r) in regions.iter().enumerate() {
        if i >= 64 {
            break;
        }
        let r_start = r.call_offset;
        let r_end = r_start + r.call_length;

        if tok_offset >= r_start && tok_offset < r_end {
            let bit = 1u64 << i;
            let bits = consumed.get();
            if bits & bit != 0 {
                // Already consumed — this is another child within the same
                // macro region. Emit NIL and advance past its tokens.
                ctx.advance_past(r_end);
                return Some(NIL_DOC);
            }
            consumed.set(bits | bit);
            ctx.advance_past(r_end);
            ctx.set_source_end(r_end);
            return Some(arena.text(&source[r_start as usize..r_end as usize]));
        }
    }
    None
}

// ── Field extraction ────────────────────────────────────────────────────

/// Extract typed field values from a raw node pointer.
fn extract_fields<'a>(
    dialect: &Dialect<'_>,
    ptr: *const u8,
    tag: u32,
    source: &'a str,
) -> Fields<'a> {
    let meta = dialect.field_meta(tag);
    let mut fields = Fields::new();
    for m in meta {
        fields.push(unsafe { extract_one(ptr, m, source) });
    }
    fields
}

/// # Safety
/// `ptr` must point to a valid node struct whose field at `m.offset` has
/// the type indicated by `m.kind`.
unsafe fn extract_one<'a>(ptr: *const u8, m: &FieldMeta, source: &'a str) -> FieldVal<'a> {
    unsafe {
        let field_ptr = ptr.add(m.offset as usize);
        match m.kind {
            FIELD_NODE_ID => FieldVal::NodeId(NodeId(*(field_ptr as *const u32))),
            FIELD_SPAN => {
                let span = &*(field_ptr as *const SourceSpan);
                if span.length == 0 {
                    FieldVal::Span("", 0)
                } else {
                    FieldVal::Span(span.as_str(source), span.offset)
                }
            }
            FIELD_BOOL => FieldVal::Bool(*(field_ptr as *const u32) != 0),
            FIELD_FLAGS => FieldVal::Flags(*(field_ptr as *const u8)),
            FIELD_ENUM => FieldVal::Enum(*(field_ptr as *const u32)),
            _ => panic!("unknown C field kind: {}", m.kind),
        }
    }
}

