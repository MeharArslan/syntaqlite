// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// ── Public API ───────────────────────────────────────────────────────────────

use crate::any::{AnyNodeTag, AnyTokenType};
use crate::util::{SqliteFlags, SqliteVersion};

/// The kind of value a struct field holds in the AST node layout.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldKind {
    /// A child node identifier.
    NodeId = 0,
    /// A source span (byte offset + length).
    Span = 1,
    /// A boolean flag.
    Bool = 2,
    /// A compact bitfield of flags.
    Flags = 3,
    /// A discriminant for an enum variant.
    Enum = 4,
}

impl FieldKind {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => FieldKind::Span,
            2 => FieldKind::Bool,
            3 => FieldKind::Flags,
            4 => FieldKind::Enum,
            _ => FieldKind::NodeId,
        }
    }
}

/// Semantic category of a SQL token, used for syntax highlighting.
///
/// Returned by [`AnyGrammar::token_category`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenCategory {
    /// SQL keyword (SELECT, FROM, WHERE, …)
    Keyword,
    /// Bind parameter or session variable (`:name`, `@var`, `?`)
    Variable,
    /// String literal or blob literal
    String,
    /// Numeric literal
    Number,
    /// Operator or comparison symbol (`+`, `=`, `||`, …)
    Operator,
    /// Comment (`-- …` or `/* … */`)
    Comment,
    /// Punctuation (`,`, `(`, `)`, `;`, …)
    Punctuation,
    /// Quoted or unquoted identifier
    Identifier,
    /// Built-in or user-defined function name
    Function,
    /// Type name (in CAST, column definitions, …)
    Type,
    /// Anything that doesn't fall into the above categories
    Other,
}

impl From<ffi::CTokenCategory> for TokenCategory {
    fn from(c: ffi::CTokenCategory) -> Self {
        match c {
            ffi::CTokenCategory::Keyword => Self::Keyword,
            ffi::CTokenCategory::Identifier => Self::Identifier,
            ffi::CTokenCategory::String => Self::String,
            ffi::CTokenCategory::Number => Self::Number,
            ffi::CTokenCategory::Operator => Self::Operator,
            ffi::CTokenCategory::Punctuation => Self::Punctuation,
            ffi::CTokenCategory::Comment => Self::Comment,
            ffi::CTokenCategory::Variable => Self::Variable,
            ffi::CTokenCategory::Function => Self::Function,
            ffi::CTokenCategory::Type => Self::Type,
            ffi::CTokenCategory::Other => Self::Other,
        }
    }
}

/// A reference to one field's metadata entry in the grammar tables.
///
/// Obtained from [`AnyGrammar::field_meta`].
pub struct FieldMeta<'a>(pub(crate) &'a ffi::CFieldMeta);

impl FieldMeta<'_> {
    /// Byte offset of this field within its parent AST node struct.
    pub fn offset(&self) -> u16 {
        self.0.offset
    }

    /// Semantic kind of this field.
    pub fn kind(&self) -> FieldKind {
        FieldKind::from_u8(self.0.kind)
    }

    /// The field name as a `&str`.
    ///
    /// # Panics
    /// Panics if the grammar table contains invalid UTF-8 in the field name
    /// (which would indicate a codegen bug).
    pub fn name(&self) -> &'static str {
        // SAFETY: `FieldMeta` is only constructed from static grammar tables
        // where `name` is always a valid, NUL-terminated UTF-8 C string.
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(self.0.name);
            cstr.to_str().expect("invalid UTF-8 in field name")
        }
    }

    /// The `idx`-th display name for enum variants, if present.
    ///
    /// # Panics
    /// Panics if the grammar table contains invalid UTF-8 in a display name
    /// (which would indicate a codegen bug).
    pub fn display_name(&self, idx: usize) -> Option<&'static str> {
        if self.0.display.is_null() || idx >= self.0.display_count as usize {
            return None;
        }
        // SAFETY: `FieldMeta` is only constructed from static grammar tables;
        // `display` and its entries are valid static C strings.
        unsafe {
            let ptr = *self.0.display.add(idx);
            if ptr.is_null() {
                return None;
            }
            let cstr = std::ffi::CStr::from_ptr(ptr);
            Some(cstr.to_str().expect("invalid UTF-8 in display name"))
        }
    }

    /// Number of display names for this field.
    pub fn display_count(&self) -> usize {
        self.0.display_count as usize
    }
}

/// Token-usage flags set by the parser during disambiguation.
///
/// Returned as part of [`crate::parser::TypedParserToken`].
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ParserTokenFlags(u8);

impl ParserTokenFlags {
    /// Construct from a raw C flag bitfield (`SyntaqliteParserTokenFlags = uint32_t`).
    pub(crate) fn from_raw(v: u32) -> Self {
        let bits = u8::try_from(v).expect("parser token flags out of range for u8");
        ParserTokenFlags(bits)
    }

    // Bit positions — mirror C SYNQ_TOKEN_FLAG_* in syntaqlite/parser.h.
    const AS_ID: u8 = 1;
    const AS_FUNCTION: u8 = 2;
    const AS_TYPE: u8 = 4;

    /// Returns the underlying flag bits.
    pub fn bits(self) -> u8 {
        self.0
    }

    /// True if the token was used as an identifier (`SYNQ_TOKEN_FLAG_AS_ID`).
    pub fn used_as_identifier(self) -> bool {
        self.0 & Self::AS_ID != 0
    }

    /// True if the token was used as a function name (`SYNQ_TOKEN_FLAG_AS_FUNCTION`).
    pub fn used_as_function(self) -> bool {
        self.0 & Self::AS_FUNCTION != 0
    }

    /// True if the token was used as a type name (`SYNQ_TOKEN_FLAG_AS_TYPE`).
    pub fn used_as_type(self) -> bool {
        self.0 & Self::AS_TYPE != 0
    }
}

/// A typed grammar handle.
///
/// Implement this for a grammar struct (e.g. `sqlite::grammar::Grammar`).
/// Must be convertible to [`AnyGrammar`] via `Into<AnyGrammar>`.
pub trait TypedGrammar: Copy + Into<AnyGrammar> {
    /// The top-level typed AST node enum for this grammar.
    type Node<'a>: crate::ast::GrammarNodeType<'a>;
    /// The grammar's typed node ID, wrapping an [`crate::ast::AnyNodeId`].
    ///
    /// Used as the return type of [`TypedNodeList::node_id`](crate::ast::TypedNodeList::node_id)
    /// so callers get a grammar-typed handle rather than a raw [`crate::ast::AnyNodeId`].
    type NodeId: Copy + From<crate::ast::AnyNodeId> + Into<crate::ast::AnyNodeId>;
    /// The typed token enum for this grammar.
    type Token: crate::ast::GrammarTokenType;
}

/// A type-erased grammar handle shared by all dialects.
///
/// Carries the target `SQLite` version and compile-time flags for a dialect.
/// It is cheap to copy and safe to send across threads.
///
/// Obtain one via a grammar-specific wrapper (e.g. `SqliteGrammar`);
/// use [`typed::TypedGrammar`](crate::typed::TypedGrammar) when you need the typed dialect API,
/// or pass `AnyGrammar` directly to [`crate::Parser`] and other grammar-agnostic infrastructure.
#[derive(Clone, Copy)]
pub struct AnyGrammar {
    pub(crate) inner: ffi::CGrammar,
}

// SAFETY: The grammar wraps an immutable reference to static C data.
unsafe impl Send for AnyGrammar {}
// SAFETY: AnyGrammar wraps a *const CGrammar to a static C grammar object; it is safe to share across threads.
unsafe impl Sync for AnyGrammar {}

impl AnyGrammar {
    /// Construct a `AnyGrammar` from a raw C grammar value.\
    ///
    /// This unsafe method exists only for use by grammar implementations which are code generated.
    /// End users should never need to call this directly.
    ///
    /// # Safety
    /// The `template` pointer inside `inner` must point to valid, `'static`
    /// C grammar tables (e.g. returned by a dialect's `extern "C"` grammar
    /// accessor such as `syntaqlite_sqlite_grammar()`).
    pub(crate) unsafe fn new(inner: ffi::CGrammar) -> Self {
        AnyGrammar { inner }
    }

    /// Set the target `SQLite` version.
    #[must_use]
    pub fn with_version(mut self, version: SqliteVersion) -> Self {
        self.inner.sqlite_version = version.as_int();
        self
    }

    /// Replace the entire cflags bitfield.
    #[must_use]
    pub fn with_cflags(mut self, flags: SqliteFlags) -> Self {
        self.inner.cflags = flags.0;
        self
    }

    /// The target `SQLite` version.
    pub fn version(&self) -> SqliteVersion {
        SqliteVersion::from_int(self.inner.sqlite_version)
    }

    /// The active compile-time flags.
    pub fn cflags(&self) -> SqliteFlags {
        SqliteFlags(self.inner.cflags)
    }

    /// Return a reference to the abstract grammar template.
    #[inline]
    fn template(&self) -> &'static ffi::CGrammarTemplate {
        // SAFETY: `inner.template` points to static C data (generated grammar tables).
        unsafe { &*self.inner.template }
    }

    /// Return the node name for the given tag.
    ///
    /// # Panics
    /// Panics if `tag` is out of bounds for this grammar.
    pub fn node_name(&self, tag: AnyNodeTag) -> &'static str {
        let raw = self.template();
        let idx = tag.0 as usize;
        assert!(
            idx < raw.node_count as usize,
            "node tag {} out of bounds (count={})",
            idx,
            raw.node_count,
        );
        // SAFETY: idx is bounds-checked above; node_names is a static array of
        // length node_count populated by codegen, with valid NUL-terminated strings.
        unsafe {
            let cstr = std::ffi::CStr::from_ptr(*raw.node_names.add(idx));
            cstr.to_str().expect("invalid UTF-8 in node name")
        }
    }

    /// Whether the given node tag represents a list node.
    pub fn is_list(&self, tag: AnyNodeTag) -> bool {
        let raw = self.template();
        let idx = tag.0 as usize;
        if idx >= raw.node_count as usize {
            return false;
        }
        // SAFETY: idx is bounds-checked above; list_tags is a static array of
        // length node_count populated by codegen.
        unsafe { *raw.list_tags.add(idx) != 0 }
    }

    /// Return the field metadata for a node tag as an iterator of [`FieldMeta`].
    pub fn field_meta(&self, tag: AnyNodeTag) -> impl ExactSizeIterator<Item = FieldMeta<'static>> {
        let raw = self.template();
        let idx = tag.0 as usize;
        // SAFETY: idx is bounds-checked; field_meta_counts and field_meta are
        // parallel static arrays of length node_count populated by codegen.
        let slice: &'static [ffi::CFieldMeta] = unsafe {
            if idx >= raw.node_count as usize {
                &[]
            } else {
                let count = *raw.field_meta_counts.add(idx) as usize;
                let ptr = *raw.field_meta.add(idx);
                if count == 0 || ptr.is_null() {
                    &[]
                } else {
                    std::slice::from_raw_parts(ptr, count)
                }
            }
        };
        slice.iter().map(FieldMeta)
    }

    /// Classify a token using parser-assigned flags, falling back to the
    /// static token-category table.
    pub fn classify_token(
        &self,
        token_type: AnyTokenType,
        flags: ParserTokenFlags,
    ) -> TokenCategory {
        if flags.used_as_function() {
            TokenCategory::Function
        } else if flags.used_as_type() {
            TokenCategory::Type
        } else if flags.used_as_identifier() {
            TokenCategory::Identifier
        } else {
            self.token_category(token_type)
        }
    }

    /// Return the [`TokenCategory`] for a token type ordinal.
    pub fn token_category(&self, token_type: AnyTokenType) -> TokenCategory {
        let raw = self.template();
        let idx = token_type.0 as usize;
        if raw.token_categories.is_null() || idx >= raw.token_type_count as usize {
            return TokenCategory::Other;
        }
        // SAFETY: token_categories is null-checked; it is a static array of
        // length token_type_count populated by codegen.
        let byte = unsafe { *raw.token_categories.add(idx) };
        TokenCategory::from(ffi::CTokenCategory::from_u8(byte))
    }

    /// Iterate over all keyword entries in the grammar's exported keyword table.
    ///
    /// Yields a [`KeywordEntry`] for each keyword, containing the token type
    /// ordinal and the keyword lexeme (e.g. `SELECT`, `WHERE`).
    ///
    /// The iterator implements [`ExactSizeIterator`], so `.len()` gives the
    /// total keyword count without consuming the iterator.
    pub fn keywords(&self) -> impl ExactSizeIterator<Item = KeywordEntry> + '_ {
        let raw = self.template();
        let count = if raw.keyword_text.is_null()
            || raw.keyword_offsets.is_null()
            || raw.keyword_lens.is_null()
            || raw.keyword_codes.is_null()
            || raw.keyword_count.is_null()
        {
            0
        } else {
            // SAFETY: keyword_count is null-checked above; points to a static u32.
            unsafe { *raw.keyword_count as usize }
        };
        KeywordIter {
            grammar: self,
            idx: 0,
            count,
        }
    }
}

impl TypedGrammar for AnyGrammar {
    type Node<'a> = crate::ast::AnyNode<'a>;
    type NodeId = crate::ast::AnyNodeId;
    type Token = AnyTokenType;
}

/// A single entry from the grammar's exported keyword table.
///
/// Yielded by [`AnyGrammar::keywords`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KeywordEntry {
    /// The token type for this keyword.
    pub token_type: AnyTokenType,
    /// The keyword lexeme (e.g. `"SELECT"`, `"WHERE"`).
    pub keyword: &'static str,
}

struct KeywordIter<'a> {
    grammar: &'a AnyGrammar,
    idx: usize,
    count: usize,
}

impl Iterator for KeywordIter<'_> {
    type Item = KeywordEntry;

    fn next(&mut self) -> Option<KeywordEntry> {
        if self.idx >= self.count {
            return None;
        }
        let raw = self.grammar.template();
        // SAFETY: all keyword pointers were null-checked in `keywords()`; arrays
        // are static, length = self.count, and self.idx < self.count.
        let entry = unsafe {
            let code = u32::from(*raw.keyword_codes.add(self.idx));
            let len = *raw.keyword_lens.add(self.idx) as usize;
            let off = *raw.keyword_offsets.add(self.idx) as usize;
            let bytes = std::slice::from_raw_parts(raw.keyword_text.cast::<u8>().add(off), len);
            KeywordEntry {
                token_type: AnyTokenType(code),
                keyword: std::str::from_utf8_unchecked(bytes),
            }
        };
        self.idx += 1;
        Some(entry)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.count - self.idx;
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for KeywordIter<'_> {}

// ── ffi ───────────────────────────────────────────────────────────────────────

pub(crate) mod ffi {
    use crate::util::ffi::CCflags;

    /// Mirrors C `SynqTokenCategory` enum defined in
    /// `include/syntaqlite/grammar.h`.
    #[repr(u8)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum CTokenCategory {
        Other = 0,
        Keyword = 1,
        Identifier = 2,
        String = 3,
        Number = 4,
        Operator = 5,
        Punctuation = 6,
        Comment = 7,
        Variable = 8,
        Function = 9,
        Type = 10,
    }

    impl CTokenCategory {
        /// Convert a raw byte from the grammar table to a `CTokenCategory`.
        /// Unknown values map to `Other`.
        pub(crate) fn from_u8(v: u8) -> Self {
            match v {
                1 => Self::Keyword,
                2 => Self::Identifier,
                3 => Self::String,
                4 => Self::Number,
                5 => Self::Operator,
                6 => Self::Punctuation,
                7 => Self::Comment,
                8 => Self::Variable,
                9 => Self::Function,
                10 => Self::Type,
                _ => Self::Other,
            }
        }
    }

    /// Mirrors C `SyntaqliteGrammarTemplate` struct defined in
    /// `include/syntaqlite/grammar.h`.
    #[repr(C)]
    pub(crate) struct CGrammarTemplate {
        pub(crate) name: *const std::ffi::c_char,

        // Range metadata
        pub(crate) range_meta: *const std::ffi::c_void,

        // AST metadata
        pub(crate) node_count: u32,
        pub(crate) node_names: *const *const std::ffi::c_char,
        pub(crate) field_meta: *const *const CFieldMeta,
        pub(crate) field_meta_counts: *const u8,
        pub(crate) list_tags: *const u8,

        // Parser lifecycle (function pointers provided by grammar)
        pub(crate) parser_alloc: *const std::ffi::c_void,
        pub(crate) parser_init: *const std::ffi::c_void,
        pub(crate) parser_finalize: *const std::ffi::c_void,
        pub(crate) parser_free: *const std::ffi::c_void,
        pub(crate) parser_feed: *const std::ffi::c_void,
        pub(crate) parser_trace: *const std::ffi::c_void,
        pub(crate) parser_expected_tokens: *const std::ffi::c_void,
        pub(crate) parser_completion_context: *const std::ffi::c_void,

        // Tokenizer (function pointer provided by grammar)
        pub(crate) get_token: *const std::ffi::c_void,

        // Keyword table metadata
        pub(crate) keyword_text: *const std::ffi::c_char,
        pub(crate) keyword_offsets: *const u16,
        pub(crate) keyword_lens: *const u8,
        pub(crate) keyword_codes: *const u8,
        pub(crate) keyword_count: *const u32,

        // Token metadata (indexed by token type ordinal)
        pub(crate) token_categories: *const u8,
        pub(crate) token_type_count: u32,
    }

    /// Mirrors C `SyntaqliteGrammar` from `include/syntaqlite/grammar.h`.
    #[repr(C)]
    #[derive(Debug, Clone, Copy)]
    pub struct CGrammar {
        pub(crate) template: *const CGrammarTemplate,
        pub(crate) sqlite_version: i32,
        pub(crate) cflags: CCflags,
    }

    /// Mirrors C `SyntaqliteFieldMeta` from `include/syntaqlite_dialect/dialect_types.h`.
    #[repr(C)]
    pub(crate) struct CFieldMeta {
        pub(crate) offset: u16,
        pub(crate) kind: u8,
        pub(crate) name: *const std::ffi::c_char,
        pub(crate) display: *const *const std::ffi::c_char,
        pub(crate) display_count: u8,
    }
}
