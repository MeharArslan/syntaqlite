use super::{FormatConfig, KeywordCase};
use super::doc::{Doc, DocArena, DocId, NIL_DOC};

/// Render a document tree to a string using the Lindig strict algorithm.
pub fn render(arena: &DocArena, root: DocId, config: &FormatConfig) -> String {
    if root == NIL_DOC {
        return String::new();
    }

    let mut out = String::new();
    let mut pos: usize = 0; // current column
    // Stack of (indent, mode, doc_id) — processed top-down
    let mut stack: Vec<(i32, Mode, DocId)> = vec![(0, Mode::Break, root)];
    let mut line_suffix_buf: Vec<(i32, Mode, DocId)> = Vec::new();

    while let Some((indent, mode, doc_id)) = stack.pop() {
        if doc_id == NIL_DOC {
            continue;
        }

        match arena.get(doc_id) {
            Doc::Text(s) => {
                out.push_str(s);
                pos += s.len();
            }

            Doc::Keyword(s) => {
                push_keyword(s, config, &mut out);
                pos += s.len();
            }

            Doc::Line => match mode {
                Mode::Flat => {
                    out.push(' ');
                    pos += 1;
                }
                Mode::Break => {
                    flush_line_suffixes(arena, &mut line_suffix_buf, config, &mut out, &mut pos);
                    out.push('\n');
                    let spaces = indent.max(0) as usize;
                    for _ in 0..spaces {
                        out.push(' ');
                    }
                    pos = spaces;
                }
            },

            Doc::SoftLine => match mode {
                Mode::Flat => {
                    // empty in flat mode
                }
                Mode::Break => {
                    flush_line_suffixes(arena, &mut line_suffix_buf, config, &mut out, &mut pos);
                    out.push('\n');
                    let spaces = indent.max(0) as usize;
                    for _ in 0..spaces {
                        out.push(' ');
                    }
                    pos = spaces;
                }
            },

            Doc::HardLine => {
                flush_line_suffixes(arena, &mut line_suffix_buf, config, &mut out, &mut pos);
                out.push('\n');
                let spaces = indent.max(0) as usize;
                for _ in 0..spaces {
                    out.push(' ');
                }
                pos = spaces;
            }

            Doc::Cat { left, right } => {
                // Push right first so left is processed first (stack is LIFO)
                stack.push((indent, mode, *right));
                stack.push((indent, mode, *left));
            }

            Doc::Nest { indent: di, child } => {
                stack.push((indent + *di as i32, mode, *child));
            }

            Doc::Group { child } => {
                if fits(arena, *child, indent, config.line_width as i32 - pos as i32) {
                    stack.push((indent, Mode::Flat, *child));
                } else {
                    stack.push((indent, Mode::Break, *child));
                }
            }

            Doc::LineSuffix { child } => {
                line_suffix_buf.push((indent, mode, *child));
            }

            Doc::BreakParent => {
                // BreakParent is a signal consumed by fits() — at render time
                // in break mode it's a no-op (the group already broke).
            }
        }
    }

    // Flush any remaining line suffixes at end of output
    flush_line_suffixes(arena, &mut line_suffix_buf, config, &mut out, &mut pos);

    out
}

// ── Private types ─────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Flat,
    Break,
}

// ── Private helpers ───────────────────────────────────────────────────

/// Render buffered line suffixes directly to output (before the next newline).
fn flush_line_suffixes(
    arena: &DocArena,
    buf: &mut Vec<(i32, Mode, DocId)>,
    config: &FormatConfig,
    out: &mut String,
    pos: &mut usize,
) {
    if buf.is_empty() {
        return;
    }
    for (indent, mode, doc_id) in buf.drain(..) {
        render_inline(arena, doc_id, indent, mode, config, out, pos);
    }
}

/// Render a doc tree directly into the output string (used for line suffixes).
fn render_inline(
    arena: &DocArena,
    doc_id: DocId,
    indent: i32,
    _mode: Mode,
    config: &FormatConfig,
    out: &mut String,
    pos: &mut usize,
) {
    if doc_id == NIL_DOC {
        return;
    }
    let mut stack: Vec<(i32, DocId)> = vec![(indent, doc_id)];
    while let Some((indent, doc_id)) = stack.pop() {
        if doc_id == NIL_DOC {
            continue;
        }
        match arena.get(doc_id) {
            Doc::Text(s) => {
                out.push_str(s);
                *pos += s.len();
            }
            Doc::Keyword(s) => {
                push_keyword(s, config, out);
                *pos += s.len();
            }
            Doc::Cat { left, right } => {
                stack.push((indent, *right));
                stack.push((indent, *left));
            }
            Doc::Nest { indent: di, child } => {
                stack.push((indent + *di as i32, *child));
            }
            _ => {}
        }
    }
}

/// Push a keyword string with the appropriate casing to the output.
fn push_keyword(s: &str, config: &FormatConfig, out: &mut String) {
    match config.keyword_case {
        KeywordCase::Preserve => out.push_str(s),
        KeywordCase::Upper => {
            for c in s.chars() {
                for u in c.to_uppercase() {
                    out.push(u);
                }
            }
        }
        KeywordCase::Lower => {
            for c in s.chars() {
                for l in c.to_lowercase() {
                    out.push(l);
                }
            }
        }
    }
}

/// Check whether a document fits within `remaining` columns when rendered flat.
fn fits(arena: &DocArena, doc_id: DocId, indent: i32, remaining: i32) -> bool {
    if remaining < 0 {
        return false;
    }

    let mut remaining = remaining;
    let mut stack: Vec<(i32, DocId)> = vec![(indent, doc_id)];

    while let Some((indent, doc_id)) = stack.pop() {
        if remaining < 0 {
            return false;
        }
        if doc_id == NIL_DOC {
            continue;
        }

        match arena.get(doc_id) {
            Doc::Text(s) | Doc::Keyword(s) => {
                remaining -= s.len() as i32;
            }
            Doc::Line => {
                // In flat mode, Line becomes a space
                remaining -= 1;
            }
            Doc::SoftLine => {
                // In flat mode, SoftLine is empty
            }
            Doc::HardLine => {
                // HardLine always breaks — doesn't fit in flat mode
                return false;
            }
            Doc::Cat { left, right } => {
                stack.push((indent, *right));
                stack.push((indent, *left));
            }
            Doc::Nest { indent: di, child } => {
                stack.push((indent + *di as i32, *child));
            }
            Doc::Group { child } => {
                // In flat mode, groups are transparent
                stack.push((indent, *child));
            }
            Doc::LineSuffix { .. } => {
                // Line suffixes don't contribute to width in flat mode
            }
            Doc::BreakParent => {
                // BreakParent forces the enclosing group to break
                return false;
            }
        }
    }

    remaining >= 0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> FormatConfig {
        FormatConfig::default()
    }

    fn narrow_config(width: usize) -> FormatConfig {
        FormatConfig {
            line_width: width,
            ..Default::default()
        }
    }

    #[test]
    fn plain_text() {
        let mut arena = DocArena::new();
        let doc = arena.text("hello world");
        assert_eq!(render(&arena, doc, &default_config()), "hello world");
    }

    #[test]
    fn cat_two_texts() {
        let mut arena = DocArena::new();
        let a = arena.text("hello");
        let b = arena.text(" world");
        let doc = arena.cat(a, b);
        assert_eq!(render(&arena, doc, &default_config()), "hello world");
    }

    #[test]
    fn group_fits_flat() {
        let mut arena = DocArena::new();
        let a = arena.text("a");
        let sp = arena.line();
        let b = arena.text("b");
        let inner = arena.cats(&[a, sp, b]);
        let doc = arena.group(inner);
        assert_eq!(render(&arena, doc, &default_config()), "a b");
    }

    #[test]
    fn group_breaks() {
        let mut arena = DocArena::new();
        let a = arena.text("aaaa");
        let sp = arena.line();
        let b = arena.text("bbbb");
        let inner = arena.cats(&[a, sp, b]);
        let doc = arena.group(inner);
        // Width 6: "aaaa bbbb" is 9 chars, doesn't fit
        assert_eq!(render(&arena, doc, &narrow_config(6)), "aaaa\nbbbb");
    }

    #[test]
    fn nest_indentation() {
        let mut arena = DocArena::new();
        let a = arena.text("a");
        let sp = arena.line();
        let b = arena.text("b");
        let inner = arena.cats(&[a, sp, b]);
        let nested = arena.nest(4, inner);
        let doc = arena.group(nested);
        // Width 3: "a b" is 3 chars, fits
        assert_eq!(render(&arena, doc, &narrow_config(3)), "a b");
        // Width 2: doesn't fit, breaks
        assert_eq!(render(&arena, doc, &narrow_config(2)), "a\n    b");
    }

    #[test]
    fn nil_doc_renders_empty() {
        let arena = DocArena::new();
        assert_eq!(render(&arena, NIL_DOC, &default_config()), "");
    }
}
