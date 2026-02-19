//! C ABI mirror structs for the dialect.

pub const FIELD_NODE_ID: u8 = 0;
pub const FIELD_SPAN: u8 = 1;
pub const FIELD_BOOL: u8 = 2;
pub const FIELD_FLAGS: u8 = 3;
pub const FIELD_ENUM: u8 = 4;

/// Mirrors C `SyntaqliteFieldMeta` from `include/syntaqlite/dialect.h`.
#[repr(C)]
pub struct FieldMeta {
    pub offset: u16,
    pub kind: u8,
    pub name: *const std::ffi::c_char,
    pub display: *const *const std::ffi::c_char,
    pub display_count: u8,
}

const _: () = {
    const P: usize = std::mem::size_of::<*const ()>();
    const fn align_up(n: usize) -> usize { (n + P - 1) & !(P - 1) }

    // offset(0) u16, kind(2) u8, [padding], name(P) ptr, display(2P) ptr, display_count(3P) u8, [padding]
    assert!(std::mem::size_of::<FieldMeta>() == align_up(3 * P + 1));
    assert!(std::mem::offset_of!(FieldMeta, offset) == 0);
    assert!(std::mem::offset_of!(FieldMeta, kind) == 2);
    assert!(std::mem::offset_of!(FieldMeta, name) == P);
    assert!(std::mem::offset_of!(FieldMeta, display) == 2 * P);
    assert!(std::mem::offset_of!(FieldMeta, display_count) == 3 * P);
};

/// Mirrors the C `Dialect` struct defined in `include/syntaqlite/dialect.h`.
#[repr(C)]
pub struct Dialect {
    pub name: *const std::ffi::c_char,

    // Parse tables + reduce actions
    pub tables: *const std::ffi::c_void,
    pub reduce_actions: *const std::ffi::c_void,

    // Range metadata
    pub range_meta: *const std::ffi::c_void,

    // Well-known token IDs (int32_t in C)
    pub tk_space: i32,
    pub tk_semi: i32,
    pub tk_comment: i32,

    // AST metadata
    pub node_count: u32,
    pub node_names: *const *const std::ffi::c_char,
    pub field_meta: *const *const FieldMeta,
    pub field_meta_counts: *const u8,
    pub list_tags: *const u8,

    // Formatter data
    pub fmt_strings: *const *const std::ffi::c_char,
    pub fmt_string_count: u16,
    pub fmt_enum_display: *const u16,
    pub fmt_enum_display_count: u16,
    pub fmt_ops: *const u8,
    pub fmt_op_count: u16,
    pub fmt_dispatch: *const u32,
    pub fmt_dispatch_count: u16,
}

const _: () = {
    const P: usize = std::mem::size_of::<*const ()>();
    const fn align_up(n: usize) -> usize { (n + P - 1) & !(P - 1) }

    // 4 pointers, then 4×i32 (16 bytes), then 4 pointers.
    const AFTER_INTS: usize = 4 * P + 16;
    // fmt_strings is the 9th pointer (indices 0..8), fmt_string_count is u16 after it.
    const FMT_STR_COUNT: usize = 9 * P + 16;
    // Each (ptr, u16) pair: the u16 sits right after the ptr, then padding to next ptr.
    const A: usize = align_up(FMT_STR_COUNT + 2);  // fmt_enum_display
    const B: usize = align_up(A + P + 2);           // fmt_ops
    const C: usize = align_up(B + P + 2);           // fmt_dispatch

    assert!(std::mem::offset_of!(Dialect, name) == 0);
    assert!(std::mem::offset_of!(Dialect, tables) == P);
    assert!(std::mem::offset_of!(Dialect, reduce_actions) == 2 * P);
    assert!(std::mem::offset_of!(Dialect, range_meta) == 3 * P);
    assert!(std::mem::offset_of!(Dialect, tk_space) == 4 * P);
    assert!(std::mem::offset_of!(Dialect, tk_semi) == 4 * P + 4);
    assert!(std::mem::offset_of!(Dialect, tk_comment) == 4 * P + 8);
    assert!(std::mem::offset_of!(Dialect, node_count) == 4 * P + 12);
    assert!(std::mem::offset_of!(Dialect, node_names) == AFTER_INTS);
    assert!(std::mem::offset_of!(Dialect, field_meta) == 5 * P + 16);
    assert!(std::mem::offset_of!(Dialect, field_meta_counts) == 6 * P + 16);
    assert!(std::mem::offset_of!(Dialect, list_tags) == 7 * P + 16);
    assert!(std::mem::offset_of!(Dialect, fmt_strings) == 8 * P + 16);
    assert!(std::mem::offset_of!(Dialect, fmt_string_count) == FMT_STR_COUNT);
    assert!(std::mem::offset_of!(Dialect, fmt_enum_display) == A);
    assert!(std::mem::offset_of!(Dialect, fmt_enum_display_count) == A + P);
    assert!(std::mem::offset_of!(Dialect, fmt_ops) == B);
    assert!(std::mem::offset_of!(Dialect, fmt_op_count) == B + P);
    assert!(std::mem::offset_of!(Dialect, fmt_dispatch) == C);
    assert!(std::mem::offset_of!(Dialect, fmt_dispatch_count) == C + P);
    assert!(std::mem::size_of::<Dialect>() == align_up(C + P + 2));
};
