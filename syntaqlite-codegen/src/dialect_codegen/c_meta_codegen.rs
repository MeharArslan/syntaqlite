// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use crate::synq_parser::{Field, Storage};
use crate::util::naming::pascal_to_snake;
use crate::writers::c_writer::CWriter;

use super::c_common::c_type_name;
use super::{AstModel, NodeLikeRef};

#[derive(Debug, Clone)]
pub enum CMetaCodegenError {
    UnknownInlineType(String),
}

impl Display for CMetaCodegenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownInlineType(t) => {
                write!(f, "unknown inline type for C field meta codegen: {t}")
            }
        }
    }
}

impl std::error::Error for CMetaCodegenError {}

pub fn generate_c_field_metadata(
    model: &AstModel<'_>,
    dialect: &str,
) -> Result<String, CMetaCodegenError> {
    let enum_names = model.enum_names();
    let flags_names = model.flags_names();

    let mut w = CWriter::new();
    w.file_header();
    w.header_guard_start("SYNTAQLITE_DIALECT_META_H");
    w.include_system("stddef.h");
    w.include_local("syntaqlite/dialect.h");
    w.include_local(&format!("syntaqlite_{dialect}/{dialect}_node.h"));
    w.newline();

    for item in model.enums() {
        let name = item.name;
        let variants = item.variants;
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

    for item in model.flags() {
        let name = item.name;
        let flags = item.flags;
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

    for node in model.nodes() {
        let name = node.name;
        let fields = node.fields;
        if fields.is_empty() {
            continue;
        }
        let sn = c_type_name(name);
        let var = format!("field_meta_{}", pascal_to_snake(name));
        w.line(&format!("static const SyntaqliteFieldMeta {}[] = {{", var));
        w.indent();
        for field in fields {
            let kind = c_field_kind(field, enum_names, flags_names)?;
            let (display, display_count) = c_field_display(field, enum_names, flags_names);
            w.line(&format!(
                "{{offsetof({}, {}), {}, \"{}\", {}, {}}},",
                sn, field.name, kind, field.name, display, display_count,
            ));
        }
        w.dedent();
        w.line("};");
        w.newline();
    }

    w.section("Node Names");
    w.line("static const char* const ast_meta_node_names[] = {");
    w.indent();
    w.line("\"Null\",");
    for item in model.node_like_items() {
        let name = match item {
            NodeLikeRef::Node(node) => node.name,
            NodeLikeRef::List(list) => list.name,
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
    for node in model.nodes() {
        if node.fields.is_empty() {
            w.line(&format!("NULL, /* {} */", node.name));
        } else {
            w.line(&format!(
                "field_meta_{}, /* {} */",
                pascal_to_snake(node.name),
                node.name
            ));
        }
    }
    for list in model.lists() {
        w.line(&format!("NULL, /* {} */", list.name));
    }
    w.dedent();
    w.line("};");
    w.newline();

    w.line("static const uint8_t ast_meta_field_meta_counts[] = {");
    w.indent();
    w.line("0, /* Null */");
    for node in model.nodes() {
        w.line(&format!("{}, /* {} */", node.fields.len(), node.name));
    }
    for list in model.lists() {
        w.line(&format!("0, /* {} */", list.name));
    }
    w.dedent();
    w.line("};");
    w.newline();

    w.section("List Tags");
    w.line("static const uint8_t ast_meta_list_tags[] = {");
    w.indent();
    w.line("0, /* Null */");
    for node in model.nodes() {
        w.line(&format!("0, /* {} */", node.name));
    }
    for list in model.lists() {
        w.line(&format!("1, /* {} */", list.name));
    }
    w.dedent();
    w.line("};");
    w.newline();

    w.header_guard_end("SYNTAQLITE_DIALECT_META_H");
    Ok(w.finish())
}

fn c_field_kind(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> Result<&'static str, CMetaCodegenError> {
    match field.storage {
        Storage::Index => Ok("SYNTAQLITE_FIELD_NODE_ID"),
        Storage::Inline => {
            let t = &field.type_name;
            if t == "SyntaqliteSourceSpan" {
                Ok("SYNTAQLITE_FIELD_SPAN")
            } else if t == "Bool" {
                Ok("SYNTAQLITE_FIELD_BOOL")
            } else if flags_names.contains(t.as_str()) {
                Ok("SYNTAQLITE_FIELD_FLAGS")
            } else if enum_names.contains(t.as_str()) {
                Ok("SYNTAQLITE_FIELD_ENUM")
            } else {
                Err(CMetaCodegenError::UnknownInlineType(t.clone()))
            }
        }
    }
}

fn c_field_display(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> (String, String) {
    match field.storage {
        Storage::Inline => {
            let t = &field.type_name;
            if enum_names.contains(t.as_str()) || flags_names.contains(t.as_str()) {
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

fn bit_position(value: u32) -> u32 {
    if value == 0 {
        return 0;
    }
    value.trailing_zeros()
}
