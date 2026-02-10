use std::ffi::{c_char, c_int, c_void};

// Opaque C types
pub(crate) enum RawParser {}
pub(crate) enum RawTokenizer {}

#[repr(C)]
pub(crate) struct RawParseResult {
    pub root: u32,
    pub error: c_int,
    pub error_msg: *const c_char,
}

#[repr(C)]
pub(crate) struct RawToken {
    pub text: *const c_char,
    pub length: u32,
    pub type_: u32,
}

// SyntaqliteMemMethods — matches config.h layout.
#[repr(C)]
pub(crate) struct RawMemMethods {
    pub x_malloc: unsafe extern "C" fn(usize) -> *mut c_void,
    pub x_free: unsafe extern "C" fn(*mut c_void),
}

// The C API uses `SyntaqliteNode*` as an opaque return. We only read via
// the tag field (first u32) and then cast to the right struct, so we just
// receive `*const u32`.

unsafe extern "C" {
    // Parser lifecycle
    pub fn syntaqlite_parser_create(mem: *const RawMemMethods) -> *mut RawParser;
    pub fn syntaqlite_parser_reset(p: *mut RawParser, source: *const c_char, len: u32);
    pub fn syntaqlite_parser_next(p: *mut RawParser) -> RawParseResult;
    pub fn syntaqlite_parser_destroy(p: *mut RawParser);

    // Parser accessors
    pub fn syntaqlite_parser_node(p: *mut RawParser, node_id: u32) -> *const u32;

    // Parser configuration
    pub fn syntaqlite_parser_set_trace(p: *mut RawParser, enable: c_int);

    // Tokenizer lifecycle
    pub fn syntaqlite_tokenizer_create(mem: *const RawMemMethods) -> *mut RawTokenizer;
    pub fn syntaqlite_tokenizer_reset(tok: *mut RawTokenizer, source: *const c_char, len: u32);
    pub fn syntaqlite_tokenizer_next(tok: *mut RawTokenizer, out: *mut RawToken) -> c_int;
    pub fn syntaqlite_tokenizer_destroy(tok: *mut RawTokenizer);
}
