//! Compiles .synq `Fmt` AST trees into FmtOp bytecode arrays and emits them
//! as generated Rust source code.
//!
//! The generated file contains:
//! - `STRINGS: &[&str]` — interned keywords/punctuation
//! - `ENUM_DISPLAY: &[u16]` — flat table mapping enum ordinals → StringId
//! - Per-node `FMT_XXX: &[FmtOp]` constant arrays
//! - Per-node `extract_xxx()` functions that produce `&[FieldVal]`
//! - `DISPATCH` table indexed by NodeTag ordinal

use std::collections::HashMap;
use std::fmt::Write as _;

#[cfg(test)]
use crate::node_parser::Storage;
use crate::node_parser::{Field, Fmt, Item};

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

// ── Fmt → FmtOp compilation ────────────────────────────────────────────

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

/// Compile a sequence of Fmt nodes into FmtOp string representations.
fn compile_seq(fmts: &[Fmt], ctx: &mut CompileCtx, ops: &mut Vec<String>) {
    for fmt in fmts {
        compile_one(fmt, ctx, ops);
    }
}

fn compile_one(fmt: &Fmt, ctx: &mut CompileCtx, ops: &mut Vec<String>) {
    match fmt {
        Fmt::Text(s) => {
            let sid = ctx.strings.intern(s);
            ops.push(format!("FmtOp::Keyword({})", sid));
        }
        Fmt::Child(field) if field == "_item" => {
            ops.push("FmtOp::ChildItem".to_string());
        }
        Fmt::Child(field) => {
            let info = ctx.field(field);
            ops.push(format!("FmtOp::Child({})", info.idx));
        }
        Fmt::Span(field) => {
            let info = ctx.field(field);
            ops.push(format!("FmtOp::Span({})", info.idx));
        }
        Fmt::Line => ops.push("FmtOp::Line".to_string()),
        Fmt::SoftLine => ops.push("FmtOp::SoftLine".to_string()),
        Fmt::HardLine => ops.push("FmtOp::HardLine".to_string()),
        Fmt::Group(body) => {
            ops.push("FmtOp::GroupStart".to_string());
            compile_seq(body, ctx, ops);
            ops.push("FmtOp::GroupEnd".to_string());
        }
        Fmt::Nest(body) => {
            ops.push("FmtOp::NestStart(2)".to_string());
            compile_seq(body, ctx, ops);
            ops.push("FmtOp::NestEnd".to_string());
        }
        Fmt::IfSet { field, then, els } => {
            compile_conditional(
                &format!("FmtOp::IfSet({}, {{SKIP}})", ctx.field(field).idx),
                then, els.as_deref(), ctx, ops,
            );
        }
        Fmt::IfFlag { field, bit, then, els } => {
            // if_flag can be used on Bool fields (no dot) or Flags fields (with dot)
            let base_field = field.as_str();
            if ctx.is_bool_field(base_field) {
                let info = ctx.field(base_field);
                compile_conditional(
                    &format!("FmtOp::IfBool({}, {{SKIP}})", info.idx),
                    then, els.as_deref(), ctx, ops,
                );
            } else if ctx.is_flags_field(base_field) {
                let info = ctx.field(base_field);
                let mask = ctx.flag_mask(base_field, bit.as_deref());
                compile_conditional(
                    &format!("FmtOp::IfFlag({}, {}, {{SKIP}})", info.idx, mask),
                    then, els.as_deref(), ctx, ops,
                );
            } else {
                panic!("if_flag on field {} which is neither Bool nor Flags", field);
            }
        }
        Fmt::IfEnum { field, variant, then, els } => {
            let info = ctx.field(field);
            let ordinal = ctx.enum_ordinal(field, variant);
            compile_conditional(
                &format!("FmtOp::IfEnum({}, {}, {{SKIP}})", info.idx, ordinal),
                then, els.as_deref(), ctx, ops,
            );
        }
        Fmt::IfSpan { field, then, els } => {
            let info = ctx.field(field);
            compile_conditional(
                &format!("FmtOp::IfSpan({}, {{SKIP}})", info.idx),
                then, els.as_deref(), ctx, ops,
            );
        }
        Fmt::Clause { keyword, field } => {
            // clause("KW", field) expands to:
            //   IfSet(field, skip) Line Keyword("KW") NestStart(2) Line Child(field) NestEnd EndIf
            let field_idx = ctx.field(field).idx;
            compile_conditional(
                &format!("FmtOp::IfSet({}, {{SKIP}})", field_idx),
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
            // Compile as a chain of IfEnum/Else/EndIf blocks.
            compile_switch(field, cases, default.as_deref(), ctx, ops);
        }
        Fmt::EnumDisplay { field, mappings } => {
            let field_idx = ctx.field(field).idx;
            let variants = ctx.enum_variants(field).to_vec();
            let base = ctx.enum_display.add(ctx.strings, &variants, mappings);
            ops.push(format!("FmtOp::EnumDisplay({}, {})", field_idx, base));
        }
        Fmt::ForEach { sep, body } => {
            ops.push("FmtOp::ForEachSelfStart".to_string());
            // The body uses `child(_item)` which compiles to ChildItem
            for item in body {
                compile_foreach_body_item(item, ctx, ops);
            }
            if let Some(sep_items) = sep {
                // The first keyword/text becomes ForEachSep which handles
                // the last-item check: on last iteration, it skips to
                // ForEachEnd, also skipping any remaining separator items.
                let mut emitted_sep = false;
                for s in sep_items {
                    if !emitted_sep {
                        match s {
                            Fmt::Text(text) => {
                                let sid = ctx.strings.intern(text);
                                ops.push(format!("FmtOp::ForEachSep({})", sid));
                                emitted_sep = true;
                                continue;
                            }
                            _ => {
                                // Non-keyword first (e.g. hardline): insert
                                // ForEachSep with empty string for skip logic
                                let sid = ctx.strings.intern("");
                                ops.push(format!("FmtOp::ForEachSep({})", sid));
                                emitted_sep = true;
                            }
                        }
                    }
                    compile_foreach_body_item(s, ctx, ops);
                }
                if !emitted_sep {
                    let sid = ctx.strings.intern("");
                    ops.push(format!("FmtOp::ForEachSep({})", sid));
                }
            }
            ops.push("FmtOp::ForEachEnd".to_string());
        }
    }
}

/// Compile a single item inside a for_each body, mapping `child(_item)` to `ChildItem`.
fn compile_foreach_body_item(fmt: &Fmt, ctx: &mut CompileCtx, ops: &mut Vec<String>) {
    match fmt {
        Fmt::Child(name) if name == "_item" => {
            ops.push("FmtOp::ChildItem".to_string());
        }
        _ => compile_one(fmt, ctx, ops),
    }
}

/// Compile a conditional (IfXxx ... Else ... EndIf) with skip-count fixup.
/// `head_template` must contain `{SKIP}` which gets replaced with the actual skip count.
fn compile_conditional(
    head_template: &str,
    then: &[Fmt],
    els: Option<&[Fmt]>,
    ctx: &mut CompileCtx,
    ops: &mut Vec<String>,
) {
    let head_pos = ops.len();
    ops.push(String::new()); // placeholder for head

    // Compile then-branch
    let then_start = ops.len();
    compile_seq(then, ctx, ops);
    let then_len = ops.len() - then_start;

    if let Some(else_body) = els {
        // Add Else (placeholder)
        let else_pos = ops.len();
        ops.push(String::new()); // placeholder for Else

        // Compile else-branch
        let else_start = ops.len();
        compile_seq(else_body, ctx, ops);
        let else_len = ops.len() - else_start;

        // EndIf
        ops.push("FmtOp::EndIf".to_string());

        // Fix up: head skip = then_len + 1 (to skip then-body + Else op)
        ops[head_pos] = head_template.replace("{SKIP}", &(then_len + 1).to_string());
        // Fix up: Else skip = else_len + 1 (to skip else-body + EndIf)
        ops[else_pos] = format!("FmtOp::Else({})", else_len + 1);
    } else {
        // No else branch
        ops.push("FmtOp::EndIf".to_string());

        // Fix up: head skip = then_len + 1 (skip then-body + EndIf)
        ops[head_pos] = head_template.replace("{SKIP}", &(then_len + 1).to_string());
    }
}

/// Compile a switch(field) { VARIANT { ... } ... default { ... } } into chained IfEnum blocks.
fn compile_switch(
    field: &str,
    cases: &[(String, Vec<Fmt>)],
    default: Option<&[Fmt]>,
    ctx: &mut CompileCtx,
    ops: &mut Vec<String>,
) {
    // Strategy: compile each case as IfEnum(field, ordinal, skip) ... then
    // the next case follows. At the end, the default (if any).
    //
    // We need skip counts that jump to the end of the entire switch.
    // Approach: compile all case bodies, then fix up skips.

    let field_idx = ctx.field(field).idx;

    // Collect compiled case bodies first to measure sizes
    struct CaseChunk {
        ordinal: u16,
        body_ops: Vec<String>,
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

    // Compile as nested if-else chain:
    // IfEnum(case0) { body0 } else { IfEnum(case1) { body1 } else { ... default ... } }

    fn emit_chain(
        field_idx: u16,
        chunks: &[CaseChunk],
        default_ops: &[String],
        ops: &mut Vec<String>,
    ) {
        if chunks.is_empty() {
            for op in default_ops {
                ops.push(op.clone());
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
            ops.push(format!("FmtOp::IfEnum({}, {}, {})", field_idx, chunk.ordinal, then_len + 1));
            for op in &chunk.body_ops {
                ops.push(op.clone());
            }
            ops.push(format!("FmtOp::Else({})", else_ops.len() + 1));
            for op in &else_ops {
                ops.push(op.clone());
            }
            ops.push("FmtOp::EndIf".to_string());
        } else {
            ops.push(format!("FmtOp::IfEnum({}, {}, {})", field_idx, chunk.ordinal, then_len + 1));
            for op in &chunk.body_ops {
                ops.push(op.clone());
            }
            ops.push("FmtOp::EndIf".to_string());
        }
    }

    emit_chain(field_idx, &chunks, &default_ops, ops);
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

/// Generate the complete `fmt_ops.rs` file.
pub fn generate_rust_fmt_ops(items: &[Item]) -> String {
    // Build enum variant maps (name → ordered variants)
    let enum_items: HashMap<String, Vec<String>> = items
        .iter()
        .filter_map(|item| match item {
            Item::Enum { name, variants } => Some((name.clone(), variants.clone())),
            _ => None,
        })
        .collect();

    // Build flags maps
    let flags_items: HashMap<String, Vec<(String, u32)>> = items
        .iter()
        .filter_map(|item| match item {
            Item::Flags { name, flags } => Some((name.clone(), flags.clone())),
            _ => None,
        })
        .collect();

    let mut strings = StringTable::new();
    let mut enum_display = EnumDisplayTable::new();

    // Compile all nodes and lists with fmt blocks
    struct CompiledNode {
        name: String,
        ops: Vec<String>,
    }
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
                compiled.push(CompiledNode {
                    name: name.clone(),
                    ops,
                });
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
                compiled.push(CompiledNode {
                    name: name.clone(),
                    ops,
                });
            }
            Item::List { name, fmt: None, .. } => {
                // Default list fmt: for_each(sep: ",") { child(_item) line }
                let comma_sid = strings.intern(",");
                let ops = vec![
                    "FmtOp::ForEachSelfStart".to_string(),
                    "FmtOp::ChildItem".to_string(),
                    format!("FmtOp::ForEachSep({})", comma_sid),
                    "FmtOp::Line".to_string(),
                    "FmtOp::ForEachEnd".to_string(),
                ];
                compiled.push(CompiledNode {
                    name: name.clone(),
                    ops,
                });
            }
            _ => {}
        }
    }

    // Count total tags for dispatch table (nodes + lists + Null)
    let tag_count = items
        .iter()
        .filter(|i| matches!(i, Item::Node { .. } | Item::List { .. }))
        .count()
        + 1;

    // -- Emit Rust source --
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
    for s in &strings.strings {
        writeln!(out, "    {:?},", s).unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out).unwrap();

    // Enum display table
    writeln!(out, "const ENUM_DISPLAY: &[u16] = &[").unwrap();
    for &sid in &enum_display.entries {
        write!(out, "{}, ", sid).unwrap();
    }
    writeln!(out, "];").unwrap();
    writeln!(out).unwrap();

    // Per-node: const ops array
    for cn in &compiled {
        let upper = upper_snake(&cn.name);

        // Ops array
        writeln!(out, "const FMT_{}: &[FmtOp] = &[", upper).unwrap();
        for op in &cn.ops {
            writeln!(out, "    {},", op).unwrap();
        }
        writeln!(out, "];").unwrap();
        writeln!(out).unwrap();
    }

    // Dispatch table
    writeln!(out, "pub const DISPATCH: [Option<NodeFmt>; {}] = {{", tag_count).unwrap();
    writeln!(out, "    const NONE: Option<NodeFmt> = None;").unwrap();
    writeln!(out, "    let mut t = [NONE; {}];", tag_count).unwrap();
    for cn in &compiled {
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
}
