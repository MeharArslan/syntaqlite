/// A handle into the `DocArena`. `NIL_DOC` represents an empty/absent document.
pub type DocId = u32;

/// Sentinel value meaning "no document". Builder methods treat NIL_DOC operands
/// as identity elements (e.g. `cat(NIL_DOC, x) == x`).
pub const NIL_DOC: DocId = u32::MAX;

/// A node in the document algebra. Lifetime `'a` covers borrowed text slices.
#[derive(Debug, Clone)]
pub enum Doc<'a> {
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
    /// Emit `broken` when enclosing group breaks, `flat` when it fits.
    IfBreak { broken: DocId, flat: DocId },
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

    pub fn with_capacity(cap: usize) -> Self {
        DocArena {
            docs: Vec::with_capacity(cap),
        }
    }

    fn push(&mut self, doc: Doc<'a>) -> DocId {
        let id = self.docs.len() as DocId;
        debug_assert!(id != NIL_DOC, "DocArena overflow");
        self.docs.push(doc);
        id
    }

    /// Get a reference to the doc at `id`. Panics if `id` is `NIL_DOC` or out of bounds.
    pub fn get(&self, id: DocId) -> &Doc<'a> {
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

    pub fn if_break(&mut self, broken: DocId, flat: DocId) -> DocId {
        self.push(Doc::IfBreak { broken, flat })
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
}

#[cfg(test)]
mod tests {
    use super::*;

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

        // cat(NIL, a) == a
        assert_eq!(arena.cat(NIL_DOC, a), a);
        // cat(a, NIL) == a
        assert_eq!(arena.cat(a, NIL_DOC), a);
        // cat(NIL, NIL) == NIL
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
        // Should be Cat { left: a, right: b }
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
        let arena = DocArena::new();
        assert_eq!(arena.docs.len(), 0);
        let mut arena = arena;
        assert_eq!(arena.cats(&[]), NIL_DOC);
    }
}
