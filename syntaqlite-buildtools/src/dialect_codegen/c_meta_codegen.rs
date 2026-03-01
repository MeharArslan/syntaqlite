// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;
use std::fmt::{Display, Formatter};

use crate::util::c_writer::CWriter;
use crate::util::pascal_to_snake;
use crate::util::synq_parser::{Field, Storage};

use super::{AstModel, NodeLikeRef, c_type_name};

/// Sentinel value indicating a field is absent in the schema metadata table.
const FIELD_ABSENT: u8 = 0xFF;

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

#[derive(Debug, Clone)]
pub enum CFmtCodegenError {
    FmtCompile(super::fmt_compiler::FmtCompileError),
}

impl Display for CFmtCodegenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FmtCompile(err) => Display::fmt(err, f),
        }
    }
}

impl std::error::Error for CFmtCodegenError {}

impl AstModel<'_> {
    pub fn generate_c_fmt_tables(&self) -> Result<String, CFmtCodegenError> {
        let compiled =
            super::fmt_compiler::try_compile_all(self).map_err(CFmtCodegenError::FmtCompile)?;

        let mut w = CWriter::new();
        w.file_header();
        w.header_guard_start("SYNTAQLITE_DIALECT_FMT_H");
        w.include_system("stdint.h");
        w.newline();

        w.line("static const char* const fmt_strings[] = {");
        w.indent();
        for s in &compiled.strings {
            let escaped = s
                .replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n");
            w.line(&format!("\"{}\",", escaped));
        }
        w.dedent();
        w.line("};");
        w.newline();

        w.line("static const uint16_t fmt_string_lens[] = {");
        w.indent();
        for chunk in compiled.strings.chunks(16) {
            let vals: Vec<String> = chunk.iter().map(|s| format!("{}", s.len())).collect();
            w.line(&format!("{},", vals.join(",")));
        }
        w.dedent();
        w.line("};");
        w.newline();

        w.line("static const uint16_t fmt_enum_display[] = {");
        w.indent();
        for chunk in compiled.enum_display.chunks(16) {
            let vals: Vec<String> = chunk.iter().map(|v| format!("{}", v)).collect();
            w.line(&format!("{},", vals.join(",")));
        }
        w.dedent();
        w.line("};");
        w.newline();

        let mut op_pool: Vec<syntaqlite::fmt::bytecode::RawOp> = Vec::new();
        let mut node_ranges: Vec<(&str, u16, u16)> = Vec::new();

        for cn in &compiled.nodes {
            let offset = op_pool.len() as u16;
            let length = cn.ops.len() as u16;
            op_pool.extend_from_slice(&cn.ops);
            node_ranges.push((&cn.name, offset, length));
        }

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

        let mut dispatch_table: Vec<u32> = vec![0xFFFF_0000; compiled.tag_count];
        let mut ordinal_map: std::collections::HashMap<&str, usize> =
            std::collections::HashMap::new();
        let mut next_ordinal = 1usize;
        for item in self.node_like_items() {
            ordinal_map.insert(item.name(), next_ordinal);
            next_ordinal += 1;
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
        Ok(w.finish())
    }

    pub fn generate_c_field_metadata(&self, dialect: &str) -> Result<String, CMetaCodegenError> {
        let enum_names = self.enum_names();
        let flags_names = self.flags_names();

        let mut w = CWriter::new();
        w.file_header();
        w.header_guard_start("SYNTAQLITE_DIALECT_META_H");
        w.include_system("stddef.h");
        w.include_local("syntaqlite/dialect.h");
        w.include_local(&format!("syntaqlite_{dialect}/{dialect}_node.h"));
        w.newline();

        for item in self.enums() {
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

        for item in self.flags() {
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

        for node in self.nodes() {
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
        for item in self.node_like_items() {
            w.line(&format!("\"{}\",", item.name()));
        }
        w.dedent();
        w.line("};");
        w.newline();

        // Build field_meta, field_meta_counts, and list_tags arrays in a single pass.
        let mut field_meta_entries = vec!["NULL, /* Null */".to_string()];
        let mut field_count_entries = vec!["0, /* Null */".to_string()];
        let mut list_tag_entries = vec!["0, /* Null */".to_string()];

        for item in self.node_like_items() {
            match item {
                NodeLikeRef::Node(node) => {
                    if node.fields.is_empty() {
                        field_meta_entries.push(format!("NULL, /* {} */", node.name));
                    } else {
                        field_meta_entries.push(format!(
                            "field_meta_{}, /* {} */",
                            pascal_to_snake(node.name),
                            node.name
                        ));
                    }
                    field_count_entries.push(format!("{}, /* {} */", node.fields.len(), node.name));
                    list_tag_entries.push(format!("0, /* {} */", node.name));
                }
                NodeLikeRef::List(list) => {
                    field_meta_entries.push(format!("NULL, /* {} */", list.name));
                    field_count_entries.push(format!("0, /* {} */", list.name));
                    list_tag_entries.push(format!("1, /* {} */", list.name));
                }
            }
        }

        w.section("Field Meta Dispatch");
        w.line("static const SyntaqliteFieldMeta* const ast_meta_field_meta[] = {");
        w.indent();
        for entry in &field_meta_entries {
            w.line(entry);
        }
        w.dedent();
        w.line("};");
        w.newline();

        w.line("static const uint8_t ast_meta_field_meta_counts[] = {");
        w.indent();
        for entry in &field_count_entries {
            w.line(entry);
        }
        w.dedent();
        w.line("};");
        w.newline();

        w.section("List Tags");
        w.line("static const uint8_t ast_meta_list_tags[] = {");
        w.indent();
        for entry in &list_tag_entries {
            w.line(entry);
        }
        w.dedent();
        w.line("};");
        w.newline();

        w.header_guard_end("SYNTAQLITE_DIALECT_META_H");
        Ok(w.finish())
    }

    pub fn generate_c_schema_contributions(&self) -> String {
        use crate::util::synq_parser::SchemaKind;

        let mut entries = Vec::new();

        for node in self.nodes() {
            let Some(schema) = node.schema else {
                continue;
            };
            let tag = self.tag_for(node.name);

            let kind = match schema.kind {
                SchemaKind::Table => 0,
                SchemaKind::View => 1,
                SchemaKind::Function => 2,
                SchemaKind::Import => 3,
            };

            let resolve_field_index = |key: &str| -> u8 {
                match schema.param(key) {
                    Some(field_name) => node
                        .fields
                        .iter()
                        .position(|f| f.name == field_name)
                        .map(|i| i as u8)
                        .unwrap_or(FIELD_ABSENT),
                    None => FIELD_ABSENT,
                }
            };

            entries.push((
                tag,
                kind,
                resolve_field_index("name"),
                resolve_field_index("columns"),
                resolve_field_index("as_select"),
                resolve_field_index("args"),
            ));
        }

        let mut w = CWriter::new();
        if entries.is_empty() {
            return w.finish();
        }

        w.newline();
        w.section("Schema Contributions");
        w.line("#define SYNTAQLITE_HAS_SCHEMA_CONTRIBUTIONS");
        w.line("static const SyntaqliteSchemaContribution schema_contributions[] = {");
        w.indent();
        for (tag, kind, name_f, cols_f, sel_f, args_f) in &entries {
            w.line(&format!(
                "{{{}, {}, {}, {}, {}, {}, {{0}}}},",
                tag, kind, name_f, cols_f, sel_f, args_f
            ));
        }
        w.dedent();
        w.line("};");
        w.newline();

        w.finish()
    }
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
