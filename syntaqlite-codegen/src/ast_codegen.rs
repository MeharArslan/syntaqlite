use std::collections::HashSet;
use std::fmt::Write as _;

use crate::c_writer::CWriter;
use crate::node_parser::{Field, Item, Storage};

// ── Public API ──────────────────────────────────────────────────────────

pub fn generate_ast_nodes_h(items: &[Item]) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = CWriter::new();

    w.file_header();
    w.header_guard_start("SYNTAQLITE_NODE_H");
    w.include_system("stddef.h");
    w.include_system("stdint.h");
    w.newline();
    w.newline();
    w.extern_c_start();

    // Shared AST primitives
    w.line("#define SYNTAQLITE_NULL_NODE 0xFFFFFFFFu");
    w.newline();
    w.typedef_struct("SyntaqliteSourceSpan", &[
        ("uint32_t", "offset"),
        ("uint16_t", "length"),
    ]);
    w.newline();

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
                let mut f = vec![("uint32_t".to_string(), "tag".to_string())];
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
                    ("uint32_t", "tag"),
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
    let mut union_members = vec![("uint32_t".to_string(), "tag".to_string())];
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
    w.header_guard_end("SYNTAQLITE_NODE_H");

    w.finish()
}

pub fn generate_ast_builder_h(items: &[Item]) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut w = CWriter::new();

    w.file_header();
    w.header_guard_start("SYNQ_AST_BUILDER_H");
    w.include_local("csrc/parser.h");
    w.include_local("syntaqlite/node.h");
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
    format!("synq_parse_{}", pascal_to_snake(name))
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

    let mut param_strs = vec!["SynqParseCtx *ctx".to_string()];
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
    let call = format!("return synq_parse_build(ctx, {}, (uint32_t)sizeof({}));", literal, sn);

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

    w.func_signature("static inline ", "uint32_t", &func,
        &["SynqParseCtx *ctx", "uint32_t list_id", "uint32_t child"], " {");
    w.indent();
    w.line(&format!("return synq_parse_list_append(ctx, {}, list_id, child);", tag));
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

// ── Rust codegen ────────────────────────────────────────────────────────

/// Map a field to its Rust type name.
fn rust_field_type(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> String {
    match field.storage {
        Storage::Index => "u32".into(),
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

/// Check if a name is a Rust keyword that needs raw-identifier syntax.
fn is_rust_keyword(name: &str) -> bool {
    matches!(
        name,
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum"
            | "extern" | "false" | "fn" | "for" | "if" | "impl" | "in"
            | "let" | "loop" | "match" | "mod" | "move" | "mut" | "pub"
            | "ref" | "return" | "self" | "Self" | "static" | "struct"
            | "super" | "trait" | "true" | "type" | "unsafe" | "use"
            | "where" | "while" | "async" | "await" | "dyn" | "abstract"
            | "become" | "box" | "do" | "final" | "macro" | "override"
            | "priv" | "typeof" | "unsized" | "virtual" | "yield" | "try"
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

/// Generate Rust source for token type enum.
pub fn generate_rust_tokens(tokens: &[(String, u32)]) -> String {
    let mut out = String::new();
    writeln!(out, "// @generated by syntaqlite-codegen — DO NOT EDIT").unwrap();
    writeln!(out).unwrap();

    // TokenType enum
    writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]").unwrap();
    writeln!(out, "#[repr(u32)]").unwrap();
    writeln!(out, "pub enum TokenType {{").unwrap();
    for (name, value) in tokens {
        // Convert UPPER_SNAKE to PascalCase for Rust variant names
        let variant = token_name_to_pascal(name);
        writeln!(out, "    {} = {},", variant, value).unwrap();
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // from_raw conversion
    writeln!(out, "impl TokenType {{").unwrap();
    writeln!(out, "    pub fn from_raw(raw: u32) -> Option<TokenType> {{").unwrap();
    writeln!(out, "        match raw {{").unwrap();
    for (name, value) in tokens {
        let variant = token_name_to_pascal(name);
        writeln!(out, "            {} => Some(TokenType::{}),", value, variant).unwrap();
    }
    writeln!(out, "            _ => None,").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();

    out
}

/// Convert a token name like "LIKE_KW" to PascalCase "LikeKw".
fn token_name_to_pascal(name: &str) -> String {
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
/// NodeTag, node structs, `is_list_tag`, and `NodeRef::as_*` methods.
/// Static types (SourceSpan, NodeList, NodeRef struct) live in hand-written
/// `crate::nodes`.
pub fn generate_rust_nodes(items: &[Item]) -> String {
    let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
    let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();

    let mut out = String::new();
    writeln!(out, "// @generated by syntaqlite-codegen — DO NOT EDIT").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "use crate::nodes::{{NodeList, NodeRef, SourceSpan}};").unwrap();
    writeln!(out).unwrap();

    // Value enums
    for item in items {
        let Item::Enum { name, variants } = item else { continue };
        writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]").unwrap();
        writeln!(out, "#[repr(u32)]").unwrap();
        writeln!(out, "pub enum {} {{", name).unwrap();
        for (i, v) in variants.iter().enumerate() {
            let variant_name = variant_to_pascal(v);
            writeln!(out, "    {} = {},", variant_name, i).unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();

        writeln!(out, "impl {} {{", name).unwrap();
        writeln!(out, "    pub fn from_raw(raw: u32) -> Option<{}> {{", name).unwrap();
        writeln!(out, "        match raw {{").unwrap();
        for (i, v) in variants.iter().enumerate() {
            let variant_name = variant_to_pascal(v);
            writeln!(out, "            {} => Some({}::{}),", i, name, variant_name).unwrap();
        }
        writeln!(out, "            _ => None,").unwrap();
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    // Flags types
    for item in items {
        let Item::Flags { name, flags } = item else { continue };
        writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]").unwrap();
        writeln!(out, "#[repr(transparent)]").unwrap();
        writeln!(out, "pub struct {}(pub u8);", name).unwrap();
        writeln!(out).unwrap();
        writeln!(out, "impl {} {{", name).unwrap();
        let mut sorted: Vec<_> = flags.iter().collect();
        sorted.sort_by_key(|(_, v)| *v);
        for (flag_name, bit) in &sorted {
            let method = flag_name.to_lowercase();
            writeln!(out, "    pub fn {}(&self) -> bool {{", method).unwrap();
            writeln!(out, "        self.0 & (1 << {}) != 0", bit).unwrap();
            writeln!(out, "    }}").unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    // NodeTag enum
    writeln!(out, "#[derive(Debug, Clone, Copy, PartialEq, Eq)]").unwrap();
    writeln!(out, "#[repr(u32)]").unwrap();
    writeln!(out, "pub enum NodeTag {{").unwrap();
    writeln!(out, "    Null = 0,").unwrap();
    let mut tag_index = 1u32;
    let mut list_tags: Vec<String> = Vec::new();
    for item in items {
        match item {
            Item::Node { name, .. } => {
                writeln!(out, "    {} = {},", name, tag_index).unwrap();
                tag_index += 1;
            }
            Item::List { name, .. } => {
                writeln!(out, "    {} = {},", name, tag_index).unwrap();
                list_tags.push(name.clone());
                tag_index += 1;
            }
            _ => {}
        }
    }
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    writeln!(out, "impl NodeTag {{").unwrap();
    writeln!(out, "    pub fn from_raw(raw: u32) -> Option<NodeTag> {{").unwrap();
    writeln!(out, "        match raw {{").unwrap();
    writeln!(out, "            0 => Some(NodeTag::Null),").unwrap();
    let mut idx = 1u32;
    for item in items {
        let name = match item {
            Item::Node { name, .. } | Item::List { name, .. } => name,
            _ => continue,
        };
        writeln!(out, "            {} => Some(NodeTag::{}),", idx, name).unwrap();
        idx += 1;
    }
    writeln!(out, "            _ => None,").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();
    writeln!(out, "    pub fn is_list(&self) -> bool {{").unwrap();
    writeln!(out, "        matches!(self, {}", list_tags.iter().map(|t| format!("NodeTag::{}", t)).collect::<Vec<_>>().join(" | ")).unwrap();
    writeln!(out, "        )").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out, "}}").unwrap();
    writeln!(out).unwrap();

    // Node structs
    for item in items {
        let Item::Node { name, fields, .. } = item else { continue };
        writeln!(out, "#[derive(Debug, Clone, Copy)]").unwrap();
        writeln!(out, "#[repr(C)]").unwrap();
        writeln!(out, "pub struct {} {{", name).unwrap();
        writeln!(out, "    pub tag: u32,").unwrap();
        for field in fields {
            let ty = rust_field_type(field, &enum_names, &flags_names);
            let fname = rust_field_name(&field.name);
            writeln!(out, "    pub {}: {},", fname, ty).unwrap();
        }
        writeln!(out, "}}").unwrap();
        writeln!(out).unwrap();
    }

    // Generated NodeRef methods (extends hand-written NodeRef struct).
    // SAFETY (applies to all casts below): NodeRef is only constructed via
    // from_raw() with a valid, non-null arena pointer. The tag check confirms
    // the concrete type before casting. All node structs are #[repr(C)] and
    // match the C layout written by the arena allocator.
    writeln!(out, "impl<'a> NodeRef<'a> {{").unwrap();
    writeln!(out, "    pub fn as_list(&self) -> Option<&'a NodeList> {{").unwrap();
    writeln!(out, "        if self.tag().is_list() {{").unwrap();
    writeln!(out, "            // SAFETY: tag confirms this is a list node; layout matches NodeList.").unwrap();
    writeln!(out, "            Some(unsafe {{ &*(self.ptr() as *const NodeList) }})").unwrap();
    writeln!(out, "        }} else {{").unwrap();
    writeln!(out, "            None").unwrap();
    writeln!(out, "        }}").unwrap();
    writeln!(out, "    }}").unwrap();
    writeln!(out).unwrap();
    for item in items {
        let Item::Node { name, .. } = item else { continue };
        let method = format!("as_{}", pascal_to_snake(name));
        writeln!(out, "    pub fn {}(&self) -> Option<&'a {}> {{", method, name).unwrap();
        writeln!(out, "        if self.tag() == NodeTag::{} {{", name).unwrap();
        writeln!(out, "            // SAFETY: tag check above confirms the concrete type.").unwrap();
        writeln!(out, "            Some(unsafe {{ &*(self.ptr() as *const {}) }})", name).unwrap();
        writeln!(out, "        }} else {{").unwrap();
        writeln!(out, "            None").unwrap();
        writeln!(out, "        }}").unwrap();
        writeln!(out, "    }}").unwrap();
        writeln!(out).unwrap();
    }
    writeln!(out, "}}").unwrap();

    out
}

/// Convert a UPPER_CASE variant to PascalCase.
/// E.g. "ROLLBACK" → "Rollback", "UNION_ALL" → "UnionAll",
/// "IS_NOT_DISTINCT" → "IsNotDistinct"
fn variant_to_pascal(name: &str) -> String {
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
