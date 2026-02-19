use std::cell::Cell;

use crate::dialect::Dialect;
use crate::dialect::ffi::{
    FieldMeta, SyntaqliteDialect, FIELD_BOOL, FIELD_ENUM, FIELD_FLAGS, FIELD_NODE_ID, FIELD_SPAN,
};
use crate::parser::{
    FieldVal, Fields, MacroRegion, NodeId, NodeList, Parser, Session, SourceSpan, Trivia,
    TriviaKind,
};

use super::FormatConfig;
use super::doc::{DocArena, DocId, NIL_DOC};
use super::interpret::Interpreter;
use super::render::render;
use super::trivia::TriviaCtx;

// ── Formatter ───────────────────────────────────────────────────────────

/// High-level SQL formatter. Created from a `Dialect`, reusable across inputs.
pub struct Formatter {
    raw: *const SyntaqliteDialect,
    meta: NodeMeta,
    parser: Parser,
    config: FormatConfig,
    /// Append semicolons after each statement.
    pub semicolons: bool,
}

// SAFETY: raw points to a static C struct with no mutable state.
unsafe impl Send for Formatter {}
unsafe impl Sync for Formatter {}

impl Formatter {
    /// Create a formatter for the given dialect with default configuration.
    pub fn new(dialect: &Dialect) -> Result<Self, &'static str> {
        let raw = dialect.raw as *const SyntaqliteDialect;
        let d = unsafe { &*raw };
        if d.fmt_strings.is_null() || d.fmt_string_count == 0 {
            return Err("C dialect has no fmt data");
        }
        let meta = NodeMeta::from_dialect(dialect);
        let mut parser = Parser::new(dialect);
        parser.set_collect_tokens(true);
        Ok(Formatter {
            raw,
            meta,
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
        let mut session = self.parser.parse(source);

        let mut roots = Vec::new();
        while let Some(result) = session.next_statement() {
            roots.push(result?);
        }

        let trivia = session.trivia();
        Ok(format_stmts(
            self.raw,
            &self.meta,
            &self.config,
            self.semicolons,
            &session,
            &roots,
            trivia,
            source,
        ))
    }

    /// Format a single pre-parsed AST node. This is the low-level entry point
    /// for cases where the caller controls parsing (e.g. macro expansion).
    pub fn format_node(&self, session: &Session<'_>, node_id: NodeId) -> String {
        let mut arena = DocArena::new();
        let doc = format_node(self.raw, session, &self.meta, node_id, &mut arena);
        render(&arena, doc, &self.config)
    }
}

// ── Multi-statement formatting ──────────────────────────────────────────

fn format_stmts<'a>(
    raw: *const SyntaqliteDialect,
    meta: &'a NodeMeta,
    config: &FormatConfig,
    semicolons: bool,
    session: &'a Session<'a>,
    roots: &[NodeId],
    trivia: &[Trivia],
    source: &str,
) -> String {
    let mut out = String::new();
    let mut trivia_cursor = 0;

    for (i, &root_id) in roots.iter().enumerate() {
        if i > 0 {
            if semicolons {
                out.push(';');
            }
            out.push_str("\n\n");
        }

        let stmt_start =
            meta.first_source_offset(session, root_id).unwrap_or(source.len() as u32);

        // Emit leading trivia (comments before this statement).
        while trivia_cursor < trivia.len() && trivia[trivia_cursor].offset < stmt_start {
            let t = &trivia[trivia_cursor];
            let text = &source[t.offset as usize..(t.offset + t.length) as usize];
            match t.kind {
                TriviaKind::LineComment => {
                    out.push_str(text);
                    out.push('\n');
                }
                TriviaKind::BlockComment => {
                    out.push_str(text);
                    out.push(' ');
                }
            }
            trivia_cursor += 1;
        }

        // Collect trivia within this statement's span.
        let stmt_end = if i + 1 < roots.len() {
            meta.first_source_offset(session, roots[i + 1])
                .unwrap_or(source.len() as u32)
        } else {
            source.len() as u32
        };

        let within_start = trivia_cursor;
        while trivia_cursor < trivia.len() && trivia[trivia_cursor].offset < stmt_end {
            trivia_cursor += 1;
        }
        let within_trivia = &trivia[within_start..trivia_cursor];

        // Format the statement, interleaving any within-statement trivia.
        let mut arena = DocArena::new();
        if within_trivia.is_empty() {
            let doc = format_node(raw, session, meta, root_id, &mut arena);
            out.push_str(&render(&arena, doc, config));
        } else {
            let trivia_ctx = TriviaCtx::new(within_trivia, source);
            let doc =
                format_node_with_trivia(raw, session, meta, root_id, &mut arena, &trivia_ctx);
            let trailing = trivia_ctx.drain_remaining(&mut arena);
            let final_doc = arena.cat(doc, trailing);
            out.push_str(&render(&arena, final_doc, config));
        }
    }

    // Emit trailing trivia after the last statement.
    while trivia_cursor < trivia.len() {
        let t = &trivia[trivia_cursor];
        let text = &source[t.offset as usize..(t.offset + t.length) as usize];
        match t.kind {
            TriviaKind::LineComment => {
                out.push_str(text);
                out.push('\n');
            }
            TriviaKind::BlockComment => {
                out.push_str(text);
            }
        }
        trivia_cursor += 1;
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
    raw: *const SyntaqliteDialect,
    session: &'a Session<'a>,
    node_meta: &'a NodeMeta,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(raw, session, node_meta, node_id, arena, None, &consumed)
}

fn format_node_with_trivia<'a>(
    raw: *const SyntaqliteDialect,
    session: &'a Session<'a>,
    node_meta: &'a NodeMeta,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    trivia_ctx: &'a TriviaCtx<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(
        raw,
        session,
        node_meta,
        node_id,
        arena,
        Some(trivia_ctx),
        &consumed,
    )
}

fn format_node_inner<'a>(
    raw: *const SyntaqliteDialect,
    session: &'a Session<'a>,
    node_meta: &'a NodeMeta,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    trivia_ctx: Option<&'a TriviaCtx<'a>>,
    consumed_regions: &Cell<u64>,
) -> DocId {
    if node_id.is_null() {
        return NIL_DOC;
    }

    let Some((ptr, tag)) = session.node_ptr(node_id) else {
        return NIL_DOC;
    };
    let d = unsafe { &*raw };
    let Some((ops_base, ops_len)) = dispatch_ops(d, tag) else {
        return NIL_DOC;
    };
    let source = session.source();
    let macro_regions = session.macro_regions();

    let format_child = move |id: NodeId, arena: &mut DocArena<'a>| {
        if !macro_regions.is_empty() {
            if let Some(verbatim) = try_macro_verbatim(
                session,
                node_meta,
                id,
                macro_regions,
                arena,
                consumed_regions,
            ) {
                return verbatim;
            }
        }
        format_node_inner(
            raw,
            session,
            node_meta,
            id,
            arena,
            trivia_ctx,
            consumed_regions,
        )
    };
    let resolve_list = |id: NodeId| -> Vec<NodeId> {
        session
            .node_ptr(id)
            .filter(|&(_, t)| node_meta.is_list(t))
            .map(|(p, _)| {
                let list = unsafe { &*(p as *const NodeList) };
                list.children().to_vec()
            })
            .unwrap_or_default()
    };
    let children = if node_meta.is_list(tag) {
        let list = unsafe { &*(ptr as *const NodeList) };
        Some(list.children() as &[NodeId])
    } else {
        None
    };

    let fields = node_meta.extract_fields(ptr, tag, source);

    let source_offset_fn =
        |id: NodeId| -> Option<u32> { node_meta.first_source_offset(session, id) };
    let source_offset: Option<&dyn Fn(NodeId) -> Option<u32>> =
        if trivia_ctx.is_some() { Some(&source_offset_fn) } else { None };

    Interpreter::new(
        d.fmt_strings,
        d.fmt_string_count as usize,
        d.fmt_enum_display,
        d.fmt_enum_display_count as usize,
        ops_base,
        ops_len,
        &format_child,
        &resolve_list,
        trivia_ctx,
        source_offset,
    )
    .run(&fields, children, arena)
}

/// Look up the ops for a given node tag in the dispatch table.
fn dispatch_ops(d: &SyntaqliteDialect, tag: u32) -> Option<(*const u8, usize)> {
    let idx = tag as usize;
    if idx >= d.fmt_dispatch_count as usize {
        return None;
    }
    let packed = unsafe { *d.fmt_dispatch.add(idx) };
    let offset = (packed >> 16) as u16;
    let length = (packed & 0xFFFF) as u16;
    if offset == 0xFFFF {
        return None;
    }
    let base = unsafe { d.fmt_ops.add(offset as usize * 6) };
    Some((base, length as usize))
}

/// Check if a node's subtree overlaps with any macro region.
fn try_macro_verbatim<'a>(
    session: &'a Session<'a>,
    node_meta: &NodeMeta,
    node_id: NodeId,
    regions: &[MacroRegion],
    arena: &mut DocArena<'a>,
    consumed: &Cell<u64>,
) -> Option<DocId> {
    let first = node_meta.first_source_offset(session, node_id)?;
    let last = node_meta.last_source_offset(session, node_id)?;

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
            let source = session.source();
            let verbatim_start = first.min(r_start) as usize;
            let verbatim_end = last.max(r_end) as usize;
            return Some(arena.text(&source[verbatim_start..verbatim_end]));
        }
    }
    None
}

// ── NodeMeta ────────────────────────────────────────────────────────────

/// Zero-copy view over the C dialect's node metadata.
/// No allocations — reads directly from C static arrays.
pub(crate) struct NodeMeta {
    raw: *const SyntaqliteDialect,
}

impl NodeMeta {
    fn from_dialect(dialect: &Dialect) -> Self {
        NodeMeta {
            raw: dialect.raw as *const SyntaqliteDialect,
        }
    }

    fn d(&self) -> &SyntaqliteDialect {
        unsafe { &*self.raw }
    }

    fn is_list(&self, tag: u32) -> bool {
        let d = self.d();
        let idx = tag as usize;
        if idx >= d.node_count as usize {
            return false;
        }
        unsafe { *d.list_tags.add(idx) != 0 }
    }

    fn field_meta(&self, tag: u32) -> &[FieldMeta] {
        let d = self.d();
        let idx = tag as usize;
        if idx >= d.node_count as usize {
            return &[];
        }
        let count = unsafe { *d.field_meta_counts.add(idx) } as usize;
        let ptr = unsafe { *d.field_meta.add(idx) };
        if count == 0 || ptr.is_null() {
            return &[];
        }
        unsafe { std::slice::from_raw_parts(ptr, count) }
    }

    fn extract_fields<'a>(&self, ptr: *const u8, tag: u32, source: &'a str) -> Fields<'a> {
        let meta = self.field_meta(tag);
        let mut fields = Fields::new();
        for m in meta {
            fields.push(unsafe { extract_one(ptr, m, source) });
        }
        fields
    }

    fn first_source_offset(
        &self,
        session: &Session<'_>,
        node_id: NodeId,
    ) -> Option<u32> {
        if node_id.is_null() {
            return None;
        }
        let (ptr, tag) = session.node_ptr(node_id)?;

        if self.is_list(tag) {
            let list = unsafe { &*(ptr as *const NodeList) };
            let children = list.children();
            if !children.is_empty() {
                return self.first_source_offset(session, children[0]);
            }
            return None;
        }

        let source = session.source();
        let fields = self.extract_fields(ptr, tag, source);

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
                if let Some(off) = self.first_source_offset(session, *child_id) {
                    return Some(off);
                }
            }
        }

        None
    }

    fn last_source_offset(
        &self,
        session: &Session<'_>,
        node_id: NodeId,
    ) -> Option<u32> {
        if node_id.is_null() {
            return None;
        }
        let (ptr, tag) = session.node_ptr(node_id)?;

        if self.is_list(tag) {
            let list = unsafe { &*(ptr as *const NodeList) };
            let children = list.children();
            if !children.is_empty() {
                return self.last_source_offset(session, children[children.len() - 1]);
            }
            return None;
        }

        let source = session.source();
        let fields = self.extract_fields(ptr, tag, source);

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
                    if let Some(end) = self.last_source_offset(session, *child_id) {
                        max = Some(max.unwrap().max(end));
                    }
                }
            }
            return max;
        }

        for field in fields.iter() {
            if let FieldVal::NodeId(child_id) = field {
                if let Some(end) = self.last_source_offset(session, *child_id) {
                    max = Some(match max {
                        Some(prev) => prev.max(end),
                        None => end,
                    });
                }
            }
        }

        max
    }
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
