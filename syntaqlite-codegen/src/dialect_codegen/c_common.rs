// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;

use crate::synq_parser::{Field, Storage};
use crate::util::naming::{pascal_to_snake, upper_snake};

pub(super) fn c_type_name(name: &str) -> String {
    format!("Syntaqlite{}", name)
}

pub(super) fn tag_name(name: &str) -> String {
    format!("SYNTAQLITE_NODE_{}", upper_snake(name))
}

pub(super) fn builder_name(name: &str) -> String {
    format!("synq_parse_{}", pascal_to_snake(name))
}

pub(super) fn field_c_type(
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

pub(super) fn refs_i32(owned: &[(String, Option<i32>)]) -> Vec<(&str, Option<i32>)> {
    owned.iter().map(|(s, v)| (s.as_str(), *v)).collect()
}

pub(super) fn range_fields(fields: &[Field]) -> Vec<(&str, u8)> {
    fields
        .iter()
        .filter_map(|f| match f.storage {
            Storage::Index => Some((f.name.as_str(), 0)),
            Storage::Inline if f.type_name == "SyntaqliteSourceSpan" => Some((f.name.as_str(), 1)),
            _ => None,
        })
        .collect()
}
