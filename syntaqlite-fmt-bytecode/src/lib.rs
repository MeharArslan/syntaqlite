//! Shared binary bytecode format definition for the formatter.
//!
//! This crate defines the constants, types, and encode/decode logic
//! shared between the bytecode emitter (syntaqlite-codegen) and the
//! loader (syntaqlite-fmt).
//!
//! Format layout:
//! ```text
//! [Header: 16 bytes] [String Table] [Enum Display Table] [Op Pool] [Dispatch Table]
//! ```
//!
//! Header fields (all little-endian):
//! - magic: 4 bytes `SQFM`
//! - format_version: u16
//! - version_hash: u16
//! - string_count: u16
//! - enum_display_count: u16
//! - op_count: u16
//! - dispatch_count: u16

pub const MAGIC: &[u8; 4] = b"SQFM";
pub const FORMAT_VERSION: u16 = 1;
/// Bump when the opcode set, numbering, or field encoding changes.
pub const BYTECODE_VERSION_HASH: u16 = 1;

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
        RawOp { opcode, a: 0, b: 0, c: 0 }
    }
}

/// Encode a sequence of raw ops + metadata into the binary bytecode format.
pub fn encode(
    strings: &[&str],
    enum_display: &[u16],
    ops: &[RawOp],
    dispatch: &[(u16, u16)],
) -> Vec<u8> {
    let mut buf = Vec::new();

    // Header (16 bytes)
    buf.extend_from_slice(MAGIC);
    buf.extend_from_slice(&FORMAT_VERSION.to_le_bytes());
    buf.extend_from_slice(&BYTECODE_VERSION_HASH.to_le_bytes());
    buf.extend_from_slice(&(strings.len() as u16).to_le_bytes());
    buf.extend_from_slice(&(enum_display.len() as u16).to_le_bytes());
    buf.extend_from_slice(&(ops.len() as u16).to_le_bytes());
    buf.extend_from_slice(&(dispatch.len() as u16).to_le_bytes());

    // String table
    for s in strings {
        buf.extend_from_slice(&(s.len() as u16).to_le_bytes());
        buf.extend_from_slice(s.as_bytes());
    }

    // Enum display table
    for &sid in enum_display {
        buf.extend_from_slice(&sid.to_le_bytes());
    }

    // Op pool (6 bytes per op)
    for op in ops {
        buf.push(op.opcode);
        buf.push(op.a);
        buf.extend_from_slice(&op.b.to_le_bytes());
        buf.extend_from_slice(&op.c.to_le_bytes());
    }

    // Dispatch table (4 bytes per entry)
    for &(offset, length) in dispatch {
        buf.extend_from_slice(&offset.to_le_bytes());
        buf.extend_from_slice(&length.to_le_bytes());
    }

    buf
}

/// Decoded bytecode header + raw sections (no FmtOp dependency).
pub struct Decoded {
    pub strings: Vec<String>,
    pub enum_display: Vec<u16>,
    pub ops: Vec<(u8, u8, u16, u16)>, // (opcode, a, b, c)
    pub dispatch: Vec<(u16, u16)>,     // (offset, length)
}

fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Decode the binary bytecode format into raw sections.
pub fn decode(data: &[u8]) -> Result<Decoded, &'static str> {
    if data.len() < 16 {
        return Err("bytecode too short for header");
    }

    if &data[0..4] != MAGIC.as_slice() {
        return Err("invalid magic bytes");
    }

    let version = read_u16_le(data, 4);
    if version != FORMAT_VERSION {
        return Err("unsupported format version");
    }

    let hash = read_u16_le(data, 6);
    if hash != BYTECODE_VERSION_HASH {
        return Err("bytecode version hash mismatch — stale .bin file?");
    }

    let string_count = read_u16_le(data, 8) as usize;
    let enum_display_count = read_u16_le(data, 10) as usize;
    let op_count = read_u16_le(data, 12) as usize;
    let dispatch_count = read_u16_le(data, 14) as usize;

    let mut pos = 16;

    // String table
    let mut strings = Vec::with_capacity(string_count);
    for _ in 0..string_count {
        if pos + 2 > data.len() {
            return Err("unexpected end of string table");
        }
        let len = read_u16_le(data, pos) as usize;
        pos += 2;
        if pos + len > data.len() {
            return Err("string data exceeds buffer");
        }
        let s = std::str::from_utf8(&data[pos..pos + len])
            .map_err(|_| "invalid UTF-8 in string table")?;
        strings.push(s.to_string());
        pos += len;
    }

    // Enum display table
    let enum_display_bytes = enum_display_count * 2;
    if pos + enum_display_bytes > data.len() {
        return Err("unexpected end of enum display table");
    }
    let mut enum_display = Vec::with_capacity(enum_display_count);
    for i in 0..enum_display_count {
        enum_display.push(read_u16_le(data, pos + i * 2));
    }
    pos += enum_display_bytes;

    // Op pool (6 bytes per op)
    let op_bytes = op_count * 6;
    if pos + op_bytes > data.len() {
        return Err("unexpected end of op pool");
    }
    let mut ops = Vec::with_capacity(op_count);
    for i in 0..op_count {
        let base = pos + i * 6;
        ops.push((
            data[base],
            data[base + 1],
            read_u16_le(data, base + 2),
            read_u16_le(data, base + 4),
        ));
    }
    pos += op_bytes;

    // Dispatch table (4 bytes per entry)
    let dispatch_bytes = dispatch_count * 4;
    if pos + dispatch_bytes > data.len() {
        return Err("unexpected end of dispatch table");
    }
    let mut dispatch = Vec::with_capacity(dispatch_count);
    for i in 0..dispatch_count {
        let base = pos + i * 4;
        dispatch.push((read_u16_le(data, base), read_u16_le(data, base + 2)));
    }

    Ok(Decoded { strings, enum_display, ops, dispatch })
}
