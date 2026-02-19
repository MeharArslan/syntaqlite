use std::cell::Cell;

use crate::dialect::Dialect;
use crate::parser::{MacroRegion, NodeId, NodeList, Parser, Session, Trivia, TriviaKind};

use super::FormatConfig;
use super::doc::{DocArena, DocId, NIL_DOC};
use super::interpret::Interpreter;
use super::render::render;
use super::trivia::TriviaCtx;

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
        if dialect.raw.fmt_strings.is_null() || dialect.raw.fmt_string_count == 0 {
            return Err("C dialect has no fmt data");
        }
        let mut parser = Parser::new(dialect);
        parser.set_collect_tokens(true);
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
        let mut session = self.parser.parse(source);

        let mut roots = Vec::new();
        while let Some(result) = session.next_statement() {
            roots.push(result?);
        }

        let trivia = session.trivia();
        Ok(format_stmts(
            self.dialect,
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
        let doc = format_node(self.dialect, session, node_id, &mut arena);
        render(&arena, doc, &self.config)
    }
}

// ── Multi-statement formatting ──────────────────────────────────────────

fn format_stmts<'a>(
    dialect: Dialect<'_>,
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
            dialect.first_source_offset(session, root_id).unwrap_or(source.len() as u32);

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
            dialect
                .first_source_offset(session, roots[i + 1])
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
            let doc = format_node(dialect, session, root_id, &mut arena);
            out.push_str(&render(&arena, doc, config));
        } else {
            let trivia_ctx = TriviaCtx::new(within_trivia, source);
            let doc =
                format_node_with_trivia(dialect, session, root_id, &mut arena, &trivia_ctx);
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
    dialect: Dialect<'a>,
    session: &'a Session<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(dialect, session, node_id, arena, None, &consumed)
}

fn format_node_with_trivia<'a>(
    dialect: Dialect<'a>,
    session: &'a Session<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
    trivia_ctx: &'a TriviaCtx<'a>,
) -> DocId {
    let consumed = Cell::new(0u64);
    format_node_inner(
        dialect,
        session,
        node_id,
        arena,
        Some(trivia_ctx),
        &consumed,
    )
}

fn format_node_inner<'a>(
    dialect: Dialect<'a>,
    session: &'a Session<'a>,
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
    let Some((ops_bytes, ops_len)) = dialect.fmt_dispatch(tag) else {
        return NIL_DOC;
    };
    let source = session.source();
    let macro_regions = session.macro_regions();

    let format_child = move |id: NodeId, arena: &mut DocArena<'a>| {
        if !macro_regions.is_empty() {
            if let Some(verbatim) = try_macro_verbatim(
                dialect,
                session,
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
            session,
            id,
            arena,
            trivia_ctx,
            consumed_regions,
        )
    };
    let resolve_list = move |id: NodeId| -> Vec<NodeId> {
        session
            .node_ptr(id)
            .filter(|&(_, t)| dialect.is_list(t))
            .map(|(p, _)| {
                let list = unsafe { &*(p as *const NodeList) };
                list.children().to_vec()
            })
            .unwrap_or_default()
    };
    let children = if dialect.is_list(tag) {
        let list = unsafe { &*(ptr as *const NodeList) };
        Some(list.children() as &[NodeId])
    } else {
        None
    };

    let fields = dialect.extract_fields(ptr, tag, source);

    let source_offset_fn =
        move |id: NodeId| -> Option<u32> { dialect.first_source_offset(session, id) };
    let source_offset: Option<&dyn Fn(NodeId) -> Option<u32>> =
        if trivia_ctx.is_some() { Some(&source_offset_fn) } else { None };

    Interpreter::new(
        dialect,
        ops_bytes,
        ops_len,
        &format_child,
        &resolve_list,
        trivia_ctx,
        source_offset,
    )
    .run(&fields, children, arena)
}

/// Check if a node's subtree overlaps with any macro region.
fn try_macro_verbatim<'a>(
    dialect: Dialect<'_>,
    session: &'a Session<'a>,
    node_id: NodeId,
    regions: &[MacroRegion],
    arena: &mut DocArena<'a>,
    consumed: &Cell<u64>,
) -> Option<DocId> {
    let first = dialect.first_source_offset(session, node_id)?;
    let last = dialect.last_source_offset(session, node_id)?;

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
