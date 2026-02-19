//! Loads the embedded binary bytecode and provides the static formatter data.

use std::sync::LazyLock;

use syntaqlite_fmt_bytecode as bc;

use crate::interpret::FmtCtx;
use crate::ops::{FmtOp, NodeFmt};

/// Loaded formatter data, owning all allocations.
pub struct LoadedFmt {
    strings: Vec<String>,
    enum_display: Vec<u16>,
    ops: Vec<FmtOp>,
    dispatch_entries: Vec<(u16, u16)>, // (offset, length) per tag
}

/// Leaked static formatter data, suitable for use with the interpreter.
pub struct StaticFmt {
    pub dispatch: &'static [Option<NodeFmt>],
    pub ctx: FmtCtx<'static>,
}

impl LoadedFmt {
    pub fn load(data: &[u8]) -> Result<Self, &'static str> {
        let decoded = bc::decode(data)?;

        let mut ops = Vec::with_capacity(decoded.ops.len());
        for &(opcode, a, b, c) in &decoded.ops {
            ops.push(decode_op(opcode, a, b, c)?);
        }

        Ok(LoadedFmt {
            strings: decoded.strings,
            enum_display: decoded.enum_display,
            ops,
            dispatch_entries: decoded.dispatch,
        })
    }

    /// Leak all owned data into `'static` references.
    /// The formatter data lives for the process lifetime.
    pub fn into_static(self) -> StaticFmt {
        // Leak strings: Vec<String> → &'static [&'static str]
        let str_refs: Vec<&'static str> = self
            .strings
            .into_iter()
            .map(|s| -> &'static str { Box::leak(s.into_boxed_str()) })
            .collect();
        let strings: &'static [&'static str] = Box::leak(str_refs.into_boxed_slice());

        // Leak enum display
        let enum_display: &'static [u16] = Box::leak(self.enum_display.into_boxed_slice());

        // Leak ops
        let ops: &'static [FmtOp] = Box::leak(self.ops.into_boxed_slice());

        // Build dispatch table
        let mut dispatch: Vec<Option<NodeFmt>> = Vec::with_capacity(self.dispatch_entries.len());
        for &(offset, length) in &self.dispatch_entries {
            if offset == 0xFFFF {
                dispatch.push(None);
            } else {
                let start = offset as usize;
                let end = start + length as usize;
                dispatch.push(Some(NodeFmt {
                    ops: &ops[start..end],
                }));
            }
        }
        let dispatch: &'static [Option<NodeFmt>] = Box::leak(dispatch.into_boxed_slice());

        let ctx = FmtCtx {
            strings,
            enum_display,
        };

        StaticFmt { dispatch, ctx }
    }
}

static LOADED: LazyLock<StaticFmt> = LazyLock::new(|| {
    LoadedFmt::load(include_bytes!("generated/fmt.bin"))
        .expect("failed to load embedded fmt bytecode")
        .into_static()
});

pub fn dispatch() -> &'static [Option<NodeFmt>] {
    LOADED.dispatch
}

pub fn ctx() -> &'static FmtCtx<'static> {
    &LOADED.ctx
}

fn decode_op(opcode: u8, a: u8, b: u16, c: u16) -> Result<FmtOp, &'static str> {
    use bc::opcodes::*;
    Ok(match opcode {
        KEYWORD => FmtOp::Keyword(b),
        SPAN => FmtOp::Span(a as u16),
        CHILD => FmtOp::Child(a as u16),
        LINE => FmtOp::Line,
        SOFTLINE => FmtOp::SoftLine,
        HARDLINE => FmtOp::HardLine,
        GROUP_START => FmtOp::GroupStart,
        GROUP_END => FmtOp::GroupEnd,
        NEST_START => FmtOp::NestStart(b as i16),
        NEST_END => FmtOp::NestEnd,
        IF_SET => FmtOp::IfSet(a as u16, c),
        ELSE_OP => FmtOp::Else(c),
        END_IF => FmtOp::EndIf,
        FOR_EACH_START => FmtOp::ForEachStart(a as u16),
        CHILD_ITEM => FmtOp::ChildItem,
        FOR_EACH_SEP => FmtOp::ForEachSep(b),
        FOR_EACH_END => FmtOp::ForEachEnd,
        IF_BOOL => FmtOp::IfBool(a as u16, c),
        IF_FLAG => FmtOp::IfFlag(a as u16, b as u8, c),
        IF_ENUM => FmtOp::IfEnum(a as u16, b, c),
        IF_SPAN => FmtOp::IfSpan(a as u16, c),
        ENUM_DISPLAY => FmtOp::EnumDisplay(a as u16, b),
        FOR_EACH_SELF_START => FmtOp::ForEachSelfStart,
        _ => return Err("unknown opcode in bytecode"),
    })
}
