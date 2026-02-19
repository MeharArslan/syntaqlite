// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::cell::Cell;

use crate::dialect::Dialect;
use crate::dialect::ffi::{FieldMeta, FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN};
use crate::parser::{Comment, CommentKind, CursorBase, FieldVal, Fields, MacroRegion, NodeId, Parser, ParserConfig, SourceSpan};

use super::FormatConfig;
use super::doc::{DocArena, DocId, NIL_DOC};
use super::interpret::Interpreter;
use super::render::render;
use super::comment::CommentCtx;

// ── Formatter ───────────────────────────────────────────────────────────

/// High-level SQL formatter. Created from a `Dialect`, reusable across inputs.
pub struct Formatter<'d> {
    dialect: Dialect<'d>,
    parser: Parser,
    config: FormatConfig,
    /// Append semicolons after each statement.
    pub semicolons: bool,
}

// SAFETY: Dialect is Send+Sync, Parser is Send.
unsafe impl Send for Formatter<'_> {}

impl<'d> Formatter<'d> {
    /// Create a formatter for the given dialect with default configuration.
    pub fn new(dialect: &Dialect<'d>) -> Result<Self, &'static str> {
        if !dialect.has_fmt_data() {
            return Err("C dialect has no fmt data");
        }
        let config = ParserConfig { collect_tokens: true, ..Default::default() };
        let parser = Parser::with_config(dialect, &config);
        Ok(Formatter {
            dialect: *dialect,
            parser,
            config: FormatConfig::default(),
            semicolons: false,
        })
    }

    /// Set the format configuration.
    pub fn with_config(mut self, config: FormatConfig) -> Self {
        self.config = config;
        self
    }

    /// Set whether to append semicolons after each statement.
    pub fn with_semicolons(mut self, semicolons: bool) -> Self {
        self.semicolons = semicolons;
        self
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
        Ok(format_stmts(
            self.dialect,
            &self.config,
            self.semicolons,
            base,
            &roots,
            &comments,
            source,
        ))
    }

    /// Format a single pre-parsed AST node. This is the low-level entry point
    /// for cases where the caller controls parsing (e.g. macro expansion).
    pub fn format_node(&self, cursor: &CursorBase<'_>, node_id: NodeId) -> String {
        let mut arena = DocArena::new();
        let doc = format_node(self.dialect, cursor, node_id, &mut arena);
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
    source: &str,
) -> String {
    let mut out = String::new();
    let mut comment_cursor = 0;

    for (i, &root_id) in roots.iter().enumerate() {
        if i > 0 {
            if semicolons {
                out.push(';');
            }
            out.push_str("\n\n");
        }

        let stmt_start =
            first_source_offset(&dialect, cursor, root_id).unwrap_or(source.len() as u32);

        // Emit leading comments before this statement.
        while comment_cursor < comments.len() && comments[comment_cursor].offset < stmt_start {
            let t = &comments[comment_cursor];
            let text = &source[t.offset as usize..(t.offset + t.length) as usize];
            match t.kind {
                CommentKind::LineComment => {
                    out.push_str(text);
                    out.push('\n');
                }
                CommentKind::BlockComment => {
                    out.push_str(text);
                    out.push(' ');
                }
            }
            comment_cursor += 1;
        }

        // Collect comments within this statement's span.
        let stmt_end = if i + 1 < roots.len() {
            first_source_offset(&dialect, cursor, roots[i + 1])
                .unwrap_or(source.len() as u32)
        } else {
            source.len() as u32
        };

        let within_start = comment_cursor;
        while comment_cursor < comments.len() && comments[comment_cursor].offset < stmt_end {
            comment_cursor += 1;
        }
        let within_comments = &comments[within_start..comment_cursor];

        // Format the statement, interleaving any within-statement comments.
        let mut arena = DocArena::new();
        if within_comments.is_empty() {
            let doc = format_node(dialect, cursor, root_id, &mut arena);
            out.push_str(&render(&arena, doc, config));
        } else {
            let comment_ctx = CommentCtx::new(within_comments, source);
            let doc =
                format_node_with_comments(dialect, cursor, root_id, &mut arena, &comment_ctx);
            let trailing = comment_ctx.drain_remaining(&mut arena);
            let final_doc = arena.cat(doc, trailing);
            out.push_str(&render(&arena, final_doc, config));
        }
    }

    // Emit trailing comments after the last statement.
    while comment_cursor < comments.len() {
        let t = &comments[comment_cursor];
        let text = &source[t.offset as usize..(t.offset + t.length) as usize];
        match t.kind {
            CommentKind::LineComment => {
                out.push_str(text);
                out.push('\n');
            }
            CommentKind::BlockComment => {
                out.push_str(text);
            }
        }
        comment_cursor += 1;
    }

    if !roots.is_empty() {
        if semicolons {
            out.push(';');
        }
        out.push('\n');
    }
    out
}

// ── Single-node formatting ──────────────────────────────────────────────

fn format_node<'a>(
    dialect: Dialect<'a>,
    cursor: &'a CursorBase<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(dialect, cursor, node_id, arena, None, &consumed)
}

fn format_node_with_comments<'a>(
    dialect: Dialect<'a>,
    cursor: &'a CursorBase<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    comment_ctx: &'a CommentCtx<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(
        dialect,
        cursor,
        node_id,
        arena,
        Some(comment_ctx),
        &consumed,
    )
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
                dialect,
                cursor,
                id,
                macro_regions,
                arena,
                consumed_regions,
            ) {
                return verbatim;
            }
        }
        format_node_inner(
            dialect,
            cursor,
            id,
            arena,
            comment_ctx,
            consumed_regions,
        )
    };
    let resolve_list = move |id: NodeId| -> Vec<NodeId> {
        cursor
            .list_children(id, &dialect)
            .map(|c| c.to_vec())
            .unwrap_or_default()
    };
    let children = cursor.list_children(node_id, &dialect);

    let fields = extract_fields(&dialect, ptr, tag, source);

    let source_offset_fn =
        move |id: NodeId| -> Option<u32> { first_source_offset(&dialect, cursor, id) };
    let source_offset: Option<&dyn Fn(NodeId) -> Option<u32>> =
        if comment_ctx.is_some() { Some(&source_offset_fn) } else { None };

    Interpreter::new(
        dialect,
        ops_bytes,
        ops_len,
        &format_child,
        &resolve_list,
        comment_ctx,
        source_offset,
    )
    .run(&fields, children, arena)
}

/// Check if a node's subtree overlaps with any macro region.
fn try_macro_verbatim<'a>(
    dialect: Dialect<'_>,
    cursor: &'a CursorBase<'a>,
    node_id: NodeId,
    regions: &[MacroRegion],
    arena: &mut DocArena<'a>,
    consumed: &Cell<u64>,
) -> Option<DocId> {
    let first = first_source_offset(&dialect, cursor, node_id)?;
    let last = last_source_offset(&dialect, cursor, node_id)?;

    for (i, r) in regions.iter().enumerate() {
        if i >= 64 {
            break;
        }
        let r_start = r.call_offset;
        let r_end = r_start + r.call_length;

        if first < r_end && last > r_start {
            let bit = 1u64 << i;
            let bits = consumed.get();
            if bits & bit != 0 {
                return Some(NIL_DOC);
            }
            consumed.set(bits | bit);
            let source = cursor.source();
            let verbatim_start = first.min(r_start) as usize;
            let verbatim_end = last.max(r_end) as usize;
            return Some(arena.text(&source[verbatim_start..verbatim_end]));
        }
    }
    None
}

// ── Field extraction ────────────────────────────────────────────────────

/// Extract typed field values from a raw node pointer.
fn extract_fields<'a>(dialect: &Dialect<'_>, ptr: *const u8, tag: u32, source: &'a str) -> Fields<'a> {
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

/// Find the earliest source offset in a node's subtree.
fn first_source_offset(
    dialect: &Dialect<'_>,
    cursor: &CursorBase<'_>,
    node_id: NodeId,
) -> Option<u32> {
    if node_id.is_null() {
        return None;
    }

    if let Some(children) = cursor.list_children(node_id, dialect) {
        return if children.is_empty() {
            None
        } else {
            first_source_offset(dialect, cursor, children[0])
        };
    }

    let (ptr, tag) = cursor.node_ptr(node_id)?;
    let source = cursor.source();
    let fields = extract_fields(dialect, ptr, tag, source);

    let mut min: Option<u32> = None;
    for field in fields.iter() {
        if let FieldVal::Span(s, offset) = field {
            if !s.is_empty() {
                min = Some(match min {
                    Some(prev) => prev.min(*offset),
                    None => *offset,
                });
            }
        }
    }
    if min.is_some() {
        return min;
    }

    for field in fields.iter() {
        if let FieldVal::NodeId(child_id) = field {
            if let Some(off) = first_source_offset(dialect, cursor, *child_id) {
                return Some(off);
            }
        }
    }

    None
}

/// Find the latest source offset (end) in a node's subtree.
fn last_source_offset(
    dialect: &Dialect<'_>,
    cursor: &CursorBase<'_>,
    node_id: NodeId,
) -> Option<u32> {
    if node_id.is_null() {
        return None;
    }

    if let Some(children) = cursor.list_children(node_id, dialect) {
        return if children.is_empty() {
            None
        } else {
            last_source_offset(dialect, cursor, children[children.len() - 1])
        };
    }

    let (ptr, tag) = cursor.node_ptr(node_id)?;
    let source = cursor.source();
    let fields = extract_fields(dialect, ptr, tag, source);

    let mut max: Option<u32> = None;
    for field in fields.iter() {
        if let FieldVal::Span(s, offset) = field {
            if !s.is_empty() {
                let end = *offset + s.len() as u32;
                max = Some(match max {
                    Some(prev) => prev.max(end),
                    None => end,
                });
            }
        }
    }
    if max.is_some() {
        for field in fields.iter() {
            if let FieldVal::NodeId(child_id) = field {
                if let Some(end) = last_source_offset(dialect, cursor, *child_id) {
                    max = Some(max.unwrap().max(end));
                }
            }
        }
        return max;
    }

    for field in fields.iter() {
        if let FieldVal::NodeId(child_id) = field {
            if let Some(end) = last_source_offset(dialect, cursor, *child_id) {
                max = Some(match max {
                    Some(prev) => prev.max(end),
                    None => end,
                });
            }
        }
    }

    max
}