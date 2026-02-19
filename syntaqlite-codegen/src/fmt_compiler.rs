//! Compiles .synq `Fmt` AST trees into FmtOp bytecode and emits them
//! as either generated Rust source code or binary bytecode.
//!
//! The generated Rust file contains:
//! - `STRINGS: &[&str]` — interned keywords/punctuation
//! - `ENUM_DISPLAY: &[u16]` — flat table mapping enum ordinals → StringId
//! - Per-node `FMT_XXX: &[FmtOp]` constant arrays
//! - `DISPATCH` table indexed by NodeTag ordinal
//!
//! The binary bytecode file contains the same data in a compact format
//! loadable at runtime (see `bytecode.rs` in syntaqlite-fmt).

use std::collections::HashMap;
use std::fmt::Write as _;

#[cfg(test)]
use crate::node_parser::Storage;
use crate::node_parser::{Field, Fmt, Item};

use syntaqlite_runtime::fmt::bytecode_format::opcodes;
use syntaqlite_runtime::fmt::bytecode_format::RawOp;

/// Convert a field index (u16) to u8 for binary encoding, panicking if too large.
fn idx_u8(idx: u16) -> u8 {
    assert!(idx < 256, "field index {} too large for bytecode encoding", idx);
    idx as u8
}

// ── String interning ────────────────────────────────────────────────────

struct StringTable {
    strings: Vec<String>,
    index: HashMap<String, u16>,
}

impl StringTable {
    fn new() -> Self {
        StringTable {
            strings: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn intern(&mut self, s: &str) -> u16 {
        if let Some(&id) = self.index.get(s) {
            return id;
        }
        let id = self.strings.len() as u16;
        self.index.insert(s.to_string(), id);
        self.strings.push(s.to_string());
        id
    }
}

// ── Enum display table ──────────────────────────────────────────────────

struct EnumDisplayTable {
    /// Flat array of StringId values. Each enum_display block reserves
    /// `variant_count` consecutive slots starting at `base`.
    entries: Vec<u16>,
}

impl EnumDisplayTable {
    fn new() -> Self {
        EnumDisplayTable {
            entries: Vec::new(),
        }
    }

    /// Reserve slots for an enum display and return the base index.
    /// `mappings` maps variant name → display string.
    /// `all_variants` is the full ordered list of variant names for the enum.
    fn add(
        &mut self,
        strings: &mut StringTable,
        all_variants: &[String],
        mappings: &[(String, String)],
    ) -> u16 {
        let base = self.entries.len() as u16;
        let map: HashMap<&str, &str> = mappings
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect();
        for variant in all_variants {
            let sid = if let Some(&display) = map.get(variant.as_str()) {
                strings.intern(display)
            } else {
                // Unmapped variants get empty string (should not be reached at runtime)
                strings.intern("")
            };
            self.entries.push(sid);
        }
        base
    }
}

// ── Field mapping ───────────────────────────────────────────────────────

/// Info about a field needed during compilation.
#[derive(Debug, Clone)]
struct FieldInfo {
    /// Index into the FieldVal array for this node.
    idx: u16,
    /// The type name (e.g. "SelectStmtFlags", "SortOrder", "SyntaqliteSourceSpan").
    type_name: String,
}

/// Collect fields for a node, assigning sequential indices.
fn build_field_map(fields: &[Field]) -> (Vec<FieldInfo>, HashMap<String, usize>) {
    let mut infos = Vec::new();
    let mut name_to_idx = HashMap::new();
    for (i, f) in fields.iter().enumerate() {
        name_to_idx.insert(f.name.clone(), i);
        infos.push(FieldInfo {
            idx: i as u16,
            type_name: f.type_name.clone(),
        });
    }
    (infos, name_to_idx)
}

// ── Fmt → RawOp compilation ─────────────────────────────────────────────

struct CompileCtx<'a> {
    strings: &'a mut StringTable,
    enum_display: &'a mut EnumDisplayTable,
    field_infos: &'a [FieldInfo],
    field_map: &'a HashMap<String, usize>,
    enum_items: &'a HashMap<String, Vec<String>>,
    flags_items: &'a HashMap<String, Vec<(String, u32)>>,
}

impl<'a> CompileCtx<'a> {
    fn field(&self, name: &str) -> &FieldInfo {
        let idx = self.field_map.get(name)
            .unwrap_or_else(|| panic!("unknown field: {}", name));
        &self.field_infos[*idx]
    }

    /// Resolve which enum type a field has (for enum_display, if_enum, switch).
    fn enum_variants(&self, field_name: &str) -> &[String] {
        let info = self.field(field_name);
        self.enum_items
            .get(&info.type_name)
            .unwrap_or_else(|| panic!("field {} has type {} which is not an enum", field_name, info.type_name))
    }

    /// Find the ordinal of a variant within an enum.
    fn enum_ordinal(&self, field_name: &str, variant: &str) -> u16 {
        let variants = self.enum_variants(field_name);
        variants
            .iter()
            .position(|v| v == variant)
            .unwrap_or_else(|| panic!("variant {} not found in enum for field {}", variant, field_name))
            as u16
    }

    /// Find the bit mask for a flag within a flags type, or handle Bool fields.
    fn flag_mask(&self, field_name: &str, bit_name: Option<&str>) -> u8 {
        let info = self.field(field_name);
        if let Some(flags) = self.flags_items.get(&info.type_name) {
            let bit = bit_name.expect("flags field requires .bit_name");
            flags
                .iter()
                .find(|(n, _)| n.to_lowercase() == bit.to_lowercase())
                .map(|(_, v)| *v as u8)
                .unwrap_or_else(|| panic!("flag {} not found in {}", bit, info.type_name))
        } else {
            panic!("field {} has type {} which is not a flags type", field_name, info.type_name);
        }
    }

    fn is_bool_field(&self, name: &str) -> bool {
        let info = self.field(name);
        info.type_name == "Bool"
    }

    fn is_flags_field(&self, name: &str) -> bool {
        let info = self.field(name);
        self.flags_items.contains_key(&info.type_name)
    }

}

/// Compile a sequence of Fmt nodes into RawOps.
fn compile_seq(fmts: &[Fmt], ctx: &mut CompileCtx, ops: &mut Vec<RawOp>) {
    for fmt in fmts {
        compile_one(fmt, ctx, ops);
    }
}

fn compile_one(fmt: &Fmt, ctx: &mut CompileCtx, ops: &mut Vec<RawOp>) {
    match fmt {
        Fmt::Text(s) => {
            let sid = ctx.strings.intern(s);
            ops.push(RawOp { opcode: opcodes::KEYWORD, a: 0, b: sid, c: 0 });
        }
        Fmt::Child(field) if field == "_item" => {
            ops.push(RawOp::simple(opcodes::CHILD_ITEM));
        }
        Fmt::Child(field) => {
            let info = ctx.field(field);
            ops.push(RawOp { opcode: opcodes::CHILD, a: idx_u8(info.idx), b: 0, c: 0 });
        }
        Fmt::Span(field) => {
            let info = ctx.field(field);
            ops.push(RawOp { opcode: opcodes::SPAN, a: idx_u8(info.idx), b: 0, c: 0 });
        }
        Fmt::Line => ops.push(RawOp::simple(opcodes::LINE)),
        Fmt::SoftLine => ops.push(RawOp::simple(opcodes::SOFTLINE)),
        Fmt::HardLine => ops.push(RawOp::simple(opcodes::HARDLINE)),
        Fmt::Group(body) => {
            ops.push(RawOp::simple(opcodes::GROUP_START));
            compile_seq(body, ctx, ops);
            ops.push(RawOp::simple(opcodes::GROUP_END));
        }
        Fmt::Nest(body) => {
            ops.push(RawOp { opcode: opcodes::NEST_START, a: 0, b: 2u16, c: 0 });
            compile_seq(body, ctx, ops);
            ops.push(RawOp::simple(opcodes::NEST_END));
        }
        Fmt::IfSet { field, then, els } => {
            let idx = idx_u8(ctx.field(field).idx);
            compile_conditional(
                RawOp { opcode: opcodes::IF_SET, a: idx, b: 0, c: 0 },
                then, els.as_deref(), ctx, ops,
            );
        }
        Fmt::IfFlag { field, bit, then, els } => {
            let base_field = field.as_str();
            if ctx.is_bool_field(base_field) {
                let idx = idx_u8(ctx.field(base_field).idx);
                compile_conditional(
                    RawOp { opcode: opcodes::IF_BOOL, a: idx, b: 0, c: 0 },
                    then, els.as_deref(), ctx, ops,
                );
            } else if ctx.is_flags_field(base_field) {
                let idx = idx_u8(ctx.field(base_field).idx);
                let mask = ctx.flag_mask(base_field, bit.as_deref());
                compile_conditional(
                    RawOp { opcode: opcodes::IF_FLAG, a: idx, b: mask as u16, c: 0 },
                    then, els.as_deref(), ctx, ops,
                );
            } else {
                panic!("if_flag on field {} which is neither Bool nor Flags", field);
            }
        }
        Fmt::IfEnum { field, variant, then, els } => {
            let idx = idx_u8(ctx.field(field).idx);
            let ordinal = ctx.enum_ordinal(field, variant);
            compile_conditional(
                RawOp { opcode: opcodes::IF_ENUM, a: idx, b: ordinal, c: 0 },
                then, els.as_deref(), ctx, ops,
            );
        }
        Fmt::IfSpan { field, then, els } => {
            let idx = idx_u8(ctx.field(field).idx);
            compile_conditional(
                RawOp { opcode: opcodes::IF_SPAN, a: idx, b: 0, c: 0 },
                then, els.as_deref(), ctx, ops,
            );
        }
        Fmt::Clause { keyword, field } => {
            let field_idx = idx_u8(ctx.field(field).idx);
            compile_conditional(
                RawOp { opcode: opcodes::IF_SET, a: field_idx, b: 0, c: 0 },
                &[
                    Fmt::Line,
                    Fmt::Text(keyword.clone()),
                    Fmt::Nest(vec![Fmt::Line, Fmt::Child(field.clone())]),
                ],
                None,
                ctx,
                ops,
            );
        }
        Fmt::Switch { field, cases, default } => {
            compile_switch(field, cases, default.as_deref(), ctx, ops);
        }
        Fmt::EnumDisplay { field, mappings } => {
            let field_idx = idx_u8(ctx.field(field).idx);
            let variants = ctx.enum_variants(field).to_vec();
            let base = ctx.enum_display.add(ctx.strings, &variants, mappings);
            ops.push(RawOp { opcode: opcodes::ENUM_DISPLAY, a: field_idx, b: base, c: 0 });
        }
        Fmt::ForEach { sep, body } => {
            ops.push(RawOp::simple(opcodes::FOR_EACH_SELF_START));
            for item in body {
                compile_foreach_body_item(item, ctx, ops);
            }
            if let Some(sep_items) = sep {
                let mut emitted_sep = false;
                for s in sep_items {
                    if !emitted_sep {
                        match s {
                            Fmt::Text(text) => {
                                let sid = ctx.strings.intern(text);
                                ops.push(RawOp { opcode: opcodes::FOR_EACH_SEP, a: 0, b: sid, c: 0 });
                                emitted_sep = true;
                                continue;
                            }
                            _ => {
                                let sid = ctx.strings.intern("");
                                ops.push(RawOp { opcode: opcodes::FOR_EACH_SEP, a: 0, b: sid, c: 0 });
                                emitted_sep = true;
                            }
                        }
                    }
                    compile_foreach_body_item(s, ctx, ops);
                }
                if !emitted_sep {
                    let sid = ctx.strings.intern("");
                    ops.push(RawOp { opcode: opcodes::FOR_EACH_SEP, a: 0, b: sid, c: 0 });
                }
            }
            ops.push(RawOp::simple(opcodes::FOR_EACH_END));
        }
    }
}

/// Compile a single item inside a for_each body, mapping `child(_item)` to `ChildItem`.
fn compile_foreach_body_item(fmt: &Fmt, ctx: &mut CompileCtx, ops: &mut Vec<RawOp>) {
    match fmt {
        Fmt::Child(name) if name == "_item" => {
            ops.push(RawOp::simple(opcodes::CHILD_ITEM));
        }
        _ => compile_one(fmt, ctx, ops),
    }
}

/// Compile a conditional (IfXxx ... Else ... EndIf) with skip-count fixup.
/// `head` must have `c = 0`; it will be set to the computed skip count.
fn compile_conditional(
    head: RawOp,
    then: &[Fmt],
    els: Option<&[Fmt]>,
    ctx: &mut CompileCtx,
    ops: &mut Vec<RawOp>,
) {
    let head_pos = ops.len();
    ops.push(head); // placeholder — c will be filled in

    // Compile then-branch
    let then_start = ops.len();
    compile_seq(then, ctx, ops);
    let then_len = ops.len() - then_start;

    if let Some(else_body) = els {
        // Add Else (placeholder)
        let else_pos = ops.len();
        ops.push(RawOp::simple(opcodes::ELSE_OP)); // c filled below

        // Compile else-branch
        let else_start = ops.len();
        compile_seq(else_body, ctx, ops);
        let else_len = ops.len() - else_start;

        // EndIf
        ops.push(RawOp::simple(opcodes::END_IF));

        // Fix up skip counts
        ops[head_pos].c = (then_len + 1) as u16;
        ops[else_pos].c = (else_len + 1) as u16;
    } else {
        // No else branch
        ops.push(RawOp::simple(opcodes::END_IF));
        ops[head_pos].c = (then_len + 1) as u16;
    }
}

/// Compile a switch(field) { VARIANT { ... } ... default { ... } } into chained IfEnum blocks.
fn compile_switch(
    field: &str,
    cases: &[(String, Vec<Fmt>)],
    default: Option<&[Fmt]>,
    ctx: &mut CompileCtx,
    ops: &mut Vec<RawOp>,
) {
    let field_idx = idx_u8(ctx.field(field).idx);

    struct CaseChunk {
        ordinal: u16,
        body_ops: Vec<RawOp>,
    }
    let mut chunks: Vec<CaseChunk> = Vec::new();
    for (variant, body) in cases {
        let ordinal = ctx.enum_ordinal(field, variant);
        let mut body_ops = Vec::new();
        compile_seq(body, ctx, &mut body_ops);
        chunks.push(CaseChunk { ordinal, body_ops });
    }

    let mut default_ops = Vec::new();
    if let Some(def) = default {
        compile_seq(def, ctx, &mut default_ops);
    }

    fn emit_chain(
        field_idx: u8,
        chunks: &[CaseChunk],
        default_ops: &[RawOp],
        ops: &mut Vec<RawOp>,
    ) {
        if chunks.is_empty() {
            for op in default_ops {
                ops.push(*op);
            }
            return;
        }

        let chunk = &chunks[0];
        let rest = &chunks[1..];

        // Compile the else branch into a temporary buffer to measure its size
        let mut else_ops = Vec::new();
        emit_chain(field_idx, rest, default_ops, &mut else_ops);

        let then_len = chunk.body_ops.len();
        let has_else = !else_ops.is_empty();

        if has_else {
            ops.push(RawOp { opcode: opcodes::IF_ENUM, a: field_idx, b: chunk.ordinal, c: (then_len + 1) as u16 });
            for op in &chunk.body_ops {
                ops.push(*op);
            }
            ops.push(RawOp { opcode: opcodes::ELSE_OP, a: 0, b: 0, c: (else_ops.len() + 1) as u16 });
            for op in &else_ops {
                ops.push(*op);
            }
            ops.push(RawOp::simple(opcodes::END_IF));
        } else {
            ops.push(RawOp { opcode: opcodes::IF_ENUM, a: field_idx, b: chunk.ordinal, c: (then_len + 1) as u16 });
            for op in &chunk.body_ops {
                ops.push(*op);
            }
            ops.push(RawOp::simple(opcodes::END_IF));
        }
    }

    emit_chain(field_idx, &chunks, &default_ops, ops);
}

// ── Shared compilation ──────────────────────────────────────────────────

struct CompiledNode {
    name: String,
    ops: Vec<RawOp>,
}

struct CompiledFmt {
    strings: StringTable,
    enum_display: EnumDisplayTable,
    nodes: Vec<CompiledNode>,
    tag_count: usize,
}

/// Compile all items into the intermediate representation shared by both emitters.
fn compile_all(items: &[Item]) -> CompiledFmt {
    let enum_items: HashMap<String, Vec<String>> = items
        .iter()
        .filter_map(|item| match item {
            Item::Enum { name, variants } => Some((name.clone(), variants.clone())),
            _ => None,
        })
        .collect();

    let flags_items: HashMap<String, Vec<(String, u32)>> = items
        .iter()
        .filter_map(|item| match item {
            Item::Flags { name, flags } => Some((name.clone(), flags.clone())),
            _ => None,
        })
        .collect();

    let mut strings = StringTable::new();
    let mut enum_display = EnumDisplayTable::new();
    let mut compiled: Vec<CompiledNode> = Vec::new();

    for item in items {
        match item {
            Item::Node { name, fields, fmt: Some(fmt_body) } => {
                let (field_infos, field_map) = build_field_map(fields);
                let mut ops = Vec::new();
                let mut cctx = CompileCtx {
                    strings: &mut strings,
                    enum_display: &mut enum_display,
                    field_infos: &field_infos,
                    field_map: &field_map,
                    enum_items: &enum_items,
                    flags_items: &flags_items,
                };
                compile_seq(fmt_body, &mut cctx, &mut ops);
                compiled.push(CompiledNode { name: name.clone(), ops });
            }
            Item::List { name, fmt: Some(fmt_body), .. } => {
                let (field_infos, field_map) = build_field_map(&[]);
                let mut ops = Vec::new();
                let mut cctx = CompileCtx {
                    strings: &mut strings,
                    enum_display: &mut enum_display,
                    field_infos: &field_infos,
                    field_map: &field_map,
                    enum_items: &enum_items,
                    flags_items: &flags_items,
                };
                compile_seq(fmt_body, &mut cctx, &mut ops);
                compiled.push(CompiledNode { name: name.clone(), ops });
            }
            Item::List { name, fmt: None, .. } => {
                // Default list fmt: for_each(sep: ",") { child(_item) line }
                let comma_sid = strings.intern(",");
                let ops = vec![
                    RawOp::simple(opcodes::FOR_EACH_SELF_START),
                    RawOp::simple(opcodes::CHILD_ITEM),
                    RawOp { opcode: opcodes::FOR_EACH_SEP, a: 0, b: comma_sid, c: 0 },
                    RawOp::simple(opcodes::LINE),
                    RawOp::simple(opcodes::FOR_EACH_END),
                ];
                compiled.push(CompiledNode { name: name.clone(), ops });
            }
            _ => {}
        }
    }

    let tag_count = items
        .iter()
        .filter(|i| matches!(i, Item::Node { .. } | Item::List { .. }))
        .count()
        + 1;

    CompiledFmt { strings, enum_display, nodes: compiled, tag_count }
}

// ── RawOp → Rust source text ────────────────────────────────────────────

fn raw_op_to_string(op: &RawOp) -> String {
    match op.opcode {
        opcodes::KEYWORD => format!("FmtOp::Keyword({})", op.b),
        opcodes::SPAN => format!("FmtOp::Span({})", op.a),
        opcodes::CHILD => format!("FmtOp::Child({})", op.a),
        opcodes::LINE => "FmtOp::Line".to_string(),
        opcodes::SOFTLINE => "FmtOp::SoftLine".to_string(),
        opcodes::HARDLINE => "FmtOp::HardLine".to_string(),
        opcodes::GROUP_START => "FmtOp::GroupStart".to_string(),
        opcodes::GROUP_END => "FmtOp::GroupEnd".to_string(),
        opcodes::NEST_START => format!("FmtOp::NestStart({})", op.b as i16),
        opcodes::NEST_END => "FmtOp::NestEnd".to_string(),
        opcodes::IF_SET => format!("FmtOp::IfSet({}, {})", op.a, op.c),
        opcodes::ELSE_OP => format!("FmtOp::Else({})", op.c),
        opcodes::END_IF => "FmtOp::EndIf".to_string(),
        opcodes::FOR_EACH_START => format!("FmtOp::ForEachStart({})", op.a),
        opcodes::CHILD_ITEM => "FmtOp::ChildItem".to_string(),
        opcodes::FOR_EACH_SEP => format!("FmtOp::ForEachSep({})", op.b),
        opcodes::FOR_EACH_END => "FmtOp::ForEachEnd".to_string(),
        opcodes::IF_BOOL => format!("FmtOp::IfBool({}, {})", op.a, op.c),
        opcodes::IF_FLAG => format!("FmtOp::IfFlag({}, {}, {})", op.a, op.b, op.c),
        opcodes::IF_ENUM => format!("FmtOp::IfEnum({}, {}, {})", op.a, op.b, op.c),
        opcodes::IF_SPAN => format!("FmtOp::IfSpan({}, {})", op.a, op.c),
        opcodes::ENUM_DISPLAY => format!("FmtOp::EnumDisplay({}, {})", op.a, op.b),
        opcodes::FOR_EACH_SELF_START => "FmtOp::ForEachSelfStart".to_string(),
        _ => panic!("unknown opcode {}", op.opcode),
    }
}

// ── Rust code emission ──────────────────────────────────────────────────

fn pascal_to_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

fn upper_snake(name: &str) -> String {
    pascal_to_snake(name).to_uppercase()
}

/// Generate the complete `fmt_ops.rs` file (const-array style, used by tests).
pub fn generate_rust_fmt_ops(items: &[Item]) -> String {
    let compiled = compile_all(items);

    let mut out = String::new();
    writeln!(out, "// @generated by syntaqlite-codegen — DO NOT EDIT").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "#![allow(unused)]").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "use syntaqlite_parser::*;").unwrap();
    writeln!(out, "use crate::interpret::FmtCtx;").unwrap();
    writeln!(out, "use crate::ops::{{FmtOp, NodeFmt}};").unwrap();
    writeln!(out, "use crate::DocArena;").unwrap();
    writeln!(out).unwrap();

    // String table
    writeln!(out, "const STRINGS: &[&str] = &[").unwrap();
    for s in &compiled.strings.strings {
        writeln!(out, "    {:?},", s).unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out).unwrap();

    // Enum display table
    writeln!(out, "const ENUM_DISPLAY: &[u16] = &[").unwrap();
    for &sid in &compiled.enum_display.entries {
        write!(out, "{}, ", sid).unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out).unwrap();

    // Per-node: const ops array
    for cn in &compiled.nodes {
        let upper = upper_snake(&cn.name);
        writeln!(out, "const FMT_{}: &[FmtOp] = &[", upper).unwrap();
        for op in &cn.ops {
            writeln!(out, "    {},", raw_op_to_string(op)).unwrap();
        }
        writeln!(out, "];").unwrap();
        writeln!(out).unwrap();
    }

    // Dispatch table
    writeln!(out, "pub const DISPATCH: [Option<NodeFmt>; {}] = {{", compiled.tag_count).unwrap();
    writeln!(out, "    const NONE: Option<NodeFmt> = None;").unwrap();
    writeln!(out, "    let mut t = [NONE; {}];", compiled.tag_count).unwrap();
    for cn in &compiled.nodes {
        let upper = upper_snake(&cn.name);
        writeln!(
            out,
            "    t[NodeTag::{} as usize] = Some(NodeFmt {{ ops: FMT_{} }});",
            cn.name, upper,
        )
        .unwrap();
    }
    writeln!(out, "    t").unwrap();
    writeln!(out, "}};").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "pub const CTX: FmtCtx<'static> = FmtCtx {{ strings: STRINGS, enum_display: ENUM_DISPLAY }};").unwrap();

    out
}

// ── Binary bytecode emission ────────────────────────────────────────────

/// Generate binary bytecode for the formatter.
///
/// Format: `[Header:16] [StringTable] [EnumDisplay] [OpPool] [DispatchTable]`
pub fn generate_fmt_bytecode(items: &[Item]) -> Vec<u8> {
    let compiled = compile_all(items);

    // Flatten all ops into a single pool, recording each node's offset and length.
    let mut op_pool: Vec<RawOp> = Vec::new();
    let mut node_ranges: Vec<(&str, u16, u16)> = Vec::new(); // (name, offset, length)

    for cn in &compiled.nodes {
        let offset = op_pool.len() as u16;
        let length = cn.ops.len() as u16;
        op_pool.extend_from_slice(&cn.ops);
        node_ranges.push((&cn.name, offset, length));
    }

    // Build ordinal map: node name → tag ordinal.
    // Ordinals are assigned sequentially starting from 1 (0 = Null tag).
    let mut ordinal_map: HashMap<&str, usize> = HashMap::new();
    let mut next_ordinal = 1usize;
    for item in items {
        match item {
            Item::Node { name, .. } | Item::List { name, .. } => {
                ordinal_map.insert(name, next_ordinal);
                next_ordinal += 1;
            }
            _ => {}
        }
    }

    // Build dispatch table
    let mut dispatch_table: Vec<(u16, u16)> = vec![(0xFFFF, 0); compiled.tag_count];
    for &(name, offset, length) in &node_ranges {
        if let Some(&ordinal) = ordinal_map.get(name) {
            dispatch_table[ordinal] = (offset, length);
        }
    }

    let string_refs: Vec<&str> = compiled.strings.strings.iter().map(|s| s.as_str()).collect();
    syntaqlite_runtime::fmt::bytecode_format::encode(&string_refs, &compiled.enum_display.entries, &op_pool, &dispatch_table)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_simple_keyword() {
        let items = vec![
            Item::Node {
                name: "Literal".into(),
                fields: vec![Field {
                    name: "source".into(),
                    storage: Storage::Inline,
                    type_name: "SyntaqliteSourceSpan".into(),
                }],
                fmt: Some(vec![Fmt::Span("source".into())]),
            },
        ];

        let output = generate_rust_fmt_ops(&items);
        assert!(output.contains("FMT_LITERAL"));
        assert!(output.contains("FmtOp::Span(0)"));
        // No more field descriptors — fields are accessed via Node::field()
        assert!(!output.contains("FIELDS_LITERAL"));
        // Dispatch table entry
        assert!(output.contains("NodeTag::Literal"));
    }

    #[test]
    fn compile_if_set_with_else() {
        let items = vec![
            Item::Node {
                name: "Test".into(),
                fields: vec![
                    Field { name: "child".into(), storage: Storage::Index, type_name: "Expr".into() },
                ],
                fmt: Some(vec![
                    Fmt::IfSet {
                        field: "child".into(),
                        then: vec![Fmt::Text("YES".into())],
                        els: Some(vec![Fmt::Text("NO".into())]),
                    },
                ]),
            },
        ];

        let output = generate_rust_fmt_ops(&items);
        assert!(output.contains("FmtOp::IfSet(0,"));
        assert!(output.contains("FmtOp::Else("));
        assert!(output.contains("FmtOp::EndIf"));
    }

    #[test]
    fn compile_switch() {
        let items = vec![
            Item::Enum {
                name: "MyOp".into(),
                variants: vec!["ADD".into(), "SUB".into()],
            },
            Item::Node {
                name: "Test".into(),
                fields: vec![
                    Field { name: "op".into(), storage: Storage::Inline, type_name: "MyOp".into() },
                ],
                fmt: Some(vec![
                    Fmt::Switch {
                        field: "op".into(),
                        cases: vec![
                            ("ADD".into(), vec![Fmt::Text("+".into())]),
                            ("SUB".into(), vec![Fmt::Text("-".into())]),
                        ],
                        default: None,
                    },
                ]),
            },
        ];

        let output = generate_rust_fmt_ops(&items);
        assert!(output.contains("FmtOp::IfEnum(0, 0,"));  // ADD = ordinal 0
        assert!(output.contains("FmtOp::IfEnum(0, 1,"));  // SUB = ordinal 1
    }

    #[test]
    fn compile_enum_display() {
        let items = vec![
            Item::Enum {
                name: "BinOp".into(),
                variants: vec!["PLUS".into(), "MINUS".into()],
            },
            Item::Node {
                name: "Test".into(),
                fields: vec![
                    Field { name: "op".into(), storage: Storage::Inline, type_name: "BinOp".into() },
                ],
                fmt: Some(vec![
                    Fmt::EnumDisplay {
                        field: "op".into(),
                        mappings: vec![
                            ("PLUS".into(), "+".into()),
                            ("MINUS".into(), "-".into()),
                        ],
                    },
                ]),
            },
        ];

        let output = generate_rust_fmt_ops(&items);
        assert!(output.contains("FmtOp::EnumDisplay(0,"));
        assert!(output.contains("ENUM_DISPLAY"));
    }

    #[test]
    fn compile_default_list() {
        let items = vec![
            Item::List {
                name: "ExprList".into(),
                child_type: "Expr".into(),
                fmt: None,
            },
        ];

        let output = generate_rust_fmt_ops(&items);
        assert!(output.contains("FMT_EXPR_LIST"));
        assert!(output.contains("FmtOp::ForEachSelfStart"));
        assert!(output.contains("FmtOp::ChildItem"));
        assert!(output.contains("FmtOp::ForEachEnd"));
    }

    #[test]
    fn compile_clause() {
        let items = vec![
            Item::Node {
                name: "Test".into(),
                fields: vec![
                    Field { name: "target".into(), storage: Storage::Index, type_name: "Expr".into() },
                ],
                fmt: Some(vec![
                    Fmt::Clause { keyword: "FROM".into(), field: "target".into() },
                ]),
            },
        ];

        let output = generate_rust_fmt_ops(&items);
        assert!(output.contains("FmtOp::IfSet(0,"));
        assert!(output.contains("FmtOp::NestStart(2)"));
        assert!(output.contains("FmtOp::Child(0)"));
        assert!(output.contains("FmtOp::NestEnd"));
    }

    #[test]
    fn dispatch_table_has_all_entries() {
        let items = vec![
            Item::Node {
                name: "Foo".into(),
                fields: vec![
                    Field { name: "x".into(), storage: Storage::Index, type_name: "Expr".into() },
                ],
                fmt: Some(vec![Fmt::Child("x".into())]),
            },
            Item::List {
                name: "FooList".into(),
                child_type: "Foo".into(),
                fmt: None,
            },
        ];
        let output = generate_rust_fmt_ops(&items);
        // Dispatch table with 3 entries (Null + Foo + FooList)
        assert!(output.contains("DISPATCH: [Option<NodeFmt>; 3]"));
        assert!(output.contains("NodeTag::Foo"));
        assert!(output.contains("NodeTag::FooList"));
    }

    #[test]
    fn bytecode_round_trip() {
        let items = vec![
            Item::Enum {
                name: "BinOp".into(),
                variants: vec!["PLUS".into(), "MINUS".into()],
            },
            Item::Node {
                name: "Literal".into(),
                fields: vec![Field {
                    name: "source".into(),
                    storage: Storage::Inline,
                    type_name: "SyntaqliteSourceSpan".into(),
                }],
                fmt: Some(vec![Fmt::Span("source".into())]),
            },
            Item::Node {
                name: "Test".into(),
                fields: vec![
                    Field { name: "op".into(), storage: Storage::Inline, type_name: "BinOp".into() },
                ],
                fmt: Some(vec![
                    Fmt::EnumDisplay {
                        field: "op".into(),
                        mappings: vec![
                            ("PLUS".into(), "+".into()),
                            ("MINUS".into(), "-".into()),
                        ],
                    },
                ]),
            },
            Item::List {
                name: "TestList".into(),
                child_type: "Test".into(),
                fmt: None,
            },
        ];

        let bytecode = generate_fmt_bytecode(&items);

        // Verify header
        assert_eq!(&bytecode[0..4], b"SQFM");
        let version = u16::from_le_bytes([bytecode[4], bytecode[5]]);
        assert_eq!(version, 1);
        let hash = u16::from_le_bytes([bytecode[6], bytecode[7]]);
        assert_eq!(hash, syntaqlite_runtime::fmt::bytecode_format::BYTECODE_VERSION_HASH);

        // Verify counts
        let string_count = u16::from_le_bytes([bytecode[8], bytecode[9]]);
        assert!(string_count > 0);
        let dispatch_count = u16::from_le_bytes([bytecode[14], bytecode[15]]);
        // 4 items (Literal, Test, TestList) + 1 (Null) = 4 tags
        // But Enum doesn't count, so: 3 nodes/lists + 1 Null = 4
        assert_eq!(dispatch_count, 4);
    }
}
