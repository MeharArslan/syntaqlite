use std::ffi::{c_char, c_int, c_void};

// Opaque C types
pub(crate) enum RawParser {}
pub(crate) enum RawDialect {}

#[repr(C)]
pub(crate) struct RawParseResult {
    pub root: u32,
    pub error: c_int,
    pub error_msg: *const c_char,
}

// SyntaqliteMemMethods — matches config.h layout.
#[repr(C)]
pub(crate) struct RawMemMethods {
    pub x_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    pub x_free: unsafe extern "C" fn(*mut c_void),
}

#[repr(C)]
pub(crate) struct RawTrivia {
    pub offset: u32,
    pub length: u32,
    pub kind: u8,
}

#[repr(C)]
pub(crate) struct RawMacroRegion {
    pub call_offset: u32,
    pub call_length: u32,
}

// The C API uses `SyntaqliteNode*` as an opaque return. We only read via
// the tag field (first u32) and then cast to the right struct, so we just
// receive `*const u32`.

unsafe extern "C" {
    // Parser lifecycle
    pub fn syntaqlite_create_parser_with_dialect(
        mem: *const RawMemMethods, dialect: *const RawDialect) -> *mut RawParser;
    pub fn syntaqlite_parser_reset(p: *mut RawParser, source: *const c_char, len: u32);
    pub fn syntaqlite_parser_next(p: *mut RawParser) -> RawParseResult;
    pub fn syntaqlite_parser_destroy(p: *mut RawParser);

    // Parser accessors
    pub fn syntaqlite_parser_node(p: *mut RawParser, node_id: u32) -> *const u32;

    // Parser configuration
    pub fn syntaqlite_parser_set_trace(p: *mut RawParser, enable: c_int);
    pub fn syntaqlite_parser_set_collect_tokens(p: *mut RawParser, enable: c_int);

    // Trivia (comments)
    pub fn syntaqlite_parser_trivia(p: *mut RawParser, count: *mut u32) -> *const RawTrivia;

    // Low-level token-feeding API
    pub fn syntaqlite_parser_feed_token(
        p: *mut RawParser, token_type: c_int,
        text: *const c_char, len: c_int) -> c_int;
    pub fn syntaqlite_parser_result(p: *mut RawParser) -> RawParseResult;
    pub fn syntaqlite_parser_finish(p: *mut RawParser) -> c_int;

    // Macro region tracking
    pub fn syntaqlite_parser_begin_macro(
        p: *mut RawParser, call_offset: u32, call_length: u32);
    pub fn syntaqlite_parser_end_macro(p: *mut RawParser);
    pub fn syntaqlite_parser_macro_regions(
        p: *mut RawParser, count: *mut u32) -> *const RawMacroRegion;

}
