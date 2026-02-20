// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fmt::{Display, Formatter};

use crate::writers::c_writer::CWriter;

use super::{AstModel, NodeLikeRef};

#[derive(Debug, Clone)]
pub enum CFmtCodegenError {
    FmtCompile(crate::fmt_compiler::FmtCompileError),
}

impl Display for CFmtCodegenError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FmtCompile(err) => Display::fmt(err, f),
        }
    }
}

impl std::error::Error for CFmtCodegenError {}

pub fn generate_c_fmt_tables(model: &AstModel<'_>) -> Result<String, CFmtCodegenError> {
    let compiled = crate::fmt_compiler::try_compile_all(model.items())
        .map_err(CFmtCodegenError::FmtCompile)?;

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

    let mut op_pool: Vec<syntaqlite_runtime::fmt::bytecode::RawOp> = Vec::new();
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

    let mut ordinal_map: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
    let mut next_ordinal = 1usize;
    for item in model.node_like_items() {
        let name = match item {
            NodeLikeRef::Node(node) => node.name,
            NodeLikeRef::List(list) => list.name,
        };
        ordinal_map.insert(name, next_ordinal);
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
