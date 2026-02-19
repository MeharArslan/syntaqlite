use crate::dialect::Dialect;
use crate::parser::{NodeId, Parser, Session, Trivia, TriviaKind};

use super::bytecode::LoadedFmt;
use super::config::FormatConfig;
use super::doc::DocArena;
use super::format::{
    first_source_offset, format_node as format_node_inner,
    format_node_with_trivia, NodeMeta,
};
use super::render::render;
use super::trivia::TriviaCtx;

/// High-level SQL formatter. Created from a `Dialect`, reusable across inputs.
pub struct Formatter {
    fmt: LoadedFmt,
    meta: NodeMeta,
    parser: Parser,
    config: FormatConfig,
    /// Append semicolons after each statement.
    pub semicolons: bool,
}

impl Formatter {
    /// Create a formatter for the given dialect with default configuration.
    pub fn new(dialect: &Dialect) -> Result<Self, &'static str> {
        let fmt = LoadedFmt::from_dialect(dialect)?;
        let meta = NodeMeta::from_dialect(dialect);
        let mut parser = Parser::new(dialect);
        parser.set_collect_tokens(true);
        Ok(Formatter {
            fmt,
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
            &self.fmt,
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
        let doc = format_node_inner(&self.fmt, session, &self.meta, node_id, &mut arena);
        render(&arena, doc, &self.config)
    }
}

fn format_stmts<'a>(
    fmt: &'a LoadedFmt,
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
            first_source_offset(session, meta, root_id).unwrap_or(source.len() as u32);

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
            first_source_offset(session, meta, roots[i + 1]).unwrap_or(source.len() as u32)
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
            let doc = format_node_inner(fmt, session, meta, root_id, &mut arena);
            out.push_str(&render(&arena, doc, config));
        } else {
            let trivia_ctx = TriviaCtx::new(within_trivia, source);
            let doc =
                format_node_with_trivia(fmt, session, meta, root_id, &mut arena, &trivia_ctx);
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
