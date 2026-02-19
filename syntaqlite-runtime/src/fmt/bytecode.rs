//! Loads binary bytecode and provides the formatter data.

use super::bytecode_format as bc;

use super::interpret::FmtCtx;
use super::ops::FmtOp;

/// Loaded formatter data, owning all allocations.
pub struct LoadedFmt {
    strings: Vec<String>,
    enum_display: Vec<u16>,
    ops: Vec<FmtOp>,
    dispatch_entries: Vec<(u16, u16)>, // (offset, length) per tag
}

impl LoadedFmt {
    /// Load formatter data from a dialect's C static bytecode.
    pub fn from_dialect(dialect: &crate::Dialect) -> Result<Self, &'static str> {
        let d = unsafe { &*(dialect.raw as *const crate::dialect::RawSyntaqliteDialect) };
        assert!(!d.fmt_data.is_null() && d.fmt_data_len > 0, "C dialect has no fmt data");
        let data = unsafe { std::slice::from_raw_parts(d.fmt_data, d.fmt_data_len as usize) };
        Self::load(data)
    }

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

    /// Get the formatting ops for a node tag, or None if no formatting is defined.
    pub fn node_ops(&self, tag: u32) -> Option<&[FmtOp]> {
        let &(offset, length) = self.dispatch_entries.get(tag as usize)?;
        if offset == 0xFFFF {
            return None;
        }
        Some(&self.ops[offset as usize..offset as usize + length as usize])
    }

    /// Build a FmtCtx borrowing from this LoadedFmt.
    pub fn ctx(&self) -> FmtCtx<'_> {
        FmtCtx {
            strings: &self.strings,
            enum_display: &self.enum_display,
        }
    }

    /// Number of dispatch entries (= node_count).
    pub fn node_count(&self) -> usize {
        self.dispatch_entries.len()
    }
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
