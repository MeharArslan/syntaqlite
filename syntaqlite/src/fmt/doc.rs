// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use super::KeywordCase;

/// A handle into the `DocArena`. `NIL_DOC` represents an empty/absent document.
pub type DocId = u32;

/// Sentinel value meaning "no document". Builder methods treat NIL_DOC operands
/// as identity elements (e.g. `cat(NIL_DOC, x) == x`).
pub const NIL_DOC: DocId = u32::MAX;

/// A node in the document algebra. Lifetime `'a` covers borrowed text slices.
#[derive(Debug, Clone)]
enum Doc<'a> {
    /// Source text (identifiers, literals). Never case-transformed.
    Text(&'a str),
    /// SQL keyword. Subject to `KeywordCase` at render time.
    Keyword(&'a str),
    /// Flat mode: space. Break mode: newline + indent.
    Line,
    /// Flat mode: empty. Break mode: newline + indent.
    SoftLine,
    /// Always newline + indent.
    HardLine,
    /// Concatenation of two documents.
    Cat { left: DocId, right: DocId },
    /// Increase indent by `indent` for `child`.
    Nest { indent: i16, child: DocId },
    /// Try to fit `child` on one line; break if it doesn't fit.
    Group { child: DocId },
    /// Defer `child` to end of current line (for trailing comments).
    LineSuffix { child: DocId },
    /// Force the enclosing group to break.
    BreakParent,
}

/// Arena-based storage for `Doc` nodes. Push-to-allocate, indexed by `DocId`.
#[derive(Debug)]
pub struct DocArena<'a> {
    docs: Vec<Doc<'a>>,
}

impl<'a> DocArena<'a> {
    pub fn new() -> Self {
        DocArena { docs: Vec::new() }
    }

    /// Create a new arena with pre-allocated capacity.
    pub fn with_capacity(cap: usize) -> Self {
        DocArena {
            docs: Vec::with_capacity(cap),
        }
    }

    /// Create a new arena, reusing the allocation from a previous arena.
    ///
    /// The old arena is consumed. The new arena has the same capacity but
    /// a fresh (possibly different) lifetime parameter.
    pub fn recycle<'b>(old: DocArena<'b>) -> Self {
        let mut docs = old.docs;
        docs.clear();
        // SAFETY: Vec is cleared so no Doc<'b> values remain. The empty Vec's
        // allocation is lifetime-independent (just a heap pointer + capacity).
        let docs: Vec<Doc<'a>> = unsafe {
            let (ptr, _, cap) = docs.into_raw_parts();
            Vec::from_raw_parts(ptr.cast(), 0, cap)
        };
        DocArena { docs }
    }

    fn push(&mut self, doc: Doc<'a>) -> DocId {
        let id = self.docs.len() as DocId;
        debug_assert!(id != NIL_DOC, "DocArena overflow");
        self.docs.push(doc);
        id
    }

    fn get(&self, id: DocId) -> &Doc<'a> {
        &self.docs[id as usize]
    }

    // -- Builder methods --

    pub fn text(&mut self, s: &'a str) -> DocId {
        self.push(Doc::Text(s))
    }

    pub fn keyword(&mut self, s: &'a str) -> DocId {
        self.push(Doc::Keyword(s))
    }

    pub fn line(&mut self) -> DocId {
        self.push(Doc::Line)
    }

    pub fn softline(&mut self) -> DocId {
        self.push(Doc::SoftLine)
    }

    pub fn hardline(&mut self) -> DocId {
        self.push(Doc::HardLine)
    }

    /// Concatenate two documents. If either operand is `NIL_DOC`, returns the other.
    pub fn cat(&mut self, left: DocId, right: DocId) -> DocId {
        if left == NIL_DOC {
            return right;
        }
        if right == NIL_DOC {
            return left;
        }
        self.push(Doc::Cat { left, right })
    }

    /// Concatenate a slice of documents left-to-right. Skips `NIL_DOC` entries.
    pub fn cats(&mut self, ids: &[DocId]) -> DocId {
        let mut result = NIL_DOC;
        for &id in ids {
            result = self.cat(result, id);
        }
        result
    }

    /// Nest (indent) `child` by `indent` spaces. Returns `NIL_DOC` if child is nil.
    pub fn nest(&mut self, indent: i16, child: DocId) -> DocId {
        if child == NIL_DOC {
            return NIL_DOC;
        }
        self.push(Doc::Nest { indent, child })
    }

    /// Group `child` — try flat first, break if it doesn't fit. Returns `NIL_DOC` if child is nil.
    pub fn group(&mut self, child: DocId) -> DocId {
        if child == NIL_DOC {
            return NIL_DOC;
        }
        self.push(Doc::Group { child })
    }

    pub fn line_suffix(&mut self, child: DocId) -> DocId {
        if child == NIL_DOC {
            return NIL_DOC;
        }
        self.push(Doc::LineSuffix { child })
    }

    pub fn break_parent(&mut self) -> DocId {
        self.push(Doc::BreakParent)
    }

    // -- Render --

    /// Render the document tree rooted at `root` to a string.
    /// Convenience wrapper around `render_into` that allocates fresh buffers.
    pub fn render(&self, root: DocId, line_width: usize, keyword_case: KeywordCase) -> String {
        let mut out = String::new();
        let mut stack = Vec::new();
        let mut fits_stack = Vec::new();
        let mut line_suffix_buf = Vec::new();
        self.render_into(
            root,
            line_width,
            keyword_case,
            &mut out,
            &mut stack,
            &mut fits_stack,
            &mut line_suffix_buf,
        );
        out
    }

    /// Render into caller-provided buffers, reusing their allocations.
    /// This avoids re-allocating the render stack, fits stack, and output
    /// string on every format call.
    pub fn render_into(
        &self,
        root: DocId,
        line_width: usize,
        keyword_case: KeywordCase,
        out: &mut String,
        stack: &mut Vec<(i32, Mode, DocId)>,
        fits_stack: &mut Vec<(i32, DocId)>,
        line_suffix_buf: &mut Vec<(i32, Mode, DocId)>,
    ) {
        if root == NIL_DOC {
            return;
        }

        out.reserve(self.docs.len() * 4);
        let mut pos: usize = 0;
        stack.push((0, Mode::Break, root));

        while let Some((indent, mode, doc_id)) = stack.pop() {
            if doc_id == NIL_DOC {
                continue;
            }

            match self.get(doc_id) {
                Doc::Text(s) => {
                    out.push_str(s);
                    pos += s.len();
                }

                Doc::Keyword(s) => {
                    push_keyword(s, keyword_case, out);
                    pos += s.len();
                }

                Doc::Line => match mode {
                    Mode::Flat => {
                        out.push(' ');
                        pos += 1;
                    }
                    Mode::Break => {
                        flush_line_suffixes(self, line_suffix_buf, keyword_case, out, &mut pos);
                        emit_newline(indent, out, &mut pos);
                    }
                },

                Doc::SoftLine => match mode {
                    Mode::Flat => {}
                    Mode::Break => {
                        flush_line_suffixes(self, line_suffix_buf, keyword_case, out, &mut pos);
                        emit_newline(indent, out, &mut pos);
                    }
                },

                Doc::HardLine => {
                    flush_line_suffixes(self, line_suffix_buf, keyword_case, out, &mut pos);
                    emit_newline(indent, out, &mut pos);
                }

                Doc::Cat { left, right } => {
                    stack.push((indent, mode, *right));
                    stack.push((indent, mode, *left));
                }

                Doc::Nest { indent: di, child } => {
                    stack.push((indent + *di as i32, mode, *child));
                }

                Doc::Group { child } => {
                    if self.fits_with(*child, indent, line_width as i32 - pos as i32, fits_stack) {
                        stack.push((indent, Mode::Flat, *child));
                    } else {
                        stack.push((indent, Mode::Break, *child));
                    }
                }

                Doc::LineSuffix { child } => {
                    line_suffix_buf.push((indent, mode, *child));
                }

                Doc::BreakParent => {}
            }
        }

        flush_line_suffixes(self, line_suffix_buf, keyword_case, out, &mut pos);
    }

    /// Check whether a document fits within `remaining` columns when rendered flat.
    /// Reuses `scratch` to avoid per-call allocation.
    fn fits_with(
        &self,
        doc_id: DocId,
        indent: i32,
        remaining: i32,
        scratch: &mut Vec<(i32, DocId)>,
    ) -> bool {
        if remaining < 0 {
            return false;
        }

        let mut remaining = remaining;
        scratch.clear();
        scratch.push((indent, doc_id));

        while let Some((indent, doc_id)) = scratch.pop() {
            if remaining < 0 {
                return false;
            }
            if doc_id == NIL_DOC {
                continue;
            }

            match self.get(doc_id) {
                Doc::Text(s) | Doc::Keyword(s) => {
                    remaining -= s.len() as i32;
                }
                Doc::Line => {
                    remaining -= 1;
                }
                Doc::SoftLine => {}
                Doc::HardLine => {
                    return false;
                }
                Doc::Cat { left, right } => {
                    scratch.push((indent, *right));
                    scratch.push((indent, *left));
                }
                Doc::Nest { indent: di, child } => {
                    scratch.push((indent + *di as i32, *child));
                }
                Doc::Group { child } => {
                    scratch.push((indent, *child));
                }
                Doc::LineSuffix { .. } => {}
                Doc::BreakParent => {
                    return false;
                }
            }
        }

        remaining >= 0
    }
}

// ── Private helpers ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Mode {
    Flat,
    Break,
}

/// Static buffer of spaces for indent emission (avoids per-char push).
const SPACES: &str = "                                                                                                                                ";

fn emit_newline(indent: i32, out: &mut String, pos: &mut usize) {
    out.push('\n');
    let spaces = indent.max(0) as usize;
    let mut remaining = spaces;
    while remaining > 0 {
        let chunk = remaining.min(SPACES.len());
        out.push_str(&SPACES[..chunk]);
        remaining -= chunk;
    }
    *pos = spaces;
}

/// Push a keyword string with the appropriate casing to the output.
///
/// Single-pass with direct pointer writes: reserves capacity once, then
/// writes transformed bytes into spare capacity with no per-byte bounds
/// checks. A single `set_len` at the end commits all bytes at once.
#[inline]
fn push_keyword(s: &str, case: KeywordCase, out: &mut String) {
    match case {
        KeywordCase::Preserve => out.push_str(s),
        KeywordCase::Upper | KeywordCase::Lower => {
            let src = s.as_bytes();
            let slen = src.len();
            out.reserve(slen);
            // SAFETY: we reserved `slen` bytes of spare capacity. We write
            // exactly `slen` valid ASCII bytes (case-transformed), then commit
            // via set_len. All fmt keywords are ASCII, so the result is valid UTF-8.
            unsafe {
                let buf = out.as_mut_vec();
                let old_len = buf.len();
                let dst = buf.as_mut_ptr().add(old_len);
                if case == KeywordCase::Upper {
                    for i in 0..slen {
                        dst.add(i).write(src.get_unchecked(i).to_ascii_uppercase());
                    }
                } else {
                    for i in 0..slen {
                        dst.add(i).write(src.get_unchecked(i).to_ascii_lowercase());
                    }
                }
                buf.set_len(old_len + slen);
            }
        }
    }
}

/// Render buffered line suffixes directly to output (before the next newline).
fn flush_line_suffixes(
    arena: &DocArena,
    buf: &mut Vec<(i32, Mode, DocId)>,
    keyword_case: KeywordCase,
    out: &mut String,
    pos: &mut usize,
) {
    if buf.is_empty() {
        return;
    }
    for (indent, _mode, doc_id) in buf.drain(..) {
        render_inline(arena, doc_id, indent, keyword_case, out, pos);
    }
}

/// Render a doc tree directly into the output string (used for line suffixes).
fn render_inline(
    arena: &DocArena,
    doc_id: DocId,
    indent: i32,
    keyword_case: KeywordCase,
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
                push_keyword(s, keyword_case, out);
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

#[cfg(test)]
mod tests {
    use super::*;

    const WIDTH: usize = 80;
    const CASE: KeywordCase = KeywordCase::Preserve;

    #[test]
    fn arena_alloc_and_get() {
        let mut arena = DocArena::new();
        let id = arena.text("hello");
        assert_eq!(id, 0);
        match arena.get(id) {
            Doc::Text(s) => assert_eq!(*s, "hello"),
            _ => panic!("expected Text"),
        }
    }

    #[test]
    fn nil_doc_identity_in_cat() {
        let mut arena = DocArena::new();
        let a = arena.text("a");
        assert_eq!(arena.cat(NIL_DOC, a), a);
        assert_eq!(arena.cat(a, NIL_DOC), a);
        assert_eq!(arena.cat(NIL_DOC, NIL_DOC), NIL_DOC);
    }

    #[test]
    fn nil_doc_identity_in_nest_and_group() {
        let mut arena = DocArena::new();
        assert_eq!(arena.nest(4, NIL_DOC), NIL_DOC);
        assert_eq!(arena.group(NIL_DOC), NIL_DOC);
        assert_eq!(arena.line_suffix(NIL_DOC), NIL_DOC);
    }

    #[test]
    fn cats_skips_nil() {
        let mut arena = DocArena::new();
        let a = arena.text("a");
        let b = arena.text("b");
        let result = arena.cats(&[NIL_DOC, a, NIL_DOC, b, NIL_DOC]);
        match arena.get(result) {
            Doc::Cat { left, right } => {
                assert_eq!(*left, a);
                assert_eq!(*right, b);
            }
            _ => panic!("expected Cat"),
        }
    }

    #[test]
    fn cats_single_element() {
        let mut arena = DocArena::new();
        let a = arena.text("a");
        assert_eq!(arena.cats(&[a]), a);
    }

    #[test]
    fn cats_empty() {
        let mut arena = DocArena::new();
        assert_eq!(arena.cats(&[]), NIL_DOC);
    }

    #[test]
    fn plain_text() {
        let mut arena = DocArena::new();
        let doc = arena.text("hello world");
        assert_eq!(arena.render(doc, WIDTH, CASE), "hello world");
    }

    #[test]
    fn cat_two_texts() {
        let mut arena = DocArena::new();
        let a = arena.text("hello");
        let b = arena.text(" world");
        let doc = arena.cat(a, b);
        assert_eq!(arena.render(doc, WIDTH, CASE), "hello world");
    }

    #[test]
    fn group_fits_flat() {
        let mut arena = DocArena::new();
        let a = arena.text("a");
        let sp = arena.line();
        let b = arena.text("b");
        let inner = arena.cats(&[a, sp, b]);
        let doc = arena.group(inner);
        assert_eq!(arena.render(doc, WIDTH, CASE), "a b");
    }

    #[test]
    fn group_breaks() {
        let mut arena = DocArena::new();
        let a = arena.text("aaaa");
        let sp = arena.line();
        let b = arena.text("bbbb");
        let inner = arena.cats(&[a, sp, b]);
        let doc = arena.group(inner);
        assert_eq!(arena.render(doc, 6, CASE), "aaaa\nbbbb");
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
        assert_eq!(arena.render(doc, 3, CASE), "a b");
        assert_eq!(arena.render(doc, 2, CASE), "a\n    b");
    }

    #[test]
    fn nil_doc_renders_empty() {
        let arena = DocArena::new();
        assert_eq!(arena.render(NIL_DOC, WIDTH, CASE), "");
    }
}
