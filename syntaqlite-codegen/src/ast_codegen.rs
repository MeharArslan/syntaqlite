// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;

use crate::c_writer::CWriter;
use crate::node_parser::{Field, Item, Storage};
use crate::rust_writer::RustWriter;

// ── Public API ──────────────────────────────────────────────────────────

pub fn generate_ast_nodes_h(items: &[Item], dialect: &str) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = CWriter::new();

    w.file_header();
    let guard = format!("SYNTAQLITE_{}_NODE_H", dialect.to_uppercase());
    w.header_guard_start(&guard);
    w.include_system("stddef.h");
    w.include_system("stdint.h");
    w.newline();
    w.include_local("syntaqlite/types.h");
    w.newline();
    // C++ doesn't have flexible array members; use [1] as a common workaround.
    w.line("#ifdef __cplusplus");
    w.line("#define SYNTAQLITE_FLEXIBLE_ARRAY 1");
    w.line("#else");
    w.line("#define SYNTAQLITE_FLEXIBLE_ARRAY");
    w.line("#endif");
    w.newline();
    w.extern_c_start();

    // Enums
    let mut any_enum = false;
    for item in items {
        let Item::Enum { name, variants } = item else {
            continue;
        };
        if !any_enum {
            w.section("Value Enums");
            any_enum = true;
        }
        let prefix = format!("SYNTAQLITE_{}", upper_snake(name));
        let owned: Vec<_> = variants
            .iter()
            .enumerate()
            .map(|(i, v)| (format!("{}_{}", prefix, v), Some(i as i32)))
            .collect();
        w.typedef_enum(&c_type_name(name), &refs_i32(&owned));
        w.newline();
    }

    // Flags
    let mut any_flags = false;
    for item in items {
        let Item::Flags { name, flags } = item else {
            continue;
        };
        if !any_flags {
            w.section("Flags Types");
            any_flags = true;
        }
        let mut sorted: Vec<_> = flags.iter().collect();
        sorted.sort_by_key(|(_, v)| *v);
        let bits: Vec<_> = sorted.iter().map(|(n, v)| (n.to_lowercase(), *v)).collect();
        let bit_refs: Vec<_> = bits.iter().map(|(n, v)| (n.as_str(), *v)).collect();
        w.typedef_flags_union(&c_type_name(name), &bit_refs);
        w.newline();
    }

    // Node tags
    w.section("Node Tags");
    let mut tag_variants: Vec<(String, Option<i32>)> =
        vec![("SYNTAQLITE_NODE_NULL".into(), Some(0))];
    for item in items {
        let name = match item {
            Item::Node { name, .. } | Item::List { name, .. } => name,
            _ => continue,
        };
        tag_variants.push((tag_name(name), None));
    }
    tag_variants.push(("SYNTAQLITE_NODE_COUNT".into(), None));
    w.typedef_enum("SyntaqliteNodeTag", &refs_i32(&tag_variants));
    w.line("#ifdef __cplusplus");
    w.line("static_assert(sizeof(SyntaqliteNodeTag) == sizeof(uint32_t),");
    w.line("              \"SyntaqliteNodeTag must be 32 bits for FFI compatibility\");");
    w.line("#else");
    w.line("_Static_assert(sizeof(SyntaqliteNodeTag) == sizeof(uint32_t),");
    w.line("               \"SyntaqliteNodeTag must be 32 bits for FFI compatibility\");");
    w.line("#endif");
    w.newline();

    // Node structs
    w.section("Node Structs");
    for item in items {
        match item {
            Item::Node { name, fields, .. } => {
                let sname = c_type_name(name);
                let mut f = vec![("SyntaqliteNodeTag".to_string(), "tag".to_string())];
                for field in fields {
                    f.push((
                        field_c_type(field, &enum_names, &flags_names),
                        field.name.clone(),
                    ));
                }
                let refs: Vec<_> = f.iter().map(|(t, n)| (t.as_str(), n.as_str())).collect();
                w.typedef_struct(&sname, &refs);
                w.newline();
            }
            Item::List {
                name, child_type, ..
            } => {
                w.comment(&format!("List of {}", child_type));
                w.typedef_list_struct(&c_type_name(name));
                w.newline();
            }
            _ => {}
        }
    }

    // Node union
    w.section("Node Union");
    let mut union_members = vec![("SyntaqliteNodeTag".to_string(), "tag".to_string())];
    for item in items {
        let name = match item {
            Item::Node { name, .. } | Item::List { name, .. } => name,
            _ => continue,
        };
        union_members.push((c_type_name(name), pascal_to_snake(name)));
    }
    let union_refs: Vec<_> = union_members
        .iter()
        .map(|(t, n)| (t.as_str(), n.as_str()))
        .collect();
    w.typedef_union("SyntaqliteNode", &union_refs);
    w.newline();

    // Abstract type unions and accessors
    for item in items {
        let Item::Abstract { name, members } = item else {
            continue;
        };
        w.section(&format!("Abstract Type: {}", name));

        // Union type
        let c_abs_name = c_type_name(name);
        let mut abs_union_members = vec![("SyntaqliteNodeTag".to_string(), "tag".to_string())];
        for member in members {
            abs_union_members.push((c_type_name(member), pascal_to_snake(member)));
        }
        let abs_refs: Vec<_> = abs_union_members
            .iter()
            .map(|(t, n)| (t.as_str(), n.as_str()))
            .collect();
        w.typedef_union(&c_abs_name, &abs_refs);
        w.newline();

        // is_* function
        let check_fn = format!("syntaqlite_is_{}", pascal_to_snake(name));
        w.line(&format!(
            "static inline int {}(SyntaqliteNodeTag tag) {{",
            check_fn
        ));
        w.indent();
        w.line("switch (tag) {");
        w.indent();
        for member in members {
            w.line(&format!("case {}: return 1;", tag_name(member)));
        }
        w.line("default: return 0;");
        w.dedent();
        w.line("}");
        w.dedent();
        w.line("}");
        w.newline();

        // Per-member accessor functions
        for member in members {
            let accessor_fn = format!(
                "syntaqlite_{}_as_{}",
                pascal_to_snake(name),
                pascal_to_snake(member)
            );
            let member_type = c_type_name(member);
            w.line(&format!(
                "static inline const {}* {}(const {}* node) {{",
                member_type, accessor_fn, c_abs_name
            ));
            w.indent();
            w.line(&format!(
                "return node->tag == {} ? &node->{} : NULL;",
                tag_name(member),
                pascal_to_snake(member)
            ));
            w.dedent();
            w.line("}");
            w.newline();
        }
    }

    w.extern_c_end();
    w.newline();

    // C++ NodeTag specializations (tag-checked NodeCast support)
    w.line("#if defined(__cplusplus) && __cplusplus >= 201703L");
    w.line("#include \"syntaqlite/parser.h\"");
    w.newline();
    w.line("namespace syntaqlite {");
    w.newline();
    for item in items {
        let name = match item {
            Item::Node { name, .. } | Item::List { name, .. } => name,
            _ => continue,
        };
        let cname = c_type_name(name);
        let tag = tag_name(name);
        w.line(&format!("template <> struct NodeTag<{}> {{", cname));
        w.line(&format!("  static constexpr bool kHasTag = true;"));
        w.line(&format!("  static constexpr uint32_t kValue = {};", tag));
        w.line("};");
    }
    w.newline();
    w.line("}  // namespace syntaqlite");
    w.line("#endif");
    w.newline();
    w.header_guard_end("SYNTAQLITE_SQLITE_NODE_H");

    w.finish()
}

pub fn generate_ast_builder_h(items: &[Item], dialect: &str) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = CWriter::new();

    w.file_header();
    w.header_guard_start("SYNTAQLITE_DIALECT_BUILDER_H");
    w.include_local("syntaqlite_ext/ast_builder.h");
    w.include_local(&format!("syntaqlite_{dialect}/{dialect}_node.h"));
    w.newline();
    w.extern_c_start();

    w.section("Builder Functions");

    for item in items {
        match item {
            Item::Node { name, fields, .. } => {
                emit_node_builder_inline(&mut w, name, fields, &enum_names, &flags_names);
            }
            Item::List { name, .. } => {
                emit_list_builder_inline(&mut w, name);
            }
            _ => {}
        }
    }

    // Range field metadata (used by synq_parse_build in ast.c)
    emit_range_metadata(&mut w, items);

    w.extern_c_end();
    w.newline();
    w.header_guard_end("SYNTAQLITE_DIALECT_BUILDER_H");

    w.finish()
}

// ── Private helpers ─────────────────────────────────────────────────────

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

fn c_type_name(name: &str) -> String {
    format!("Syntaqlite{}", name)
}

fn tag_name(name: &str) -> String {
    format!("SYNTAQLITE_NODE_{}", upper_snake(name))
}

fn builder_name(name: &str) -> String {
    format!("synq_parse_{}", pascal_to_snake(name))
}

fn field_c_type(field: &Field, enum_names: &HashSet<&str>, flags_names: &HashSet<&str>) -> String {
    match field.storage {
        Storage::Index => "uint32_t".into(),
        Storage::Inline => {
            let t = &field.type_name;
            if enum_names.contains(t.as_str()) || flags_names.contains(t.as_str()) {
                c_type_name(t)
            } else {
                t.clone()
            }
        }
    }
}

fn refs_i32(owned: &[(String, Option<i32>)]) -> Vec<(&str, Option<i32>)> {
    owned.iter().map(|(s, v)| (s.as_str(), *v)).collect()
}

/// Collect range-relevant fields: (field_name, kind) where kind=0 is index, kind=1 is span.
fn range_fields(fields: &[Field]) -> Vec<(&str, u8)> {
    fields
        .iter()
        .filter_map(|f| match f.storage {
            Storage::Index => Some((f.name.as_str(), 0)),
            Storage::Inline if f.type_name == "SyntaqliteSourceSpan" => Some((f.name.as_str(), 1)),
            _ => None,
        })
        .collect()
}

// ── Builder codegen helpers ─────────────────────────────────────────────

fn emit_node_builder_inline(
    w: &mut CWriter,
    name: &str,
    fields: &[Field],
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) {
    let sn = c_type_name(name);
    let tag = tag_name(name);
    let func = builder_name(name);

    let mut param_strs = vec!["SynqParseCtx *ctx".to_string()];
    for field in fields {
        param_strs.push(format!(
            "{} {}",
            field_c_type(field, enum_names, flags_names),
            field.name
        ));
    }
    let params: Vec<&str> = param_strs.iter().map(|s| s.as_str()).collect();
    w.func_signature("static inline ", "uint32_t", &func, &params, " {");

    // Compound literal initializer parts
    let mut init_parts = vec![format!(".tag = {}", tag)];
    for field in fields {
        init_parts.push(format!(".{} = {}", field.name, field.name));
    }

    let literal = format!("&({}){{{}}}", sn, init_parts.join(", "));
    let call = format!(
        "return synq_parse_build(ctx, {}, (uint32_t)sizeof({}));",
        literal, sn
    );

    w.indent();
    if call.len() <= 80 {
        w.line(&call);
    } else {
        w.line("return synq_parse_build(ctx,");
        w.indent();
        w.line(&format!("&({}){{", sn));
        w.indent();
        for (i, part) in init_parts.iter().enumerate() {
            let comma = if i < init_parts.len() - 1 { "," } else { "" };
            w.line(&format!("{}{}", part, comma));
        }
        w.dedent();
        w.line(&format!("}}, (uint32_t)sizeof({}));", sn));
        w.dedent();
    }
    w.dedent();
    w.line("}");
    w.newline();
}

fn emit_list_builder_inline(w: &mut CWriter, name: &str) {
    let func = builder_name(name);
    let tag = tag_name(name);

    w.func_signature(
        "static inline ",
        "uint32_t",
        &func,
        &["SynqParseCtx *ctx", "uint32_t list_id", "uint32_t child"],
        " {",
    );
    w.indent();
    w.line(&format!(
        "return synq_parse_list_append(ctx, {}, list_id, child);",
        tag
    ));
    w.dedent();
    w.line("}");
    w.newline();
}

fn emit_range_metadata(w: &mut CWriter, items: &[Item]) {
    w.section("Range Field Metadata");
    // SyntaqliteFieldRangeMeta and SyntaqliteRangeMetaEntry are defined in syntaqlite/dialect.h.
    w.newline();

    // Per-node arrays
    for item in items {
        let Item::Node { name, fields, .. } = item else {
            continue;
        };
        let rf = range_fields(fields);
        if rf.is_empty() {
            continue;
        }
        let sn = c_type_name(name);
        let var = format!("range_meta_{}", pascal_to_snake(name));
        w.line(&format!(
            "static const SyntaqliteFieldRangeMeta {}[] = {{",
            var
        ));
        w.indent();
        for (fname, kind) in &rf {
            w.line(&format!("{{offsetof({}, {}), {}}},", sn, fname, kind));
        }
        w.dedent();
        w.line("};");
        w.newline();
    }

    // Dispatch table
    w.line("static const SyntaqliteRangeMetaEntry range_meta_table[] = {");
    w.indent();
    w.line("[SYNTAQLITE_NODE_NULL] = {NULL, 0},");
    for item in items {
        match item {
            Item::Node { name, fields, .. } => {
                let tag = tag_name(name);
                let rf = range_fields(fields);
                if rf.is_empty() {
                    w.line(&format!("[{}] = {{NULL, 0}},", tag));
                } else {
                    let var = format!("range_meta_{}", pascal_to_snake(name));
                    w.line(&format!("[{}] = {{{}, {}}},", tag, var, rf.len()));
                }
            }
            Item::List { name, .. } => {
                w.line(&format!("[{}] = {{NULL, 0}},", tag_name(name)));
            }
            _ => {}
        }
    }
    w.dedent();
    w.line("};");
    w.newline();
}

// ── C field metadata codegen ────────────────────────────────────────────

/// Generate a C header (`dialect_meta.h`) containing `SyntaqliteFieldMeta` arrays,
/// display string tables for enums/flags, and the top-level dispatch tables
/// (`node_names`, `field_meta`, `field_meta_counts`, `list_tags`).
///
/// This header is included by the dialect's `dialect.c` and provides
/// all the AST metadata needed by `SyntaqliteDialect`.
pub fn generate_c_field_meta(items: &[Item], dialect: &str) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = CWriter::new();
    w.file_header();
    w.header_guard_start("SYNTAQLITE_DIALECT_META_H");
    w.include_system("stddef.h");
    w.include_local("syntaqlite/dialect.h");
    w.include_local(&format!("syntaqlite_{dialect}/{dialect}_node.h"));
    w.newline();

    // Emit display string arrays for enums
    for item in items {
        let Item::Enum { name, variants } = item else {
            continue;
        };
        let var = format!("display_{}", pascal_to_snake(name));
        w.line(&format!("static const char* const {}[] = {{", var));
        w.indent();
        for v in variants {
            w.line(&format!("\"{}\",", v));
        }
        w.dedent();
        w.line("};");
        w.newline();
    }

    // Emit display string arrays for flags (indexed by bit position)
    for item in items {
        let Item::Flags { name, flags } = item else {
            continue;
        };
        let max_bit_pos = flags
            .iter()
            .map(|(_, v)| bit_position(*v))
            .max()
            .unwrap_or(0);
        let var = format!("display_{}", pascal_to_snake(name));
        w.line(&format!("static const char* const {}[] = {{", var));
        w.indent();
        for pos in 0..=max_bit_pos {
            let label = flags
                .iter()
                .find(|(_, v)| bit_position(*v) == pos)
                .map(|(n, _)| n.as_str())
                .unwrap_or("");
            w.line(&format!("\"{}\",", label));
        }
        w.dedent();
        w.line("};");
        w.newline();
    }

    // Emit per-node SyntaqliteFieldMeta arrays
    for item in items {
        let Item::Node { name, fields, .. } = item else {
            continue;
        };
        if fields.is_empty() {
            continue;
        }
        let sn = c_type_name(name);
        let var = format!("field_meta_{}", pascal_to_snake(name));
        w.line(&format!("static const SyntaqliteFieldMeta {}[] = {{", var));
        w.indent();
        for field in fields {
            let kind = c_field_kind(field, &enum_names, &flags_names);
            let (display, display_count) = c_field_display(field, &enum_names, &flags_names);
            w.line(&format!(
                "{{offsetof({}, {}), {}, \"{}\", {}, {}}},",
                sn, field.name, kind, field.name, display, display_count,
            ));
        }
        w.dedent();
        w.line("};");
        w.newline();
    }

    // Top-level tables
    w.section("Node Names");
    w.line("static const char* const ast_meta_node_names[] = {");
    w.indent();
    w.line("\"Null\",");
    for item in items {
        let name = match item {
            Item::Node { name, .. } | Item::List { name, .. } => name,
            _ => continue,
        };
        w.line(&format!("\"{}\",", name));
    }
    w.dedent();
    w.line("};");
    w.newline();

    w.section("Field Meta Dispatch");
    w.line("static const SyntaqliteFieldMeta* const ast_meta_field_meta[] = {");
    w.indent();
    w.line("NULL, /* Null */");
    for item in items {
        match item {
            Item::Node { name, fields, .. } => {
                if fields.is_empty() {
                    w.line(&format!("NULL, /* {} */", name));
                } else {
                    w.line(&format!(
                        "field_meta_{}, /* {} */",
                        pascal_to_snake(name),
                        name
                    ));
                }
            }
            Item::List { name, .. } => {
                w.line(&format!("NULL, /* {} */", name));
            }
            _ => {}
        }
    }
    w.dedent();
    w.line("};");
    w.newline();

    w.line("static const uint8_t ast_meta_field_meta_counts[] = {");
    w.indent();
    w.line("0, /* Null */");
    for item in items {
        match item {
            Item::Node { name, fields, .. } => {
                w.line(&format!("{}, /* {} */", fields.len(), name));
            }
            Item::List { name, .. } => {
                w.line(&format!("0, /* {} */", name));
            }
            _ => {}
        }
    }
    w.dedent();
    w.line("};");
    w.newline();

    w.section("List Tags");
    w.line("static const uint8_t ast_meta_list_tags[] = {");
    w.indent();
    w.line("0, /* Null */");
    for item in items {
        match item {
            Item::Node { name, .. } => {
                w.line(&format!("0, /* {} */", name));
            }
            Item::List { name, .. } => {
                w.line(&format!("1, /* {} */", name));
            }
            _ => {}
        }
    }
    w.dedent();
    w.line("};");
    w.newline();

    w.header_guard_end("SYNTAQLITE_DIALECT_META_H");
    w.finish()
}

/// Generate a C header containing structured fmt arrays (strings, enum_display, ops, dispatch).
pub fn generate_c_fmt_arrays(items: &[Item]) -> String {
    let compiled = crate::fmt_compiler::compile_all(items);

    let mut w = CWriter::new();
    w.file_header();
    w.header_guard_start("SYNTAQLITE_DIALECT_FMT_H");
    w.include_system("stdint.h");
    w.newline();

    // String table
    w.line("static const char* const fmt_strings[] = {");
    w.indent();
    for s in &compiled.strings {
        // Escape for C string literal
        let escaped = s
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n");
        w.line(&format!("\"{}\",", escaped));
    }
    w.dedent();
    w.line("};");
    w.newline();

    // Enum display table
    w.line("static const uint16_t fmt_enum_display[] = {");
    w.indent();
    let enum_display = &compiled.enum_display;
    for chunk in enum_display.chunks(16) {
        let vals: Vec<String> = chunk.iter().map(|v| format!("{}", v)).collect();
        w.line(&format!("{},", vals.join(",")));
    }
    w.dedent();
    w.line("};");
    w.newline();

    // Flatten all ops into a single pool, recording each node's offset and length.
    let mut op_pool: Vec<syntaqlite_runtime::fmt::bytecode::RawOp> = Vec::new();
    let mut node_ranges: Vec<(&str, u16, u16)> = Vec::new();

    for cn in &compiled.nodes {
        let offset = op_pool.len() as u16;
        let length = cn.ops.len() as u16;
        op_pool.extend_from_slice(&cn.ops);
        node_ranges.push((&cn.name, offset, length));
    }

    // Op pool (6 bytes per op, packed as raw bytes)
    w.line("static const uint8_t fmt_ops[] = {");
    w.indent();
    for op in &op_pool {
        let b_bytes = op.b.to_le_bytes();
        let c_bytes = op.c.to_le_bytes();
        w.line(&format!(
            "{},{},{},{},{},{},",
            op.opcode, op.a, b_bytes[0], b_bytes[1], c_bytes[0], c_bytes[1],
        ));
    }
    w.dedent();
    w.line("};");
    w.newline();

    // Build dispatch table: packed (offset << 16 | length) per node tag
    let mut dispatch_table: Vec<u32> = vec![0xFFFF_0000; compiled.tag_count];

    // Build ordinal map
    let mut ordinal_map: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
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

    for &(name, offset, length) in &node_ranges {
        if let Some(&ordinal) = ordinal_map.get(name) {
            dispatch_table[ordinal] = ((offset as u32) << 16) | (length as u32);
        }
    }

    w.line("static const uint32_t fmt_dispatch[] = {");
    w.indent();
    for chunk in dispatch_table.chunks(8) {
        let vals: Vec<String> = chunk.iter().map(|v| format!("0x{:08x}", v)).collect();
        w.line(&format!("{},", vals.join(",")));
    }
    w.dedent();
    w.line("};");
    w.newline();

    w.header_guard_end("SYNTAQLITE_DIALECT_FMT_H");
    w.finish()
}

/// Map a field to its SYNTAQLITE_FIELD_* constant string.
fn c_field_kind(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> &'static str {
    match field.storage {
        Storage::Index => "SYNTAQLITE_FIELD_NODE_ID",
        Storage::Inline => {
            let t = &field.type_name;
            if t == "SyntaqliteSourceSpan" {
                "SYNTAQLITE_FIELD_SPAN"
            } else if t == "Bool" {
                "SYNTAQLITE_FIELD_BOOL"
            } else if flags_names.contains(t.as_str()) {
                "SYNTAQLITE_FIELD_FLAGS"
            } else if enum_names.contains(t.as_str()) {
                "SYNTAQLITE_FIELD_ENUM"
            } else {
                panic!("unknown inline type for C field meta codegen: {}", t)
            }
        }
    }
}

/// Return (display_expr, display_count) for a field's C metadata.
fn c_field_display(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> (String, String) {
    match field.storage {
        Storage::Inline => {
            let t = &field.type_name;
            if enum_names.contains(t.as_str()) {
                let var = format!("display_{}", pascal_to_snake(t));
                let count = format!("sizeof({}) / sizeof({}[0])", var, var);
                (var, count)
            } else if flags_names.contains(t.as_str()) {
                let var = format!("display_{}", pascal_to_snake(t));
                let count = format!("sizeof({}) / sizeof({}[0])", var, var);
                (var, count)
            } else {
                ("NULL".into(), "0".into())
            }
        }
        _ => ("NULL".into(), "0".into()),
    }
}

/// Convert a bit value (power of 2) to its bit position.
fn bit_position(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }
    value.trailing_zeros()
}

// ── Rust codegen ────────────────────────────────────────────────────────

/// Map a field to its Rust type name (for FFI structs in ffi.rs).
fn rust_ffi_field_type(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> String {
    match field.storage {
        Storage::Index => "NodeId".into(),
        Storage::Inline => {
            let t = &field.type_name;
            if t == "Bool" {
                "Bool".into()
            } else if enum_names.contains(t.as_str()) || flags_names.contains(t.as_str()) {
                format!("super::ast::{}", t)
            } else if t == "SyntaqliteSourceSpan" {
                "SourceSpan".into()
            } else {
                t.clone()
            }
        }
    }
}

/// Map a field to its Rust type name (for pub API structs).
fn rust_field_type(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> String {
    match field.storage {
        Storage::Index => "NodeId".into(),
        Storage::Inline => {
            let t = &field.type_name;
            if enum_names.contains(t.as_str()) || flags_names.contains(t.as_str()) {
                t.clone()
            } else if t == "SyntaqliteSourceSpan" {
                "SourceSpan".into()
            } else {
                t.clone()
            }
        }
    }
}

/// Map a field to its ergonomic return type for view struct accessors.
fn rust_view_return_type(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
    node_names: &HashSet<&str>,
    list_names: &HashSet<&str>,
) -> String {
    match field.storage {
        Storage::Index => {
            let t = field.type_name.as_str();
            if list_names.contains(t) {
                format!("Option<{}<'a>>", t)
            } else if node_names.contains(t) {
                format!("Option<{}<'a>>", t)
            } else {
                // Abstract type (Expr, Stmt, etc.) — newtype wrapper
                format!("Option<{}<'a>>", t)
            }
        }
        Storage::Inline => {
            let t = &field.type_name;
            if t == "Bool" {
                "bool".into()
            } else if t == "SyntaqliteSourceSpan" {
                "&'a str".into()
            } else if enum_names.contains(t.as_str()) || flags_names.contains(t.as_str()) {
                t.clone()
            } else {
                t.clone()
            }
        }
    }
}

/// Generate the accessor body for a view struct field.
fn rust_view_accessor_body(field: &Field) -> String {
    let fname = rust_field_name(&field.name);
    match field.storage {
        Storage::Index => {
            format!("FromArena::from_arena(self.reader, self.raw.{})", fname)
        }
        Storage::Inline => {
            let t = &field.type_name;
            if t == "Bool" {
                format!("self.raw.{} == crate::ffi::Bool::True", fname)
            } else if t == "SyntaqliteSourceSpan" {
                format!("self.raw.{}.as_str(self.reader.source())", fname)
            } else {
                format!("self.raw.{}", fname)
            }
        }
    }
}

/// Check if a name is a Rust keyword that needs raw-identifier syntax.
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break"
            | "const"
            | "continue"
            | "crate"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "type"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "async"
            | "await"
            | "dyn"
            | "abstract"
            | "become"
            | "box"
            | "do"
            | "final"
            | "macro"
            | "override"
            | "priv"
            | "typeof"
            | "unsized"
            | "virtual"
            | "yield"
            | "try"
    )
}

/// Escape a name if it's a Rust keyword.
fn rust_field_name(name: &str) -> String {
    if is_rust_keyword(name) {
        format!("r#{}", name)
    } else {
        name.to_string()
    }
}

fn emit_rust_value_enum(w: &mut RustWriter, name: &str, variants: &[String]) {
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    w.line("#[repr(u32)]");
    w.open_block(&format!("pub enum {} {{", name));
    for (i, v) in variants.iter().enumerate() {
        let variant_name = upper_snake_to_pascal(v);
        w.line(&format!("{} = {},", variant_name, i));
    }
    w.close_block("}");
    w.newline();

    w.open_block(&format!("impl {} {{", name));
    w.line("#[allow(dead_code)]");
    w.open_block(&format!(
        "pub(crate) fn from_raw(raw: u32) -> Option<{}> {{",
        name
    ));
    w.open_block("match raw {");
    for (i, v) in variants.iter().enumerate() {
        let variant_name = upper_snake_to_pascal(v);
        w.line(&format!("{} => Some({}::{}),", i, name, variant_name));
    }
    w.line("_ => None,");
    w.close_block("}");
    w.close_block("}");
    w.newline();

    w.open_block("pub fn as_str(&self) -> &'static str {");
    w.open_block("match self {");
    for v in variants {
        let variant_name = upper_snake_to_pascal(v);
        w.line(&format!("{}::{} => \"{}\",", name, variant_name, v));
    }
    w.close_block("}");
    w.close_block("}");
    w.close_block("}");
    w.newline();
}

fn emit_rust_flags_type(w: &mut RustWriter, name: &str, flags: &[(String, u32)]) {
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]");
    w.line("#[repr(transparent)]");
    w.line(&format!("pub struct {}(pub u8);", name));
    w.newline();

    w.open_block(&format!("impl {} {{", name));
    let mut sorted: Vec<_> = flags.iter().collect();
    sorted.sort_by_key(|(_, v)| *v);
    for (flag_name, bit) in &sorted {
        let method = flag_name.to_lowercase();
        w.open_block(&format!("pub fn {}(&self) -> bool {{", method));
        w.line(&format!("self.0 & {} != 0", bit));
        w.close_block("}");
    }
    w.newline();
    w.open_block("pub fn dump_str(&self) -> String {");
    w.line("if self.0 == 0 { return \"(none)\".into(); }");
    w.line("let mut s = String::new();");
    for (flag_name, _) in &sorted {
        let method = flag_name.to_lowercase();
        w.line(&format!(
            "if self.{}() {{ if !s.is_empty() {{ s.push(' '); }} s.push_str(\"{}\"); }}",
            method, flag_name
        ));
    }
    w.line("s");
    w.close_block("}");
    w.close_block("}");
    w.newline();
}

fn emit_rust_node_tag_type(w: &mut RustWriter, items: &[Item]) {
    let mut tag_names: Vec<String> = Vec::new();
    let mut list_tags: Vec<String> = Vec::new();
    for item in items {
        match item {
            Item::Node { name, .. } => tag_names.push(name.clone()),
            Item::List { name, .. } => {
                tag_names.push(name.clone());
                list_tags.push(name.clone());
            }
            _ => {}
        }
    }

    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    w.line("#[repr(u32)]");
    w.open_block("pub enum NodeTag {");
    w.line("Null = 0,");
    for (i, name) in tag_names.iter().enumerate() {
        w.line(&format!("{name} = {},", i + 1));
    }
    w.close_block("}");
    w.newline();

    w.open_block("impl NodeTag {");
    w.line("#[allow(dead_code)]");
    w.open_block("pub(crate) fn from_raw(raw: u32) -> Option<NodeTag> {");
    w.open_block("match raw {");
    w.line("0 => Some(NodeTag::Null),");
    for (i, name) in tag_names.iter().enumerate() {
        w.line(&format!("{} => Some(NodeTag::{name}),", i + 1));
    }
    w.line("_ => None,");
    w.close_block("}");
    w.close_block("}");
    w.newline();

    w.line("#[allow(dead_code)]");
    w.open_block("pub(crate) fn is_list(&self) -> bool {");
    if list_tags.is_empty() {
        w.line("false");
    } else {
        w.line(&format!(
            "matches!(self, {})",
            list_tags
                .iter()
                .map(|t| format!("NodeTag::{t}"))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }
    w.close_block("}");
    w.close_block("}");
    w.newline();
}

fn emit_rust_node_structs(
    w: &mut RustWriter,
    items: &[Item],
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
    struct_visibility: &str,
    field_visibility: &str,
    field_type: fn(&Field, &HashSet<&str>, &HashSet<&str>) -> String,
) {
    for item in items {
        let Item::Node { name, fields, .. } = item else {
            continue;
        };
        w.line("#[derive(Debug, Clone, Copy)]");
        w.line("#[repr(C)]");
        w.open_block(&format!("{struct_visibility} struct {name} {{"));
        w.line(&format!("{field_visibility} tag: u32,"));
        for field in fields {
            let ty = field_type(field, enum_names, flags_names);
            let fname = rust_field_name(&field.name);
            w.line(&format!("{field_visibility} {fname}: {ty},"));
        }
        w.close_block("}");
        w.newline();
    }
}

fn emit_rust_node_tag_accessor(w: &mut RustWriter, items: &[Item]) {
    w.doc_comment("The node's tag.");
    w.open_block("pub fn tag(&self) -> NodeTag {");
    w.open_block("match self {");
    for item in items {
        let name = match item {
            Item::Node { name, .. } | Item::List { name, .. } => name,
            _ => continue,
        };
        w.line(&format!("Node::{name}(..) => NodeTag::{name},"));
    }
    w.line("Node::__Phantom(_) => unreachable!(),");
    w.close_block("}");
    w.close_block("}");
    w.newline();
}

/// Generate Rust source for token type enum.
pub fn generate_rust_tokens(tokens: &[(String, u32)]) -> String {
    let mut w = RustWriter::new();
    w.file_header();

    // TokenType enum
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]");
    w.line("#[repr(u32)]");
    w.open_block("pub enum TokenType {");
    for (name, value) in tokens {
        let variant = upper_snake_to_pascal(name);
        w.line(&format!("{} = {},", variant, value));
    }
    w.close_block("}");
    w.newline();

    // from_raw conversion
    w.open_block("impl TokenType {");
    w.line("#[allow(dead_code)]");
    w.open_block("pub(crate) fn from_raw(raw: u32) -> Option<TokenType> {");
    w.open_block("match raw {");
    for (name, value) in tokens {
        let variant = upper_snake_to_pascal(name);
        w.line(&format!("{} => Some(TokenType::{}),", value, variant));
    }
    w.line("_ => None,");
    w.close_block("}");
    w.close_block("}");
    w.close_block("}");
    w.newline();

    w.open_block("impl From<TokenType> for u32 {");
    w.open_block("fn from(t: TokenType) -> u32 {");
    w.line("t as u32");
    w.close_block("}");
    w.close_block("}");

    w.finish()
}

/// Convert an UPPER_SNAKE name to PascalCase.
/// E.g. "LIKE_KW" → "LikeKw", "UNION_ALL" → "UnionAll"
fn upper_snake_to_pascal(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut s = first.to_uppercase().to_string();
                    s.extend(chars.map(|c| c.to_ascii_lowercase()));
                    s
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Generate Rust source for all AST node types.
///
/// Only emits dynamic content derived from .synq definitions: enums, flags,
/// NodeTag, node structs, `is_list_tag`, and the `Node<'a>` enum.
/// Static types (SourceSpan, NodeList) live in hand-written `crate::nodes`.
pub fn generate_rust_nodes(items: &[Item]) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = RustWriter::new();
    w.file_header();
    w.line("use syntaqlite_runtime::parser::{NodeId, NodeList, SourceSpan};");
    w.line("use std::marker::PhantomData;");
    w.newline();

    // Value enums
    for item in items {
        let Item::Enum { name, variants } = item else {
            continue;
        };
        emit_rust_value_enum(&mut w, name, variants);
    }

    // Flags types
    for item in items {
        let Item::Flags { name, flags } = item else {
            continue;
        };
        emit_rust_flags_type(&mut w, name, flags);
    }

    // NodeTag enum
    emit_rust_node_tag_type(&mut w, items);

    // Node structs
    emit_rust_node_structs(
        &mut w,
        items,
        &enum_names,
        &flags_names,
        "pub",
        "pub",
        rust_field_type,
    );

    // Node<'a> enum — typed wrapper for AST nodes
    w.doc_comment("A typed AST node. Pattern-match to access the concrete type.");
    w.line("#[derive(Debug, Clone, Copy)]");
    w.line("pub enum Node<'a> {");
    w.indent();
    for item in items {
        match item {
            Item::Node { name, .. } => {
                w.line(&format!("{}(&'a {}),", name, name));
            }
            Item::List {
                name, child_type, ..
            } => {
                w.doc_comment(&format!("List of {}", child_type));
                w.line(&format!("{}(&'a NodeList),", name));
            }
            _ => {}
        }
    }
    w.doc_comment("Placeholder for PhantomData lifetime — never constructed.");
    w.line("#[doc(hidden)]");
    w.line("__Phantom(PhantomData<&'a ()>),");
    w.dedent();
    w.line("}");
    w.newline();

    w.line("impl<'a> Node<'a> {");
    w.indent();

    // from_raw
    w.doc_comment("Construct a typed `Node` from a raw arena pointer.");
    w.doc_comment("");
    w.doc_comment("# Safety");
    w.doc_comment("`ptr` must be non-null, well-aligned, and valid for `'a`.");
    w.doc_comment("Its first `u32` must be a valid `NodeTag` discriminant.");
    w.line("pub(crate) unsafe fn from_raw(ptr: *const u32) -> Node<'a> {");
    w.indent();
    w.line("// SAFETY: caller guarantees ptr is valid for 'a with a valid tag.");
    w.line("unsafe {");
    w.line("let tag = NodeTag::from_raw(*ptr).unwrap_or(NodeTag::Null);");
    w.line("match tag {");
    w.indent();
    for item in items {
        match item {
            Item::Node { name, .. } => {
                w.line(&format!(
                    "NodeTag::{} => Node::{}(&*(ptr as *const {})),",
                    name, name, name
                ));
            }
            Item::List { name, .. } => {
                w.line(&format!(
                    "NodeTag::{} => Node::{}(&*(ptr as *const NodeList)),",
                    name, name
                ));
            }
            _ => {}
        }
    }
    w.line("_ => unreachable!(\"unknown node tag\"),");
    w.dedent();
    w.line("}");
    w.line("} // unsafe");
    w.dedent();
    w.line("}");
    w.newline();

    // tag()
    emit_rust_node_tag_accessor(&mut w, items);

    // as_list()
    w.doc_comment("If this is a list node, return the list.");
    w.line("pub fn as_list(&self) -> Option<&'a NodeList> {");
    w.indent();
    w.line("match self {");
    w.indent();
    for item in items {
        if let Item::List { name, .. } = item {
            w.line(&format!("Node::{}(l) => Some(l),", name));
        }
    }
    w.line("_ => None,");
    w.dedent();
    w.line("}");
    w.dedent();
    w.line("}");

    w.dedent();
    w.line("}");
    w.newline();

    w.finish()
}

/// Generate Rust source for the FFI layer (`ffi.rs`).
///
/// Emits `pub(crate)` `#[repr(C)]` node structs and the `Bool` enum.
/// Enum/flags types are referenced via `super::ast::`.
pub fn generate_rust_ffi_nodes(items: &[Item]) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = RustWriter::new();
    w.file_header();
    w.lines(
        "
        #![allow(dead_code)]

        use syntaqlite_runtime::parser::{NodeId, SourceSpan};
    ",
    );
    w.newline();

    // Bool enum — FFI-internal
    w.lines(
        "
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        #[repr(u32)]
        pub(crate) enum Bool {
            False = 0,
            True = 1,
        }
    ",
    );
    w.newline();

    // Node structs — pub(crate), #[repr(C)]
    emit_rust_node_structs(
        &mut w,
        items,
        &enum_names,
        &flags_names,
        "pub(crate)",
        "pub(crate)",
        rust_ffi_field_type,
    );

    w.finish()
}

/// Generate Rust source for the public AST layer (`ast.rs`).
///
/// Emits enums, flags, `NodeTag`, view structs with ergonomic accessors,
/// and the `Node<'a>` enum that wraps them.
pub fn generate_rust_ast(items: &[Item]) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();
    let node_names: HashSet<&str> = items
        .iter()
        .filter_map(|i| {
            if let Item::Node { name, .. } = i {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect();
    let list_names: HashSet<&str> = items
        .iter()
        .filter_map(|i| {
            if let Item::List { name, .. } = i {
                Some(name.as_str())
            } else {
                None
            }
        })
        .collect();
    // Abstract types: explicitly declared via `abstract Name { ... }` in .synq files.
    let abstract_items: Vec<(&str, &[String])> = items
        .iter()
        .filter_map(|i| {
            if let Item::Abstract { name, members } = i {
                Some((name.as_str(), members.as_slice()))
            } else {
                None
            }
        })
        .collect();

    let mut w = RustWriter::new();
    w.file_header();
    w.lines("
        pub use syntaqlite_runtime::parser::{Comment, CommentKind, FromArena, NodeId, NodeReader, SourceSpan, TypedList};
        pub(crate) use syntaqlite_runtime::parser::NodeList;
        use std::marker::PhantomData;
    ");
    w.newline();

    // Value enums (skip Bool — it lives in ffi.rs)
    for item in items {
        let Item::Enum { name, variants } = item else {
            continue;
        };
        if name == "Bool" {
            continue;
        }
        emit_rust_value_enum(&mut w, name, variants);
    }

    // Flags types
    for item in items {
        let Item::Flags { name, flags } = item else {
            continue;
        };
        emit_rust_flags_type(&mut w, name, flags);
    }

    // NodeTag enum
    emit_rust_node_tag_type(&mut w, items);

    // Abstract type enums (Expr, Stmt, etc.)
    for &(abs_name, members) in &abstract_items {
        w.doc_comment(&format!(
            "Abstract `{}` — pattern-match to access the concrete type.",
            abs_name
        ));
        w.line("#[derive(Debug, Clone, Copy)]");
        w.line(&format!("pub enum {}<'a> {{", abs_name));
        w.indent();
        for member in members {
            if node_names.contains(member.as_str()) || list_names.contains(member.as_str()) {
                w.line(&format!("{}({}<'a>),", member, member));
            }
        }
        w.doc_comment(&format!(
            "A node that doesn't match any known `{}` variant.",
            abs_name
        ));
        w.line("Other(Node<'a>),");
        w.dedent();
        w.line("}");
        w.newline();

        // FromArena impl
        w.line(&format!("impl<'a> FromArena<'a> for {}<'a> {{", abs_name));
        w.indent();
        w.line("fn from_arena(reader: &'a NodeReader<'a>, id: NodeId) -> Option<Self> {");
        w.indent();
        w.line("let node = Node::resolve(reader, id)?;");
        w.line("Some(match node {");
        w.indent();
        for member in members {
            if node_names.contains(member.as_str()) || list_names.contains(member.as_str()) {
                w.line(&format!(
                    "Node::{}(n) => {}::{}(n),",
                    member, abs_name, member
                ));
            }
        }
        w.line(&format!("other => {}::Other(other),", abs_name));
        w.dedent();
        w.line("})");
        w.dedent();
        w.line("}");
        w.dedent();
        w.line("}");
        w.newline();
    }

    // View structs — ergonomic wrappers around FFI structs
    for item in items {
        let Item::Node { name, fields, .. } = item else {
            continue;
        };

        // Struct definition
        let uses_reader = fields
            .iter()
            .any(|f| f.storage == Storage::Index || f.type_name == "SyntaqliteSourceSpan");
        w.line("#[derive(Clone, Copy)]");
        w.line(&format!("pub struct {}<'a> {{", name));
        w.indent();
        w.line(&format!("raw: &'a crate::ffi::{},", name));
        if !uses_reader {
            w.line("#[allow(dead_code)]");
        }
        w.line("reader: &'a NodeReader<'a>,");
        w.line("id: NodeId,");
        w.dedent();
        w.line("}");
        w.newline();

        // Debug impl — delegate to raw FFI struct
        w.line(&format!("impl std::fmt::Debug for {}<'_> {{", name));
        w.indent();
        w.line("fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {");
        w.indent();
        w.line("self.raw.fmt(f)");
        w.dedent();
        w.line("}");
        w.dedent();
        w.line("}");
        w.newline();

        // Display impl — dump AST via NodeReader
        w.line(&format!("impl std::fmt::Display for {}<'_> {{", name));
        w.indent();
        w.lines(
            "
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                let mut buf = String::new();
                self.reader.dump_node(self.id, &mut buf, 0);
                f.write_str(&buf)
            }
        ",
        );
        w.dedent();
        w.line("}");
        w.newline();

        // Accessor methods
        w.line(&format!("impl<'a> {}<'a> {{", name));
        w.indent();
        for field in fields {
            let fname = rust_field_name(&field.name);
            let return_type =
                rust_view_return_type(field, &enum_names, &flags_names, &node_names, &list_names);
            let body = rust_view_accessor_body(field);
            w.line(&format!("pub fn {}(&self) -> {} {{", fname, return_type));
            w.indent();
            w.line(&body);
            w.dedent();
            w.line("}");
        }
        w.dedent();
        w.line("}");
        w.newline();

        // FromArena impl — resolve from arena by NodeId
        w.line(&format!("impl<'a> FromArena<'a> for {}<'a> {{", name));
        w.indent();
        w.line("fn from_arena(reader: &'a NodeReader<'a>, id: NodeId) -> Option<Self> {");
        w.indent();
        w.line("let (ptr, _) = reader.node_ptr(id)?;");
        w.line(&format!(
            "Some({} {{ raw: unsafe {{ &*(ptr as *const crate::ffi::{}) }}, reader, id }})",
            name, name
        ));
        w.dedent();
        w.line("}");
        w.dedent();
        w.line("}");
        w.newline();
    }

    // Typed list type aliases
    for item in items {
        let Item::List {
            name, child_type, ..
        } = item
        else {
            continue;
        };
        let ct = child_type.as_str();
        let element_type = if node_names.contains(ct) || list_names.contains(ct) {
            format!("{}<'a>", ct)
        } else {
            "Node<'a>".into()
        };
        w.doc_comment(&format!("Typed list of `{}`.", child_type));
        w.line(&format!(
            "pub type {}<'a> = TypedList<'a, {}>;",
            name, element_type
        ));
        w.newline();
    }

    // Node<'a> enum — wraps view structs
    w.doc_comment("A typed AST node. Pattern-match to access the concrete type.");
    w.line("#[derive(Debug, Clone, Copy)]");
    w.line("pub enum Node<'a> {");
    w.indent();
    for item in items {
        match item {
            Item::Node { name, .. } => {
                w.line(&format!("{}({}<'a>),", name, name));
            }
            Item::List {
                name, child_type, ..
            } => {
                w.doc_comment(&format!("List of {}", child_type));
                w.line(&format!("{}({}<'a>),", name, name));
            }
            _ => {}
        }
    }
    w.doc_comment("Placeholder for PhantomData lifetime — never constructed.");
    w.line("#[doc(hidden)]");
    w.line("__Phantom(PhantomData<&'a ()>),");
    w.dedent();
    w.line("}");
    w.newline();

    w.line("impl<'a> Node<'a> {");
    w.indent();

    // from_raw
    w.doc_comment("Construct a typed `Node` from a raw arena pointer.");
    w.doc_comment("");
    w.doc_comment("# Safety");
    w.doc_comment("`ptr` must be non-null, well-aligned, and valid for `'a`.");
    w.doc_comment("Its first `u32` must be a valid `NodeTag` discriminant.");
    w.line("pub(crate) unsafe fn from_raw(ptr: *const u32, reader: &'a NodeReader<'a>, id: NodeId) -> Node<'a> {");
    w.indent();
    w.line("// SAFETY: caller guarantees ptr is valid for 'a with a valid tag.");
    w.line("unsafe {");
    w.line("let tag = NodeTag::from_raw(*ptr).unwrap_or(NodeTag::Null);");
    w.line("match tag {");
    w.indent();
    for item in items {
        match item {
            Item::Node { name, .. } => {
                w.line(&format!("NodeTag::{n} => Node::{n}({n} {{ raw: &*(ptr as *const crate::ffi::{n}), reader, id }}),", n = name));
            }
            Item::List { name, .. } => {
                w.line(&format!("NodeTag::{n} => Node::{n}(TypedList::new(&*(ptr as *const NodeList), reader)),", n = name));
            }
            _ => {}
        }
    }
    w.line("_ => unreachable!(\"unknown node tag\"),");
    w.dedent();
    w.line("}");
    w.line("} // unsafe");
    w.dedent();
    w.line("}");
    w.newline();

    // resolve
    w.doc_comment("Resolve a `NodeId` into a typed `Node`, or `None` if null/invalid.");
    w.lines(
        "
        pub(crate) fn resolve(reader: &'a NodeReader<'a>, id: NodeId) -> Option<Node<'a>> {
            let (ptr, _tag) = reader.node_ptr(id)?;
            Some(unsafe { Node::from_raw(ptr as *const u32, reader, id) })
        }
    ",
    );
    w.newline();

    // tag()
    emit_rust_node_tag_accessor(&mut w, items);

    w.dedent();
    w.line("}");
    w.newline();

    // Display impl for Node
    w.line("impl std::fmt::Display for Node<'_> {");
    w.indent();
    w.line("fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {");
    w.indent();
    w.line("match self {");
    w.indent();
    for item in items {
        if let Item::Node { name, .. } = item {
            w.line(&format!(
                "Node::{n}(n) => std::fmt::Display::fmt(n, f),",
                n = name
            ));
        }
    }
    // Fallback for list variants and phantom
    w.line("_ => std::fmt::Debug::fmt(self, f),");
    w.dedent();
    w.line("}");
    w.dedent();
    w.line("}");
    w.dedent();
    w.line("}");
    w.newline();

    // FromArena impl for Node
    w.lines(
        "
        impl<'a> FromArena<'a> for Node<'a> {
            fn from_arena(reader: &'a NodeReader<'a>, id: NodeId) -> Option<Self> {
                Node::resolve(reader, id)
            }
        }
    ",
    );
    w.newline();

    w.finish()
}

// ── Dialect crate boilerplate ───────────────────────────────────────────

/// Generate `lib.rs` for a dialect crate.
///
/// `dialect_fn` is the C function name that returns the dialect pointer
/// (e.g. `"syntaqlite_sqlite_dialect"`).
pub fn generate_rust_lib(dialect_fn: &str) -> String {
    let mut w = RustWriter::new();
    w.file_header();
    w.lines(
        "
        mod ffi;
        /// Typed AST nodes for this dialect.
        ///
        /// Each SQL statement type (e.g. `SELECT`, `INSERT`) has a corresponding struct
        /// with typed accessors for its fields. The top-level enum is [`ast::Stmt`],
        /// returned by [`StatementCursor::next_statement`] and
        /// [`LowLevelCursor::finish`](low_level::LowLevelCursor::finish).
        pub mod ast;
        mod wrappers;
    ",
    );
    w.newline();
    w.line("use std::sync::LazyLock;");
    w.newline();
    w.line("use syntaqlite_runtime::dialect::ffi as dialect_ffi;");
    w.line("unsafe extern \"C\" {");
    w.indent();
    w.line(&format!(
        "fn {}() -> *const dialect_ffi::Dialect;",
        dialect_fn
    ));
    w.dedent();
    w.line("}");
    w.newline();
    w.line("static DIALECT: LazyLock<syntaqlite_runtime::Dialect<'static>> =");
    w.line(&format!(
        "    LazyLock::new(|| unsafe {{ syntaqlite_runtime::Dialect::from_raw({}()) }});",
        dialect_fn
    ));
    w.newline();
    w.lines(
        "
        /// Low-level APIs for advanced use cases (e.g. custom token feeding/tokenizing).
        pub mod low_level {
            pub use crate::wrappers::{LowLevelCursor, LowLevelParser, Tokenizer, TokenCursor};
            pub use crate::tokens::TokenType;

            /// Access the dialect handle (for use with `syntaqlite_runtime` APIs).
            pub fn dialect() -> &'static syntaqlite_runtime::Dialect<'static> {
                &crate::DIALECT
            }
        }
    ",
    );
    w.newline();
    w.lines(
        "
        pub use wrappers::{Formatter, Parser, StatementCursor};
        pub use syntaqlite_runtime::ParseError;
    ",
    );
    w.newline();
    w.lines(
        "
        /// Configuration types for parsers and formatters.
        pub mod config {
            pub use syntaqlite_runtime::fmt::{FormatConfig, KeywordCase};
            pub use syntaqlite_runtime::parser::ParserConfig;
        }
    ",
    );
    w.newline();
    w.line("mod tokens;");
    w.finish()
}

/// Generate `wrappers.rs` for a dialect crate.
pub fn generate_rust_wrappers() -> String {
    let mut w = RustWriter::new();
    w.file_header();
    w.lines(
        "
        use std::ops::Range;

        use crate::ast::{FromArena, Stmt};
        use crate::low_level::TokenType;
        use crate::ParseError;
    ",
    );
    w.newline();

    // Parser
    w.lines(
        "
        /// A parser pre-configured for this dialect.
        ///
        /// Returns typed `StatementCursor` wrappers from `parse()`.
        pub struct Parser {
            inner: syntaqlite_runtime::Parser,
        }

        impl Parser {
            /// Create a new parser with default configuration.
            pub fn new() -> Self {
                Parser {
                    inner: syntaqlite_runtime::Parser::new(&crate::DIALECT),
                }
            }

            /// Create a parser with the given configuration.
            pub fn with_config(config: &syntaqlite_runtime::parser::ParserConfig) -> Self {
                Parser {
                    inner: syntaqlite_runtime::Parser::with_config(&crate::DIALECT, config),
                }
            }

            /// Access the current configuration.
            pub fn config(&self) -> &syntaqlite_runtime::parser::ParserConfig {
                self.inner.config()
            }

            /// Parse source text and return a `StatementCursor` for iterating statements.
            pub fn parse<'a>(&'a mut self, source: &'a str) -> StatementCursor<'a> {
                StatementCursor { inner: self.inner.parse(source) }
            }
        }
    ",
    );
    w.newline();

    // StatementCursor
    w.lines(
        "
        /// A high-level parsing cursor with typed node access.
        pub struct StatementCursor<'a> {
            inner: syntaqlite_runtime::StatementCursor<'a>,
        }

        impl<'a> StatementCursor<'a> {
            /// Parse and return the next SQL statement as a typed `Stmt`.
            ///
            /// The returned `Stmt` borrows this cursor, so it cannot outlive it.
            /// Returns `None` when all statements have been consumed.
            pub fn next_statement(&mut self) -> Option<Result<Stmt<'_>, ParseError>> {
                let id = match self.inner.next_statement()? {
                    Ok(id) => id,
                    Err(e) => return Some(Err(e)),
                };
                let reader = self.inner.reader();
                Some(Ok(Stmt::from_arena(reader, id).expect(\"parser returned invalid node\")))
            }
        }
    ",
    );
    w.newline();

    // LowLevelParser
    w.lines("
        /// A low-level parser for token-by-token feeding.
        ///
        /// Feed tokens one at a time via `LowLevelCursor`.
        pub struct LowLevelParser {
            inner: syntaqlite_runtime::parser::LowLevelParser,
        }

        impl LowLevelParser {
            /// Create a new low-level parser with default configuration.
            pub fn new() -> Self {
                LowLevelParser {
                    inner: syntaqlite_runtime::parser::LowLevelParser::new(&crate::DIALECT),
                }
            }

            /// Create a low-level parser with the given configuration.
            pub fn with_config(config: &syntaqlite_runtime::parser::ParserConfig) -> Self {
                LowLevelParser {
                    inner: syntaqlite_runtime::parser::LowLevelParser::with_config(&crate::DIALECT, config),
                }
            }

            /// Bind source text and return a `LowLevelCursor` for token feeding.
            pub fn feed<'a>(&'a mut self, source: &'a str) -> LowLevelCursor<'a> {
                LowLevelCursor { inner: self.inner.feed(source) }
            }
        }
    ");
    w.newline();

    // LowLevelCursor
    w.lines("
        /// A low-level cursor for feeding tokens one at a time.
        ///
        /// After calling `finish()`, no further feeding methods may be called.
        pub struct LowLevelCursor<'a> {
            inner: syntaqlite_runtime::parser::LowLevelCursor<'a>,
        }

        impl<'a> LowLevelCursor<'a> {
            /// Feed a typed token to the parser.
            ///
            /// Returns `Ok(Some(stmt))` when a statement completes,
            /// `Ok(None)` to keep going, or `Err` on parse error.
            ///
            /// The returned `Stmt` borrows this cursor, so it cannot be held
            /// across further `feed_token` calls.
            ///
            /// `span` is a byte range into the source text bound by this cursor.
            pub fn feed_token(
                &mut self,
                token_type: TokenType,
                span: Range<usize>,
            ) -> Result<Option<Stmt<'_>>, ParseError> {
                match self.inner.feed_token(token_type.into(), span)? {
                    None => Ok(None),
                    Some(id) => {
                        let reader = self.inner.base().reader();
                        Ok(Some(Stmt::from_arena(reader, id).expect(\"parser returned invalid node\")))
                    }
                }
            }

            /// Signal end of input.
            ///
            /// Returns `Ok(Some(stmt))` if a final statement completed,
            /// `Ok(None)` if there was nothing pending, or `Err` on parse error.
            ///
            /// After calling `finish()`, no further feeding methods may be called.
            pub fn finish(&mut self) -> Result<Option<Stmt<'_>>, ParseError> {
                match self.inner.finish()? {
                    None => Ok(None),
                    Some(id) => {
                        let reader = self.inner.base().reader();
                        Ok(Some(Stmt::from_arena(reader, id).expect(\"parser returned invalid node\")))
                    }
                }
            }

            /// Mark subsequent fed tokens as being inside a macro expansion.
            pub fn begin_macro(&mut self, call_offset: u32, call_length: u32) {
                self.inner.begin_macro(call_offset, call_length)
            }

            /// End the innermost macro expansion region.
            pub fn end_macro(&mut self) {
                self.inner.end_macro()
            }
        }
    ");
    w.newline();

    // Formatter
    w.lines("
        /// SQL formatter pre-configured for this dialect.
        pub struct Formatter {
            inner: syntaqlite_runtime::fmt::Formatter<'static>,
        }

        impl Formatter {
            /// Create a formatter with default configuration.
            pub fn new() -> Result<Self, &'static str> {
                let inner = syntaqlite_runtime::fmt::Formatter::new(&crate::DIALECT)?;
                Ok(Formatter { inner })
            }

            /// Create a formatter with the given configuration.
            pub fn with_config(config: crate::config::FormatConfig) -> Result<Self, &'static str> {
                let inner = syntaqlite_runtime::fmt::Formatter::with_config(&crate::DIALECT, config)?;
                Ok(Formatter { inner })
            }

            /// Access the current configuration.
            pub fn config(&self) -> &crate::config::FormatConfig {
                self.inner.config()
            }

            /// Format SQL source text.
            pub fn format(
                &mut self,
                source: &str,
            ) -> Result<String, ParseError> {
                self.inner.format(source)
            }
        }
    ");
    w.newline();

    // Tokenizer
    w.lines(
        "
        /// A tokenizer for SQL.
        pub struct Tokenizer {
            inner: syntaqlite_runtime::parser::Tokenizer,
        }

        impl Tokenizer {
            /// Create a new tokenizer.
            pub fn new() -> Self {
                Tokenizer {
                    inner: syntaqlite_runtime::parser::Tokenizer::new(*crate::DIALECT),
                }
            }

            /// Bind source text and return a cursor for iterating typed tokens.
            pub fn tokenize<'a>(&'a mut self, source: &'a str) -> TokenCursor<'a> {
                TokenCursor {
                    inner: self.inner.tokenize(source),
                }
            }

            /// Zero-copy variant: bind a null-terminated source and return a
            /// `TokenCursor`. The source must be valid UTF-8 (panics otherwise).
            pub fn tokenize_cstr<'a>(&'a mut self, source: &'a std::ffi::CStr) -> TokenCursor<'a> {
                TokenCursor {
                    inner: self.inner.tokenize_cstr(source),
                }
            }
        }
    ",
    );
    w.newline();

    // TokenCursor
    w.lines(
        "
        /// An active tokenizer session yielding typed tokens.
        pub struct TokenCursor<'a> {
            inner: syntaqlite_runtime::parser::TokenCursor<'a>,
        }

        impl<'a> Iterator for TokenCursor<'a> {
            type Item = (TokenType, &'a str);

            fn next(&mut self) -> Option<Self::Item> {
                let raw = self.inner.next()?;
                let tt = TokenType::from_raw(raw.token_type)
                    .unwrap_or(TokenType::Illegal);
                Some((tt, raw.text))
            }
        }
    ",
    );

    w.finish()
}

/// Generate `dialect.c` — the dialect descriptor struct and public API functions.
///
/// `dialect` is a short name like `"sqlite"` or `"perfetto"`.
pub fn generate_dialect_c(dialect: &str) -> String {
    let upper = dialect.to_uppercase();
    let mut w = CWriter::new();
    w.file_header();
    w.include_local("syntaqlite/parser.h");
    w.include_local(&format!("syntaqlite_{dialect}/{dialect}_tokens.h"));
    w.include_local("syntaqlite/dialect.h");
    w.include_local("csrc/dialect_builder.h");
    w.include_local("csrc/dialect_meta.h");
    w.include_local("csrc/dialect_fmt.h");
    w.include_local("csrc/sqlite_parse.h");
    w.include_local("csrc/sqlite_tokenize.h");
    w.newline();

    w.section(&format!("{} dialect descriptor", dialect));
    w.newline();
    w.line(&format!(
        "static const SyntaqliteDialect {upper}_DIALECT = {{"
    ));
    w.line(&format!("    .name = \"{dialect}\","));
    w.newline();
    w.line("    .range_meta = range_meta_table,");
    w.line("    .tk_space = SYNTAQLITE_TK_SPACE,");
    w.line("    .tk_semi = SYNTAQLITE_TK_SEMI,");
    w.line("    .tk_comment = SYNTAQLITE_TK_COMMENT,");
    w.newline();
    w.line("    // AST metadata");
    w.line("    .node_count = sizeof(ast_meta_node_names) / sizeof(ast_meta_node_names[0]),");
    w.line("    .node_names = ast_meta_node_names,");
    w.line("    .field_meta = ast_meta_field_meta,");
    w.line("    .field_meta_counts = ast_meta_field_meta_counts,");
    w.line("    .list_tags = ast_meta_list_tags,");
    w.newline();
    w.line("    // Formatter data");
    w.line("    .fmt_strings = fmt_strings,");
    w.line("    .fmt_string_count = sizeof(fmt_strings) / sizeof(fmt_strings[0]),");
    w.line("    .fmt_enum_display = fmt_enum_display,");
    w.line("    .fmt_enum_display_count = sizeof(fmt_enum_display) / sizeof(fmt_enum_display[0]),");
    w.line("    .fmt_ops = fmt_ops,");
    w.line("    .fmt_op_count = sizeof(fmt_ops) / 6,");
    w.line("    .fmt_dispatch = fmt_dispatch,");
    w.line("    .fmt_dispatch_count = sizeof(fmt_dispatch) / sizeof(fmt_dispatch[0]),");
    w.newline();
    let pascal = pascal_case(dialect);
    w.line("    // Parser lifecycle");
    w.line(&format!("    .parser_alloc = Synq{pascal}ParseAlloc,"));
    w.line(&format!("    .parser_init = Synq{pascal}ParseInit,"));
    w.line(&format!(
        "    .parser_finalize = Synq{pascal}ParseFinalize,"
    ));
    w.line(&format!("    .parser_free = Synq{pascal}ParseFree,"));
    w.line(&format!("    .parser_feed = Synq{pascal}Parse,"));
    w.line("#ifndef NDEBUG");
    w.line(&format!("    .parser_trace = Synq{pascal}ParseTrace,"));
    w.line("#endif");
    w.newline();
    w.line("    // Tokenizer");
    w.line(&format!("    .get_token = Synq{pascal}GetToken,"));
    w.line("};");
    w.newline();

    w.section("Public API");
    w.newline();
    w.line(&format!(
        "const SyntaqliteDialect* syntaqlite_{dialect}_dialect(void) {{"
    ));
    w.line(&format!("    return &{upper}_DIALECT;"));
    w.line("}");
    w.newline();
    w.line(&format!(
        "SyntaqliteParser* syntaqlite_create_{dialect}_parser(const SyntaqliteMemMethods* mem) {{"
    ));
    w.line(&format!(
        "    return syntaqlite_create_parser_with_dialect(mem, &{upper}_DIALECT);"
    ));
    w.line("}");

    w.finish()
}

/// Generate the public API header for a dialect.
///
/// `dialect` is a short name like `"sqlite"` or `"perfetto"`.
pub fn generate_dialect_h(dialect: &str) -> String {
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_{upper}_H");
    let mut w = CWriter::new();
    w.file_header();
    w.line(&format!("#ifndef {guard}"));
    w.line(&format!("#define {guard}"));
    w.newline();
    w.include_local("syntaqlite/config.h");
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("extern \"C\" {");
    w.line("#endif");
    w.newline();
    w.line("typedef struct SyntaqliteDialect SyntaqliteDialect;");
    w.line("typedef struct SyntaqliteParser SyntaqliteParser;");
    w.newline();
    w.line(&format!(
        "const SyntaqliteDialect* syntaqlite_{dialect}_dialect(void);"
    ));
    w.line(&format!(
        "SyntaqliteParser* syntaqlite_create_{dialect}_parser(const SyntaqliteMemMethods* mem);"
    ));
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("}");
    w.line("#endif");
    w.newline();
    w.line("#if defined(__cplusplus) && __cplusplus >= 201703L");
    w.include_local("syntaqlite/parser.h");
    w.newline();
    w.line("namespace syntaqlite {");
    w.newline();
    let pascal = pascal_case(dialect);
    w.line(&format!("inline Parser {pascal}Parser() {{"));
    w.line(&format!(
        "  return Parser(syntaqlite_create_{dialect}_parser(nullptr));"
    ));
    w.line("}");
    w.newline();
    w.line("}  // namespace syntaqlite");
    w.line("#endif");
    w.newline();
    w.line(&format!("#endif  // {guard}"));

    w.finish()
}

/// Generate the dialect dispatch header for amalgamation builds.
///
/// Produces a header like `sqlite_dialect_dispatch.h` that defines the
/// `SYNQ_PARSER_ALLOC`, etc. macros to call the dialect's parser/tokenizer
/// functions directly (bypassing function pointer indirection).
pub fn generate_dialect_dispatch_h(dialect: &str) -> String {
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_{upper}_DIALECT_DISPATCH_H");
    let mut w = CWriter::new();
    w.file_header();
    w.line(&format!("#ifndef {guard}"));
    w.line(&format!("#define {guard}"));
    w.newline();
    let pascal = pascal_case(dialect);
    w.line(&format!(
        "#define SYNQ_PARSER_ALLOC(d, m)          Synq{pascal}ParseAlloc(m)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_INIT(d, p)           Synq{pascal}ParseInit(p)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_FINALIZE(d, p)       Synq{pascal}ParseFinalize(p)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_FREE(d, p, f)        Synq{pascal}ParseFree(p, f)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_FEED(d, p, t, m, c)  Synq{pascal}Parse(p, t, m, c)"
    ));
    w.line(&format!(
        "#define SYNQ_PARSER_TRACE(d, f, s)       Synq{pascal}ParseTrace(f, s)"
    ));
    w.line(&format!(
        "#define SYNQ_GET_TOKEN(d, z, t)          Synq{pascal}GetToken(z, t)"
    ));
    w.newline();
    w.line(&format!("#endif  // {guard}"));
    w.finish()
}

/// Generate forward declarations for the Lemon-generated parser functions.
///
/// Produces `sqlite_parse.h` with declarations for `SynqSqliteParseAlloc`,
/// `SynqSqliteParseFree`, etc.  Needed by the amalgamation so that
/// `dialect.c` (emitted before `sqlite_parse.c`) can reference the symbols.
pub fn generate_parse_h(dialect: &str) -> String {
    let pascal = pascal_case(dialect);
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_{upper}_PARSE_H");
    let mut w = CWriter::new();
    w.file_header();
    w.line(&format!("#ifndef {guard}"));
    w.line(&format!("#define {guard}"));
    w.newline();
    w.line("#include <stddef.h>");
    w.line("#include <stdio.h>");
    w.newline();
    w.include_local("syntaqlite_ext/ast_builder.h");
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("extern \"C\" {");
    w.line("#endif");
    w.newline();
    w.line(&format!(
        "void* Synq{pascal}ParseAlloc(void* (*mallocProc)(size_t));"
    ));
    w.line(&format!("void Synq{pascal}ParseInit(void* parser);"));
    w.line(&format!("void Synq{pascal}ParseFinalize(void* parser);"));
    w.line(&format!(
        "void Synq{pascal}ParseFree(void* parser, void (*freeProc)(void*));"
    ));
    w.line(&format!(
        "void Synq{pascal}Parse(void* parser, int token_type, SynqParseToken minor,"
    ));
    w.line(&format!(
        "{}SynqParseCtx* pCtx);",
        " ".repeat(5 + 4 + pascal.len() + 5 + 1)
    ));
    w.line("#ifndef NDEBUG");
    w.line(&format!(
        "void Synq{pascal}ParseTrace(FILE* trace_file, char* prompt);"
    ));
    w.line("#endif");
    w.newline();
    w.line("#ifdef __cplusplus");
    w.line("}");
    w.line("#endif");
    w.newline();
    w.line(&format!("#endif  // {guard}"));
    w.finish()
}

/// Generate forward declaration for the tokenizer function.
pub fn generate_tokenize_h(dialect: &str) -> String {
    let pascal = pascal_case(dialect);
    let upper = dialect.to_uppercase();
    let guard = format!("SYNTAQLITE_INTERNAL_{upper}_TOKENIZE_H");
    let mut w = CWriter::new();
    w.file_header();
    w.line(&format!("#ifndef {guard}"));
    w.line(&format!("#define {guard}"));
    w.newline();
    w.include_local("syntaqlite_ext/sqlite_compat.h");
    w.newline();
    w.line(&format!(
        "i64 Synq{pascal}GetToken(const unsigned char* z, int* tokenType);"
    ));
    w.newline();
    w.line(&format!("#endif  // {guard}"));
    w.finish()
}

pub fn pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}
