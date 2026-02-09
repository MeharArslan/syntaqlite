use std::collections::HashSet;

use crate::c_writer::CWriter;
use crate::node_parser::{Field, Item, Storage};

// ── Public API ──────────────────────────────────────────────────────────

pub fn generate_ast_nodes_h(items: &[Item]) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = CWriter::new();

    w.file_header();
    w.header_guard_start("SYNTAQLITE_AST_NODES_H");
    w.include_system("stddef.h");
    w.include_system("stdint.h");
    w.newline();
    w.include_local("syntaqlite/ast.h");
    w.newline();
    w.extern_c_start();

    // Enums
    let mut any_enum = false;
    for item in items {
        let Item::Enum { name, variants } = item else { continue };
        if !any_enum { w.section("Value Enums"); any_enum = true; }
        let prefix = format!("SYNTAQLITE_{}", upper_snake(name));
        let owned: Vec<_> = variants.iter().enumerate()
            .map(|(i, v)| (format!("{}_{}", prefix, v), Some(i as i32)))
            .collect();
        w.typedef_enum(&c_type_name(name), &refs_i32(&owned));
        w.newline();
    }

    // Flags
    let mut any_flags = false;
    for item in items {
        let Item::Flags { name, flags } = item else { continue };
        if !any_flags { w.section("Flags Types"); any_flags = true; }
        let mut sorted: Vec<_> = flags.iter().collect();
        sorted.sort_by_key(|(_, v)| *v);
        let bits: Vec<_> = sorted.iter()
            .map(|(n, v)| (n.to_lowercase(), *v))
            .collect();
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
    w.newline();

    // Node structs
    w.section("Node Structs");
    for item in items {
        match item {
            Item::Node { name, fields, .. } => {
                let sname = c_type_name(name);
                let mut f = vec![("uint8_t".to_string(), "tag".to_string())];
                for field in fields {
                    f.push((field_c_type(field, &enum_names, &flags_names), field.name.clone()));
                }
                let refs: Vec<_> = f.iter().map(|(t, n)| (t.as_str(), n.as_str())).collect();
                w.typedef_struct(&sname, &refs);
                w.newline();
            }
            Item::List { name, child_type, .. } => {
                w.comment(&format!("List of {}", child_type));
                w.typedef_struct(&c_type_name(name), &[
                    ("uint8_t", "tag"),
                    ("uint8_t", "_pad[3]"),
                    ("uint32_t", "count"),
                    ("uint32_t", "children[]"),
                ]);
                w.newline();
            }
            _ => {}
        }
    }

    // Node union
    w.section("Node Union");
    let mut union_members = vec![("uint8_t".to_string(), "tag".to_string())];
    for item in items {
        let name = match item {
            Item::Node { name, .. } | Item::List { name, .. } => name,
            _ => continue,
        };
        union_members.push((c_type_name(name), pascal_to_snake(name)));
    }
    let union_refs: Vec<_> = union_members.iter().map(|(t, n)| (t.as_str(), n.as_str())).collect();
    w.typedef_union("SyntaqliteNode", &union_refs);
    w.newline();

    w.extern_c_end();
    w.newline();
    w.header_guard_end("SYNTAQLITE_AST_NODES_H");

    w.finish()
}

pub fn generate_ast_builder_h(items: &[Item]) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = CWriter::new();

    w.file_header();
    w.header_guard_start("SYNQ_AST_BUILDER_H");
    w.include_local("csrc/ast.h");
    w.include_local("syntaqlite/ast_nodes.h");
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

    // Range field metadata (used by synq_ast_build in ast.c)
    emit_range_metadata(&mut w, items);

    w.extern_c_end();
    w.newline();
    w.header_guard_end("SYNQ_AST_BUILDER_H");

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
    format!("synq_ast_{}", pascal_to_snake(name))
}

fn field_c_type(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> String {
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
    fields.iter().filter_map(|f| match f.storage {
        Storage::Index => Some((f.name.as_str(), 0)),
        Storage::Inline if f.type_name == "SyntaqliteSourceSpan" => Some((f.name.as_str(), 1)),
        _ => None,
    }).collect()
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

    let mut param_strs = vec!["SynqAstContext *ctx".to_string()];
    for field in fields {
        param_strs.push(format!("{} {}", field_c_type(field, enum_names, flags_names), field.name));
    }
    let params: Vec<&str> = param_strs.iter().map(|s| s.as_str()).collect();
    w.func_signature("static inline ", "uint32_t", &func, &params, " {");

    // Compound literal initializer parts
    let mut init_parts = vec![format!(".tag = {}", tag)];
    for field in fields {
        init_parts.push(format!(".{} = {}", field.name, field.name));
    }

    let literal = format!("&({}){{{}}}", sn, init_parts.join(", "));
    let call = format!("return synq_ast_build(ctx, {}, {}, sizeof({}));", tag, literal, sn);

    w.indent();
    if call.len() <= 80 {
        w.line(&call);
    } else {
        w.line(&format!("return synq_ast_build(ctx, {},", tag));
        w.indent();
        w.line(&format!("&({}){{", sn));
        w.indent();
        for (i, part) in init_parts.iter().enumerate() {
            let comma = if i < init_parts.len() - 1 { "," } else { "" };
            w.line(&format!("{}{}", part, comma));
        }
        w.dedent();
        w.line(&format!("}}, sizeof({}));", sn));
        w.dedent();
    }
    w.dedent();
    w.line("}");
    w.newline();
}

fn emit_list_builder_inline(w: &mut CWriter, name: &str) {
    let func = builder_name(name);
    let sn = c_type_name(name);
    let tag = tag_name(name);

    // Empty list creator
    w.comment(&format!("Create empty {}", name));
    w.func_signature("static inline ", "uint32_t", &format!("{}_empty", func),
        &["SynqAstContext *ctx"], " {");
    w.indent();
    w.line(&format!("return synq_ast_list_empty(ctx, {}, sizeof({}));", tag, sn));
    w.dedent();
    w.line("}");
    w.newline();

    // Single-child creator
    w.comment(&format!("Create {} with single child", name));
    w.func_signature("static inline ", "uint32_t", &func,
        &["SynqAstContext *ctx", "uint32_t first_child"], " {");
    w.indent();
    w.line(&format!("return synq_ast_list_start(ctx, {}, first_child);", tag));
    w.dedent();
    w.line("}");
    w.newline();

    // Append function
    w.comment(&format!("Append child to {} (may reallocate, returns new list ID)", name));
    w.func_signature("static inline ", "uint32_t", &format!("{}_append", func),
        &["SynqAstContext *ctx", "uint32_t list_id", "uint32_t child"], " {");
    w.indent();
    w.line("if (list_id == SYNTAQLITE_NULL_NODE) {");
    w.indent();
    w.line(&format!("return synq_ast_list_start(ctx, {}, child);", tag));
    w.dedent();
    w.line("}");
    w.line(&format!("return synq_ast_list_append(ctx, list_id, child, {});", tag));
    w.dedent();
    w.line("}");
    w.newline();
}

fn emit_range_metadata(w: &mut CWriter, items: &[Item]) {
    w.section("Range Field Metadata");
    w.line("typedef struct { uint16_t offset; uint8_t kind; } SynqFieldRangeMeta;");
    w.newline();

    // Per-node arrays
    for item in items {
        let Item::Node { name, fields, .. } = item else { continue };
        let rf = range_fields(fields);
        if rf.is_empty() { continue; }
        let sn = c_type_name(name);
        let var = format!("range_meta_{}", pascal_to_snake(name));
        w.line(&format!("static const SynqFieldRangeMeta {}[] = {{", var));
        w.indent();
        for (fname, kind) in &rf {
            w.line(&format!("{{offsetof({}, {}), {}}},", sn, fname, kind));
        }
        w.dedent();
        w.line("};");
        w.newline();
    }

    // Dispatch table
    w.line("static const struct { const SynqFieldRangeMeta *fields; uint8_t count; } range_meta_table[] = {");
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
