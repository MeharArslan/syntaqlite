// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Compiles .synq `Fmt` AST trees into `FmtOp` bytecode and emits them
//! as either generated Rust source code or binary bytecode.
//!
//! The generated Rust file contains:
//! - `STRINGS: &[&str]` — interned keywords/punctuation
//! - `ENUM_DISPLAY: &[u16]` — flat table mapping enum ordinals → `StringId`
//! - Per-node `FMT_XXX: &[FmtOp]` constant arrays
//! - `DISPATCH` table indexed by `NodeTag` ordinal
//!
//! The binary bytecode file contains the same data in a compact format
//! loadable at runtime (see `bytecode.rs` in syntaqlite-fmt).

use std::collections::HashMap;
use std::fmt::{Display, Formatter};

use super::AstModel;
use crate::util::rust_writer::RustWriter;
use crate::util::synq_parser::{Field, Fmt, Item};

const DEFAULT_NEST_INDENT: u16 = 2;

use syntaqlite_common::fmt::bytecode::RawOp;
use syntaqlite_common::fmt::bytecode::opcodes;

#[derive(Debug, Clone)]
pub(crate) enum FmtCompileError {
    FieldIndexTooLarge(u16),
    UnknownField(String),
    NonEnumField { field: String, type_name: String },
    UnknownEnumVariant { field: String, variant: String },
    MissingFlagBitName(String),
    UnknownFlagBit { type_name: String, bit: String },
    NonFlagsField { field: String, type_name: String },
    InvalidIfFlagField(String),
}

impl Display for FmtCompileError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FieldIndexTooLarge(idx) => write!(f, "field index {idx} too large for bytecode"),
            Self::UnknownField(name) => write!(f, "unknown field: {name}"),
            Self::NonEnumField { field, type_name } => {
                write!(f, "field {field} has non-enum type {type_name}")
            }
            Self::UnknownEnumVariant { field, variant } => {
                write!(f, "variant {variant} not found in enum for field {field}")
            }
            Self::MissingFlagBitName(field) => {
                write!(f, "flags field {field} requires `.bit_name`")
            }
            Self::UnknownFlagBit { type_name, bit } => {
                write!(f, "flag {bit} not found in {type_name}")
            }
            Self::NonFlagsField { field, type_name } => {
                write!(f, "field {field} has non-flags type {type_name}")
            }
            Self::InvalidIfFlagField(field) => {
                write!(
                    f,
                    "if_flag on field {field} which is neither Bool nor Flags"
                )
            }
        }
    }
}

impl std::error::Error for FmtCompileError {}

/// Convert a field index (u16) to u8 for binary encoding.
const fn idx_u8(idx: u16) -> Result<u8, FmtCompileError> {
    if idx >= 256 {
        return Err(FmtCompileError::FieldIndexTooLarge(idx));
    }
    // Safety: checked above that idx < 256
    #[allow(clippy::cast_possible_truncation)]
    let val = idx as u8;
    Ok(val)
}

const fn op0(opcode: u8) -> RawOp {
    RawOp::simple(opcode)
}

const fn opa(opcode: u8, a: u8) -> RawOp {
    RawOp {
        opcode,
        a,
        b: 0,
        c: 0,
    }
}

const fn opab(opcode: u8, a: u8, b: u16) -> RawOp {
    RawOp { opcode, a, b, c: 0 }
}

const fn opabc(opcode: u8, a: u8, b: u16, c: u16) -> RawOp {
    RawOp { opcode, a, b, c }
}

// ── String interning ────────────────────────────────────────────────────

struct StringTable {
    strings: Vec<String>,
    index: HashMap<String, u16>,
}

impl StringTable {
    fn new() -> Self {
        Self {
            strings: Vec::new(),
            index: HashMap::new(),
        }
    }

    fn intern(&mut self, s: &str) -> u16 {
        if let Some(&id) = self.index.get(s) {
            return id;
        }
        let id = u16::try_from(self.strings.len()).expect("string table exceeds u16");
        self.index.insert(s.to_string(), id);
        self.strings.push(s.to_string());
        id
    }
}

// ── Enum display table ──────────────────────────────────────────────────

struct EnumDisplayTable {
    /// Flat array of `StringId` values. Each `enum_display` block reserves
    /// `variant_count` consecutive slots starting at `base`.
    entries: Vec<u16>,
}

impl EnumDisplayTable {
    const fn new() -> Self {
        Self {
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
        let base = u16::try_from(self.entries.len()).expect("enum display table exceeds u16");
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
    /// Index into the `FieldVal` array for this node.
    idx: u16,
    /// The type name (e.g. "`SelectStmtFlags`", "`SortOrder`", "`SyntaqliteSourceSpan`").
    type_name: String,
}

/// Collect fields for a node, assigning sequential indices.
fn build_field_map(fields: &[Field]) -> (Vec<FieldInfo>, HashMap<String, usize>) {
    let mut infos = Vec::new();
    let mut name_to_idx = HashMap::new();
    for (i, f) in fields.iter().enumerate() {
        name_to_idx.insert(f.name.clone(), i);
        infos.push(FieldInfo {
            idx: u16::try_from(i).expect("field index exceeds u16"),
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

impl CompileCtx<'_> {
    fn field(&self, name: &str) -> Result<&FieldInfo, FmtCompileError> {
        let Some(idx) = self.field_map.get(name) else {
            return Err(FmtCompileError::UnknownField(name.to_string()));
        };
        Ok(&self.field_infos[*idx])
    }

    /// Resolve which enum type a field has (for `enum_display`, `if_enum`, switch).
    fn enum_variants(&self, field_name: &str) -> Result<&[String], FmtCompileError> {
        let info = self.field(field_name)?;
        let Some(variants) = self.enum_items.get(&info.type_name) else {
            return Err(FmtCompileError::NonEnumField {
                field: field_name.to_string(),
                type_name: info.type_name.clone(),
            });
        };
        Ok(variants)
    }

    /// Find the ordinal of a variant within an enum.
    fn enum_ordinal(&self, field_name: &str, variant: &str) -> Result<u16, FmtCompileError> {
        let variants = self.enum_variants(field_name)?;
        let Some(ordinal) = variants.iter().position(|v| v == variant) else {
            return Err(FmtCompileError::UnknownEnumVariant {
                field: field_name.to_string(),
                variant: variant.to_string(),
            });
        };
        Ok(u16::try_from(ordinal).expect("enum ordinal exceeds u16"))
    }

    /// Find the bit mask for a flag within a flags type, or handle Bool fields.
    fn flag_mask(&self, field_name: &str, bit_name: Option<&str>) -> Result<u8, FmtCompileError> {
        let info = self.field(field_name)?;
        if let Some(flags) = self.flags_items.get(&info.type_name) {
            let Some(bit) = bit_name else {
                return Err(FmtCompileError::MissingFlagBitName(field_name.to_string()));
            };
            let Some(mask) = flags
                .iter()
                .find(|(n, _)| n.to_lowercase() == bit.to_lowercase())
                .map(|(_, v)| u8::try_from(*v).expect("flag mask exceeds u8"))
            else {
                return Err(FmtCompileError::UnknownFlagBit {
                    type_name: info.type_name.clone(),
                    bit: bit.to_string(),
                });
            };
            Ok(mask)
        } else {
            Err(FmtCompileError::NonFlagsField {
                field: field_name.to_string(),
                type_name: info.type_name.clone(),
            })
        }
    }

    fn is_bool_field(&self, name: &str) -> Result<bool, FmtCompileError> {
        let info = self.field(name)?;
        Ok(info.type_name == "Bool")
    }

    fn is_flags_field(&self, name: &str) -> Result<bool, FmtCompileError> {
        let info = self.field(name)?;
        Ok(self.flags_items.contains_key(&info.type_name))
    }
}

/// Compile a sequence of Fmt nodes into `RawOps`.
fn compile_seq(
    fmts: &[Fmt],
    ctx: &mut CompileCtx<'_>,
    ops: &mut Vec<RawOp>,
) -> Result<(), FmtCompileError> {
    for fmt in fmts {
        compile_one(fmt, ctx, ops)?;
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn compile_one(
    fmt: &Fmt,
    ctx: &mut CompileCtx<'_>,
    ops: &mut Vec<RawOp>,
) -> Result<(), FmtCompileError> {
    match fmt {
        Fmt::Text(s) => {
            let sid = ctx.strings.intern(s);
            ops.push(opab(opcodes::KEYWORD, 0, sid));
        }
        Fmt::Child(field) if field == "_item" => {
            ops.push(op0(opcodes::CHILD_ITEM));
        }
        Fmt::Child(field) => {
            let info = ctx.field(field)?;
            ops.push(opa(opcodes::CHILD, idx_u8(info.idx)?));
        }
        Fmt::Span(field) => {
            let info = ctx.field(field)?;
            ops.push(opa(opcodes::SPAN, idx_u8(info.idx)?));
        }
        Fmt::Line => ops.push(op0(opcodes::LINE)),
        Fmt::SoftLine => ops.push(op0(opcodes::SOFTLINE)),
        Fmt::HardLine => ops.push(op0(opcodes::HARDLINE)),
        Fmt::Group(body) => {
            ops.push(op0(opcodes::GROUP_START));
            compile_seq(body, ctx, ops)?;
            ops.push(op0(opcodes::GROUP_END));
        }
        Fmt::Nest(body) => {
            ops.push(opab(opcodes::NEST_START, 0, DEFAULT_NEST_INDENT));
            compile_seq(body, ctx, ops)?;
            ops.push(op0(opcodes::NEST_END));
        }
        Fmt::IfSet { field, then, els } => {
            compile_field_conditional(opcodes::IF_SET, field, 0, then, els.as_deref(), ctx, ops)?;
        }
        Fmt::IfFlag {
            field,
            bit,
            then,
            els,
        } => {
            let base_field = field.as_str();
            if ctx.is_bool_field(base_field)? {
                compile_field_conditional(
                    opcodes::IF_BOOL,
                    base_field,
                    0,
                    then,
                    els.as_deref(),
                    ctx,
                    ops,
                )?;
            } else if ctx.is_flags_field(base_field)? {
                let mask = ctx.flag_mask(base_field, bit.as_deref())?;
                compile_field_conditional(
                    opcodes::IF_FLAG,
                    base_field,
                    u16::from(mask),
                    then,
                    els.as_deref(),
                    ctx,
                    ops,
                )?;
            } else {
                return Err(FmtCompileError::InvalidIfFlagField(field.clone()));
            }
        }
        Fmt::IfEnum {
            field,
            variant,
            then,
            els,
        } => {
            let ordinal = ctx.enum_ordinal(field, variant)?;
            compile_field_conditional(
                opcodes::IF_ENUM,
                field,
                ordinal,
                then,
                els.as_deref(),
                ctx,
                ops,
            )?;
        }
        Fmt::IfSpan { field, then, els } => {
            compile_field_conditional(opcodes::IF_SPAN, field, 0, then, els.as_deref(), ctx, ops)?;
        }
        Fmt::Clause { keyword, field } => {
            let field_idx = idx_u8(ctx.field(field)?.idx)?;
            compile_conditional(
                opabc(opcodes::IF_SET, field_idx, 0, 0),
                &[
                    Fmt::Line,
                    Fmt::Text(keyword.clone()),
                    Fmt::Nest(vec![Fmt::Line, Fmt::Child(field.clone())]),
                ],
                None,
                ctx,
                ops,
            )?;
        }
        Fmt::Switch {
            field,
            cases,
            default,
        } => {
            compile_switch(field, cases, default.as_deref(), ctx, ops)?;
        }
        Fmt::EnumDisplay { field, mappings } => {
            let field_idx = idx_u8(ctx.field(field)?.idx)?;
            let variants = ctx.enum_variants(field)?.to_vec();
            let base = ctx.enum_display.add(ctx.strings, &variants, mappings);
            ops.push(opab(opcodes::ENUM_DISPLAY, field_idx, base));
        }
        Fmt::ForEach { sep, body } => {
            ops.push(op0(opcodes::FOR_EACH_SELF_START));
            for item in body {
                compile_foreach_body_item(item, ctx, ops)?;
            }
            if let Some(sep_items) = sep {
                let mut emitted_sep = false;
                for s in sep_items {
                    if !emitted_sep {
                        if let Fmt::Text(text) = s {
                            let sid = ctx.strings.intern(text);
                            ops.push(opab(opcodes::FOR_EACH_SEP, 0, sid));
                            emitted_sep = true;
                            continue;
                        }
                        let sid = ctx.strings.intern("");
                        ops.push(opab(opcodes::FOR_EACH_SEP, 0, sid));
                        emitted_sep = true;
                    }
                    compile_foreach_body_item(s, ctx, ops)?;
                }
                if !emitted_sep {
                    let sid = ctx.strings.intern("");
                    ops.push(opab(opcodes::FOR_EACH_SEP, 0, sid));
                }
            }
            ops.push(op0(opcodes::FOR_EACH_END));
        }
    }
    Ok(())
}

fn compile_field_conditional(
    opcode: u8,
    field: &str,
    b: u16,
    then: &[Fmt],
    els: Option<&[Fmt]>,
    ctx: &mut CompileCtx<'_>,
    ops: &mut Vec<RawOp>,
) -> Result<(), FmtCompileError> {
    let idx = idx_u8(ctx.field(field)?.idx)?;
    compile_conditional(opabc(opcode, idx, b, 0), then, els, ctx, ops)
}

/// Compile a single item inside a `for_each` body, mapping `child(_item)` to `ChildItem`.
fn compile_foreach_body_item(
    fmt: &Fmt,
    ctx: &mut CompileCtx<'_>,
    ops: &mut Vec<RawOp>,
) -> Result<(), FmtCompileError> {
    match fmt {
        Fmt::Child(name) if name == "_item" => {
            ops.push(op0(opcodes::CHILD_ITEM));
            Ok(())
        }
        _ => compile_one(fmt, ctx, ops),
    }
}

/// Compile a conditional (`IfXxx` ... Else ... `EndIf`) with skip-count fixup.
/// `head` must have `c = 0`; it will be set to the computed skip count.
fn compile_conditional(
    head: RawOp,
    then: &[Fmt],
    els: Option<&[Fmt]>,
    ctx: &mut CompileCtx<'_>,
    ops: &mut Vec<RawOp>,
) -> Result<(), FmtCompileError> {
    let head_pos = ops.len();
    ops.push(head); // placeholder — c will be filled in

    // Compile then-branch
    let then_start = ops.len();
    compile_seq(then, ctx, ops)?;
    let then_len = ops.len() - then_start;

    if let Some(else_body) = els {
        // Add Else (placeholder)
        let else_pos = ops.len();
        ops.push(op0(opcodes::ELSE_OP)); // c filled below

        // Compile else-branch
        let else_start = ops.len();
        compile_seq(else_body, ctx, ops)?;
        let else_len = ops.len() - else_start;

        // EndIf
        ops.push(op0(opcodes::END_IF));

        // Fix up skip counts
        ops[head_pos].c = u16::try_from(then_len + 1).expect("skip count exceeds u16");
        ops[else_pos].c = u16::try_from(else_len + 1).expect("skip count exceeds u16");
    } else {
        // No else branch
        ops.push(op0(opcodes::END_IF));
        ops[head_pos].c = u16::try_from(then_len + 1).expect("skip count exceeds u16");
    }
    Ok(())
}

/// Compile a switch(field) { VARIANT { ... } ... default { ... } } into chained `IfEnum` blocks.
fn compile_switch(
    field: &str,
    cases: &[(String, Vec<Fmt>)],
    default: Option<&[Fmt]>,
    ctx: &mut CompileCtx<'_>,
    ops: &mut Vec<RawOp>,
) -> Result<(), FmtCompileError> {
    let field_idx = idx_u8(ctx.field(field)?.idx)?;

    #[allow(clippy::items_after_statements)]
    struct CaseChunk {
        ordinal: u16,
        body_ops: Vec<RawOp>,
    }
    let mut chunks: Vec<CaseChunk> = Vec::new();
    for (variant, body) in cases {
        let ordinal = ctx.enum_ordinal(field, variant)?;
        let mut body_ops = Vec::new();
        compile_seq(body, ctx, &mut body_ops)?;
        chunks.push(CaseChunk { ordinal, body_ops });
    }

    let mut default_ops = Vec::new();
    if let Some(def) = default {
        compile_seq(def, ctx, &mut default_ops)?;
    }

    #[allow(clippy::items_after_statements)]
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
            ops.push(opabc(
                opcodes::IF_ENUM,
                field_idx,
                chunk.ordinal,
                u16::try_from(then_len + 1).expect("skip count exceeds u16"),
            ));
            for op in &chunk.body_ops {
                ops.push(*op);
            }
            ops.push(opabc(
                opcodes::ELSE_OP,
                0,
                0,
                u16::try_from(else_ops.len() + 1).expect("skip count exceeds u16"),
            ));
            for op in &else_ops {
                ops.push(*op);
            }
            ops.push(op0(opcodes::END_IF));
        } else {
            ops.push(opabc(
                opcodes::IF_ENUM,
                field_idx,
                chunk.ordinal,
                u16::try_from(then_len + 1).expect("skip count exceeds u16"),
            ));
            for op in &chunk.body_ops {
                ops.push(*op);
            }
            ops.push(op0(opcodes::END_IF));
        }
    }

    emit_chain(field_idx, &chunks, &default_ops, ops);
    Ok(())
}

// ── Shared compilation ──────────────────────────────────────────────────

pub(crate) struct CompiledNode {
    pub name: String,
    pub ops: Vec<RawOp>,
}

pub(crate) struct CompiledFmt {
    pub strings: Vec<String>,
    pub enum_display: Vec<u16>,
    pub nodes: Vec<CompiledNode>,
    pub tag_count: usize,
}

/// Compile all items into the intermediate representation shared by both emitters.
/// Uses the model's `all_items()` to include both base and extension items, and
/// `node_like_items()` for the correct total tag count.
pub(crate) fn try_compile_all(model: &AstModel<'_>) -> Result<CompiledFmt, FmtCompileError> {
    let items: Vec<&Item> = model.all_items().collect();

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

    for item in &items {
        match item {
            Item::Node {
                name,
                fields,
                fmt: Some(fmt_body),
                ..
            } => compiled.push(compile_named_fmt(
                name,
                fields,
                fmt_body,
                &mut strings,
                &mut enum_display,
                &enum_items,
                &flags_items,
            )?),
            Item::List {
                name,
                fmt: Some(fmt_body),
                ..
            } => compiled.push(compile_named_fmt(
                name,
                &[],
                fmt_body,
                &mut strings,
                &mut enum_display,
                &enum_items,
                &flags_items,
            )?),
            Item::List {
                name, fmt: None, ..
            } => {
                let comma_sid = strings.intern(",");
                let ops = vec![
                    op0(opcodes::FOR_EACH_SELF_START),
                    op0(opcodes::CHILD_ITEM),
                    opab(opcodes::FOR_EACH_SEP, 0, comma_sid),
                    op0(opcodes::LINE),
                    op0(opcodes::FOR_EACH_END),
                ];
                compiled.push(CompiledNode {
                    name: name.clone(),
                    ops,
                });
            }
            _ => {}
        }
    }

    // Use model.node_like_items() for the correct total count (base + extension).
    let tag_count = model.node_like_items().len() + 1;

    Ok(CompiledFmt {
        strings: strings.strings,
        enum_display: enum_display.entries,
        nodes: compiled,
        tag_count,
    })
}

fn compile_named_fmt(
    name: &str,
    fields: &[Field],
    fmt_body: &[Fmt],
    strings: &mut StringTable,
    enum_display: &mut EnumDisplayTable,
    enum_items: &HashMap<String, Vec<String>>,
    flags_items: &HashMap<String, Vec<(String, u32)>>,
) -> Result<CompiledNode, FmtCompileError> {
    let (field_infos, field_map) = build_field_map(fields);
    let mut ops = Vec::new();
    let mut cctx = CompileCtx {
        strings,
        enum_display,
        field_infos: &field_infos,
        field_map: &field_map,
        enum_items,
        flags_items,
    };
    compile_seq(fmt_body, &mut cctx, &mut ops)?;
    Ok(CompiledNode {
        name: name.to_string(),
        ops,
    })
}

// ── Rust static generation ──────────────────────────────────────────────────

/// Generate a Rust source file containing the four formatter statics for a dialect.
///
/// Produces `{prefix}_FMT_STRINGS`, `{prefix}_FMT_ENUM_DISPLAY`,
/// `{prefix}_FMT_OPS`, and `{prefix}_FMT_DISPATCH`.
///
/// The statics use the packed 6-byte-per-instruction binary format expected
/// by `AnyDialect::fmt_dispatch` / `op_at` in `fmt/interpret.rs`.
pub(crate) fn generate_rust_fmt_statics(
    model: &AstModel<'_>,
    prefix: &str,
) -> Result<String, FmtCompileError> {
    let compiled = try_compile_all(model)?;

    // ── Flatten ops into a packed byte stream ────────────────────────────
    let mut flat_ops: Vec<u8> = Vec::new();
    let mut name_to_dispatch: HashMap<String, (u16, u16)> = HashMap::new();

    for node in &compiled.nodes {
        let offset = u16::try_from(flat_ops.len() / 6).expect("fmt op offset fits u16");
        let length = u16::try_from(node.ops.len()).expect("fmt op length fits u16");
        for op in &node.ops {
            flat_ops.push(op.opcode);
            flat_ops.push(op.a);
            flat_ops.extend_from_slice(&op.b.to_le_bytes());
            flat_ops.extend_from_slice(&op.c.to_le_bytes());
        }
        name_to_dispatch.insert(node.name.clone(), (offset, length));
    }

    // ── Build dispatch table indexed by node tag ─────────────────────────
    // Default: sentinel — offset=0xFFFF means no formatter ops for this tag.
    let tag_count = compiled.tag_count;
    let mut dispatch_table: Vec<u32> = vec![0xFFFF_0000u32; tag_count];
    for node_like in model.node_like_items() {
        let name = node_like.name();
        if let Some(&(offset, length)) = name_to_dispatch.get(name) {
            let tag = model.tag_for(name) as usize;
            if tag < tag_count {
                dispatch_table[tag] = (u32::from(offset) << 16) | u32::from(length);
            }
        }
    }

    // ── Emit Rust source ─────────────────────────────────────────────────
    let mut w = RustWriter::new();
    w.file_header();
    w.line(&format!(
        "//! Formatter string table and opcode data for the `{prefix}` dialect."
    ));
    w.newline();

    // String table
    w.line(&format!("/// String table for the `{prefix}` formatter."));
    w.line(&format!(
        "pub(crate) static {prefix}_FMT_STRINGS: &[&str] = &["
    ));
    for s in &compiled.strings {
        w.line(&format!("    {s:?},"));
    }
    w.line("];");
    w.newline();

    // Enum display table
    w.line(&format!(
        "/// Enum display table for the `{prefix}` formatter."
    ));
    w.line(&format!(
        "pub(crate) static {prefix}_FMT_ENUM_DISPLAY: &[u16] = &["
    ));
    for chunk in compiled.enum_display.chunks(16) {
        let row: Vec<String> = chunk.iter().map(|v| v.to_string()).collect();
        w.line(&format!("    {},", row.join(", ")));
    }
    w.line("];");
    w.newline();

    // Packed opcode stream (6 bytes per instruction)
    w.line("/// Packed formatter opcode stream (6 bytes per instruction).");
    w.line(&format!("pub(crate) static {prefix}_FMT_OPS: &[u8] = &["));
    for chunk in flat_ops.chunks(6) {
        let row: Vec<String> = chunk.iter().map(|b| format!("0x{b:02X}")).collect();
        w.line(&format!("    {},", row.join(", ")));
    }
    w.line("];");
    w.newline();

    // Dispatch table
    w.line("/// Packed dispatch table: each `u32` encodes `(offset << 16) | length`.");
    w.line(&format!(
        "pub(crate) static {prefix}_FMT_DISPATCH: &[u32] = &["
    ));
    for (tag, &packed) in dispatch_table.iter().enumerate() {
        w.line(&format!("    0x{packed:08X}, // tag {tag}"));
    }
    w.line("];");

    Ok(w.finish())
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;

    use super::*;
    use crate::util::synq_parser::Storage;
    use crate::util::upper_snake;

    fn raw_op_to_string(op: RawOp) -> String {
        match op.opcode {
            opcodes::KEYWORD => format!("FmtOp::Keyword({})", op.b),
            opcodes::SPAN => format!("FmtOp::Span({})", op.a),
            opcodes::CHILD => format!("FmtOp::Child({})", op.a),
            opcodes::LINE => "FmtOp::Line".to_string(),
            opcodes::SOFTLINE => "FmtOp::SoftLine".to_string(),
            opcodes::HARDLINE => "FmtOp::HardLine".to_string(),
            opcodes::GROUP_START => "FmtOp::GroupStart".to_string(),
            opcodes::GROUP_END => "FmtOp::GroupEnd".to_string(),
            opcodes::NEST_START => format!(
                "FmtOp::NestStart({})",
                i16::try_from(op.b).expect("nest indent fits i16")
            ),
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

    fn generate_rust_fmt_ops(model: &AstModel<'_>) -> Result<String, FmtCompileError> {
        let compiled = try_compile_all(model)?;

        let mut out = String::new();
        writeln!(out, "// @generated by syntaqlite-buildtools — DO NOT EDIT").unwrap();
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
        for s in &compiled.strings {
            writeln!(out, "    {s:?},").unwrap();
        }
        writeln!(out, "];").unwrap();
        writeln!(out).unwrap();

        // Enum display table
        writeln!(out, "const ENUM_DISPLAY: &[u16] = &[").unwrap();
        for &sid in &compiled.enum_display {
            write!(out, "{sid}, ").unwrap();
        }
        writeln!(out, "];").unwrap();
        writeln!(out).unwrap();

        // Per-node: const ops array
        for cn in &compiled.nodes {
            let upper = upper_snake(&cn.name);
            writeln!(out, "const FMT_{upper}: &[FmtOp] = &[").unwrap();
            for op in &cn.ops {
                writeln!(out, "    {},", raw_op_to_string(*op)).unwrap();
            }
            writeln!(out, "];").unwrap();
            writeln!(out).unwrap();
        }

        // Dispatch table
        writeln!(
            out,
            "pub const DISPATCH: [Option<NodeFmt>; {}] = {{",
            compiled.tag_count
        )
        .unwrap();
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

        Ok(out)
    }

    #[test]
    fn compile_simple_keyword() {
        let items = vec![Item::Node {
            name: "Literal".into(),
            fields: vec![Field {
                name: "source".into(),
                storage: Storage::Inline,
                type_name: "SyntaqliteSourceSpan".into(),
            }],
            fmt: Some(vec![Fmt::Span("source".into())]),
            semantic: None,
        }];

        let output = generate_rust_fmt_ops(&AstModel::new(&items)).unwrap();
        assert!(output.contains("FMT_LITERAL"));
        assert!(output.contains("FmtOp::Span(0)"));
        // No more field descriptors — fields are accessed via Node::field()
        assert!(!output.contains("FIELDS_LITERAL"));
        // Dispatch table entry
        assert!(output.contains("NodeTag::Literal"));
    }

    #[test]
    fn compile_if_set_with_else() {
        let items = vec![Item::Node {
            name: "Test".into(),
            fields: vec![Field {
                name: "child".into(),
                storage: Storage::Index,
                type_name: "Expr".into(),
            }],
            fmt: Some(vec![Fmt::IfSet {
                field: "child".into(),
                then: vec![Fmt::Text("YES".into())],
                els: Some(vec![Fmt::Text("NO".into())]),
            }]),
            semantic: None,
        }];

        let output = generate_rust_fmt_ops(&AstModel::new(&items)).unwrap();
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
                fields: vec![Field {
                    name: "op".into(),
                    storage: Storage::Inline,
                    type_name: "MyOp".into(),
                }],
                fmt: Some(vec![Fmt::Switch {
                    field: "op".into(),
                    cases: vec![
                        ("ADD".into(), vec![Fmt::Text("+".into())]),
                        ("SUB".into(), vec![Fmt::Text("-".into())]),
                    ],
                    default: None,
                }]),
                semantic: None,
            },
        ];

        let output = generate_rust_fmt_ops(&AstModel::new(&items)).unwrap();
        assert!(output.contains("FmtOp::IfEnum(0, 0,")); // ADD = ordinal 0
        assert!(output.contains("FmtOp::IfEnum(0, 1,")); // SUB = ordinal 1
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
                fields: vec![Field {
                    name: "op".into(),
                    storage: Storage::Inline,
                    type_name: "BinOp".into(),
                }],
                fmt: Some(vec![Fmt::EnumDisplay {
                    field: "op".into(),
                    mappings: vec![("PLUS".into(), "+".into()), ("MINUS".into(), "-".into())],
                }]),
                semantic: None,
            },
        ];

        let output = generate_rust_fmt_ops(&AstModel::new(&items)).unwrap();
        assert!(output.contains("FmtOp::EnumDisplay(0,"));
        assert!(output.contains("ENUM_DISPLAY"));
    }

    #[test]
    fn compile_default_list() {
        let items = vec![Item::List {
            name: "ExprList".into(),
            child_type: "Expr".into(),
            fmt: None,
        }];

        let output = generate_rust_fmt_ops(&AstModel::new(&items)).unwrap();
        assert!(output.contains("FMT_EXPR_LIST"));
        assert!(output.contains("FmtOp::ForEachSelfStart"));
        assert!(output.contains("FmtOp::ChildItem"));
        assert!(output.contains("FmtOp::ForEachEnd"));
    }

    #[test]
    fn compile_clause() {
        let items = vec![Item::Node {
            name: "Test".into(),
            fields: vec![Field {
                name: "target".into(),
                storage: Storage::Index,
                type_name: "Expr".into(),
            }],
            fmt: Some(vec![Fmt::Clause {
                keyword: "FROM".into(),
                field: "target".into(),
            }]),
            semantic: None,
        }];

        let output = generate_rust_fmt_ops(&AstModel::new(&items)).unwrap();
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
                fields: vec![Field {
                    name: "x".into(),
                    storage: Storage::Index,
                    type_name: "Expr".into(),
                }],
                fmt: Some(vec![Fmt::Child("x".into())]),
                semantic: None,
            },
            Item::List {
                name: "FooList".into(),
                child_type: "Foo".into(),
                fmt: None,
            },
        ];
        let output = generate_rust_fmt_ops(&AstModel::new(&items)).unwrap();
        // Dispatch table with 3 entries (Null + Foo + FooList)
        assert!(output.contains("DISPATCH: [Option<NodeFmt>; 3]"));
        assert!(output.contains("NodeTag::Foo"));
        assert!(output.contains("NodeTag::FooList"));
    }

    // ── generate_rust_fmt_statics tests ─────────────────────────────────────

    #[test]
    fn fmt_statics_has_all_four_statics() {
        let items = vec![Item::Node {
            name: "Literal".into(),
            fields: vec![Field {
                name: "source".into(),
                storage: Storage::Inline,
                type_name: "SyntaqliteSourceSpan".into(),
            }],
            fmt: Some(vec![Fmt::Span("source".into())]),
            semantic: None,
        }];
        let model = AstModel::new(&items);
        let out = generate_rust_fmt_statics(&model, "TEST").unwrap();
        assert!(out.contains("TEST_FMT_STRINGS"), "missing strings static");
        assert!(
            out.contains("TEST_FMT_ENUM_DISPLAY"),
            "missing enum_display static"
        );
        assert!(out.contains("TEST_FMT_OPS"), "missing ops static");
        assert!(out.contains("TEST_FMT_DISPATCH"), "missing dispatch static");
    }

    #[test]
    fn fmt_statics_span_op_bytes_are_correct() {
        // Span(field=0) → opcode=SPAN(1), a=0, b=0, c=0 → bytes 0x01,0x00,0x00,0x00,0x00,0x00
        use syntaqlite_common::fmt::bytecode::opcodes;
        let items = vec![Item::Node {
            name: "Lit".into(),
            fields: vec![Field {
                name: "s".into(),
                storage: Storage::Inline,
                type_name: "SyntaqliteSourceSpan".into(),
            }],
            fmt: Some(vec![Fmt::Span("s".into())]),
            semantic: None,
        }];
        let model = AstModel::new(&items);
        let out = generate_rust_fmt_statics(&model, "TEST").unwrap();
        let expected = format!("0x{:02X}, 0x00, 0x00, 0x00, 0x00, 0x00", opcodes::SPAN);
        assert!(
            out.contains(&expected),
            "expected bytes {expected} in:\n{out}"
        );
    }

    #[test]
    fn fmt_statics_dispatch_table_size_equals_tag_count() {
        // 2 nodes + 1 list → tag_count = 4 (0..=3), dispatch has 4 entries
        let items = vec![
            Item::Node {
                name: "A".into(),
                fields: vec![],
                fmt: Some(vec![]),
                semantic: None,
            },
            Item::Node {
                name: "B".into(),
                fields: vec![],
                fmt: Some(vec![]),
                semantic: None,
            },
            Item::List {
                name: "AList".into(),
                child_type: "A".into(),
                fmt: None,
            },
        ];
        let model = AstModel::new(&items);
        let out = generate_rust_fmt_statics(&model, "TEST").unwrap();
        // Dispatch table should have entries for tags 0,1,2,3
        assert!(out.contains("// tag 0"), "missing tag 0");
        assert!(out.contains("// tag 1"), "missing tag 1");
        assert!(out.contains("// tag 2"), "missing tag 2");
        assert!(out.contains("// tag 3"), "missing tag 3");
        // Should NOT have tag 4
        assert!(!out.contains("// tag 4"), "unexpected tag 4");
    }

    #[test]
    fn fmt_statics_tag0_is_always_sentinel() {
        let items = vec![Item::Node {
            name: "Foo".into(),
            fields: vec![],
            fmt: Some(vec![]),
            semantic: None,
        }];
        let model = AstModel::new(&items);
        let out = generate_rust_fmt_statics(&model, "TEST").unwrap();
        // Tag 0 must be the sentinel 0xFFFF0000
        assert!(
            out.contains("0xFFFF0000, // tag 0"),
            "tag 0 must be sentinel, got:\n{out}"
        );
    }

    #[test]
    fn fmt_statics_node_without_fmt_gets_sentinel() {
        // Node with no fmt block → no ops compiled → dispatch entry stays sentinel
        let items = vec![
            Item::Node {
                name: "WithFmt".into(),
                fields: vec![Field {
                    name: "s".into(),
                    storage: Storage::Inline,
                    type_name: "SyntaqliteSourceSpan".into(),
                }],
                fmt: Some(vec![Fmt::Span("s".into())]),
                semantic: None,
            },
            Item::Node {
                name: "NoFmt".into(),
                fields: vec![],
                fmt: None,
                semantic: None,
            },
        ];
        let model = AstModel::new(&items);
        let out = generate_rust_fmt_statics(&model, "TEST").unwrap();
        // NoFmt is tag 2; its entry must be the sentinel
        let lines: Vec<&str> = out.lines().collect();
        let tag2_line = lines.iter().find(|l| l.contains("// tag 2")).unwrap();
        assert!(
            tag2_line.contains("0xFFFF0000"),
            "NoFmt (tag 2) should be sentinel, got: {tag2_line}"
        );
    }

    #[test]
    fn fmt_statics_keyword_in_string_table() {
        let items = vec![Item::Node {
            name: "Kw".into(),
            fields: vec![],
            fmt: Some(vec![Fmt::Text("SELECT".into())]),
            semantic: None,
        }];
        let model = AstModel::new(&items);
        let out = generate_rust_fmt_statics(&model, "TEST").unwrap();
        assert!(
            out.contains("\"SELECT\""),
            "string table should contain SELECT"
        );
    }
}
