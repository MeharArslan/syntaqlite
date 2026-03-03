// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Shared opcode constants and raw op type used by both
//! the codegen emitter (syntaqlite-buildtools) and the runtime interpreter.

// ── Opcodes ─────────────────────────────────────────────────────────────

pub mod opcodes {
    pub const KEYWORD: u8 = 0;
    pub const SPAN: u8 = 1;
    pub const CHILD: u8 = 2;
    pub const LINE: u8 = 3;
    pub const SOFTLINE: u8 = 4;
    pub const HARDLINE: u8 = 5;
    pub const GROUP_START: u8 = 6;
    pub const GROUP_END: u8 = 7;
    pub const NEST_START: u8 = 8;
    pub const NEST_END: u8 = 9;
    pub const IF_SET: u8 = 10;
    pub const ELSE_OP: u8 = 11;
    pub const END_IF: u8 = 12;
    pub const FOR_EACH_START: u8 = 13;
    pub const CHILD_ITEM: u8 = 14;
    pub const FOR_EACH_SEP: u8 = 15;
    pub const FOR_EACH_END: u8 = 16;
    pub const IF_BOOL: u8 = 17;
    pub const IF_FLAG: u8 = 18;
    pub const IF_ENUM: u8 = 19;
    pub const IF_SPAN: u8 = 20;
    pub const ENUM_DISPLAY: u8 = 21;
    pub const FOR_EACH_SELF_START: u8 = 22;
}

/// A compiled op in its binary encoding: 6 bytes total.
/// `a` is used for field indices, `b` for string IDs / ordinals / masks,
/// `c` for skip counts.
#[derive(Clone, Copy)]
pub struct RawOp {
    pub opcode: u8,
    pub a: u8,
    pub b: u16,
    pub c: u16,
}

impl RawOp {
    pub fn simple(opcode: u8) -> Self {
        RawOp {
            opcode,
            a: 0,
            b: 0,
            c: 0,
        }
    }
}
