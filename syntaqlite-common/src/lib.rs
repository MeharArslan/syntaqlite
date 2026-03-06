// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#![cfg_attr(test, expect(clippy::unwrap_used, clippy::similar_names))]

//! Shared primitives with no generated-file dependencies.
//!
//! Safe for the bootstrap tool (`syntaqlite-buildtools`) to depend on.

/// Formatter bytecode types shared between the codegen tool and the runtime.
pub mod fmt {
    /// Binary encoding of formatter bytecode programs.
    pub mod bytecode {
        /// Opcode constants for the formatter bytecode instruction set.
        pub mod opcodes {
            /// Emit a fixed keyword string from the dialect's string table.
            pub const KEYWORD: u8 = 0;
            /// Emit a source span (verbatim text slice) from the AST node.
            pub const SPAN: u8 = 1;
            /// Recurse into a child node field.
            pub const CHILD: u8 = 2;
            /// Emit a line break (collapses to a space when the group fits on one line).
            pub const LINE: u8 = 3;
            /// Emit a line break only if the enclosing group does not fit on one line.
            pub const SOFTLINE: u8 = 4;
            /// Always emit a hard (unconditional) line break.
            pub const HARDLINE: u8 = 5;
            /// Begin a group; the renderer tries to fit the group on one line.
            pub const GROUP_START: u8 = 6;
            /// End a group started by `GROUP_START`.
            pub const GROUP_END: u8 = 7;
            /// Begin an indented nesting block.
            pub const NEST_START: u8 = 8;
            /// End a nesting block started by `NEST_START`.
            pub const NEST_END: u8 = 9;
            /// Begin a conditional block gated on whether a node field is present.
            pub const IF_SET: u8 = 10;
            /// Else branch of an `IF_SET` / `IF_BOOL` / `IF_FLAG` / `IF_ENUM` block.
            pub const ELSE_OP: u8 = 11;
            /// End a conditional block.
            pub const END_IF: u8 = 12;
            /// Begin iteration over a list field.
            pub const FOR_EACH_START: u8 = 13;
            /// Emit the current list element inside a `FOR_EACH` loop.
            pub const CHILD_ITEM: u8 = 14;
            /// Emit a separator between list elements.
            pub const FOR_EACH_SEP: u8 = 15;
            /// End a `FOR_EACH` loop.
            pub const FOR_EACH_END: u8 = 16;
            /// Begin a conditional block gated on a boolean inline field.
            pub const IF_BOOL: u8 = 17;
            /// Begin a conditional block gated on a flags bitmask field.
            pub const IF_FLAG: u8 = 18;
            /// Begin a conditional block gated on an enum field value.
            pub const IF_ENUM: u8 = 19;
            /// Begin a conditional block gated on whether a span field is non-empty.
            pub const IF_SPAN: u8 = 20;
            /// Emit the display string for an enum field value.
            pub const ENUM_DISPLAY: u8 = 21;
            /// Begin iteration over the node itself as a list (self-referential list node).
            pub const FOR_EACH_SELF_START: u8 = 22;
        }

        /// A compiled op in its binary encoding: 6 bytes total.
        /// `a` is used for field indices, `b` for string IDs / ordinals / masks,
        /// `c` for skip counts.
        #[derive(Clone, Copy)]
        pub struct RawOp {
            /// The opcode byte identifying the instruction type.
            pub opcode: u8,
            /// Field index or auxiliary operand A.
            pub a: u8,
            /// String ID, ordinal, or bitmask operand B.
            pub b: u16,
            /// Skip count or auxiliary operand C.
            pub c: u16,
        }

        impl RawOp {
            /// Construct an instruction with only an opcode and all other fields zeroed.
            #[must_use]
            pub const fn simple(opcode: u8) -> Self {
                Self {
                    opcode,
                    a: 0,
                    b: 0,
                    c: 0,
                }
            }
        }
    }
}
