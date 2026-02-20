// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;

use crate::node_parser::{Field, Fmt, Item};

mod c_ast;
mod c_dialect;
mod rust_ast;
mod rust_dialect;

pub use c_ast::{
    CCodegenError, generate_ast_builder_h, generate_ast_builder_h_from_model, generate_ast_nodes_h,
    generate_ast_nodes_h_from_model, generate_c_field_meta, generate_c_field_meta_from_model,
    generate_c_fmt_arrays, try_generate_c_field_meta, try_generate_c_field_meta_from_model,
    try_generate_c_field_meta_from_model_typed, try_generate_c_field_meta_typed,
    try_generate_c_fmt_arrays, try_generate_c_fmt_arrays_typed,
};
pub use c_dialect::{
    generate_dialect_c, generate_dialect_dispatch_h, generate_dialect_h, generate_parse_h,
    generate_tokenize_h,
};
pub use rust_ast::{
    generate_rust_ast, generate_rust_ast_from_model, generate_rust_ffi_nodes,
    generate_rust_ffi_nodes_from_model, generate_rust_tokens,
};
pub use rust_dialect::{generate_rust_lib, generate_rust_wrappers};

pub use crate::util::naming::pascal_case;

pub struct AstModel<'a> {
    items: &'a [Item],
    enum_names: HashSet<&'a str>,
    flags_names: HashSet<&'a str>,
    node_names: HashSet<&'a str>,
    list_names: HashSet<&'a str>,
    enums: Vec<EnumRef<'a>>,
    flags: Vec<FlagsRef<'a>>,
    nodes: Vec<NodeRef<'a>>,
    lists: Vec<ListRef<'a>>,
    node_like_items: Vec<NodeLikeRef<'a>>,
    abstract_items: Vec<(&'a str, &'a [String])>,
}

#[derive(Clone, Copy)]
pub struct EnumRef<'a> {
    pub name: &'a str,
    pub variants: &'a [String],
}

#[derive(Clone, Copy)]
pub struct FlagsRef<'a> {
    pub name: &'a str,
    pub flags: &'a [(String, u32)],
}

#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
    pub name: &'a str,
    pub fields: &'a [Field],
    pub fmt: Option<&'a [Fmt]>,
}

#[derive(Clone, Copy)]
pub struct ListRef<'a> {
    pub name: &'a str,
    pub child_type: &'a str,
    pub fmt: Option<&'a [Fmt]>,
}

#[derive(Clone, Copy)]
pub enum NodeLikeRef<'a> {
    Node(NodeRef<'a>),
    List(ListRef<'a>),
}

impl<'a> AstModel<'a> {
    pub fn new(items: &'a [Item]) -> Self {
        let enum_names: HashSet<&str> = items.iter().filter_map(Item::as_enum_name).collect();
        let flags_names: HashSet<&str> = items.iter().filter_map(Item::as_flags_name).collect();
        let node_names: HashSet<&str> = items
            .iter()
            .filter_map(|i| match i {
                Item::Node { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        let list_names: HashSet<&str> = items
            .iter()
            .filter_map(|i| match i {
                Item::List { name, .. } => Some(name.as_str()),
                _ => None,
            })
            .collect();
        let enums: Vec<EnumRef<'a>> = items
            .iter()
            .filter_map(|i| match i {
                Item::Enum { name, variants } => Some(EnumRef {
                    name: name.as_str(),
                    variants: variants.as_slice(),
                }),
                _ => None,
            })
            .collect();
        let flags: Vec<FlagsRef<'a>> = items
            .iter()
            .filter_map(|i| match i {
                Item::Flags { name, flags } => Some(FlagsRef {
                    name: name.as_str(),
                    flags: flags.as_slice(),
                }),
                _ => None,
            })
            .collect();
        let nodes: Vec<NodeRef<'a>> = items
            .iter()
            .filter_map(|i| match i {
                Item::Node { name, fields, fmt } => Some(NodeRef {
                    name: name.as_str(),
                    fields: fields.as_slice(),
                    fmt: fmt.as_deref(),
                }),
                _ => None,
            })
            .collect();
        let lists: Vec<ListRef<'a>> = items
            .iter()
            .filter_map(|i| match i {
                Item::List {
                    name,
                    child_type,
                    fmt,
                } => Some(ListRef {
                    name: name.as_str(),
                    child_type: child_type.as_str(),
                    fmt: fmt.as_deref(),
                }),
                _ => None,
            })
            .collect();
        let node_like_items: Vec<NodeLikeRef<'a>> = items
            .iter()
            .filter_map(|i| match i {
                Item::Node { name, fields, fmt } => Some(NodeLikeRef::Node(NodeRef {
                    name: name.as_str(),
                    fields: fields.as_slice(),
                    fmt: fmt.as_deref(),
                })),
                Item::List {
                    name,
                    child_type,
                    fmt,
                } => Some(NodeLikeRef::List(ListRef {
                    name: name.as_str(),
                    child_type: child_type.as_str(),
                    fmt: fmt.as_deref(),
                })),
                _ => None,
            })
            .collect();
        let abstract_items: Vec<(&str, &[String])> = items
            .iter()
            .filter_map(|i| match i {
                Item::Abstract { name, members } => Some((name.as_str(), members.as_slice())),
                _ => None,
            })
            .collect();
        Self {
            items,
            enum_names,
            flags_names,
            node_names,
            list_names,
            enums,
            flags,
            nodes,
            lists,
            node_like_items,
            abstract_items,
        }
    }

    pub fn items(&self) -> &'a [Item] {
        self.items
    }

    pub fn enum_names(&self) -> &HashSet<&'a str> {
        &self.enum_names
    }

    pub fn flags_names(&self) -> &HashSet<&'a str> {
        &self.flags_names
    }

    pub fn node_names(&self) -> &HashSet<&'a str> {
        &self.node_names
    }

    pub fn list_names(&self) -> &HashSet<&'a str> {
        &self.list_names
    }

    pub fn abstract_items(&self) -> &[(&'a str, &'a [String])] {
        &self.abstract_items
    }

    pub fn enums(&self) -> &[EnumRef<'a>] {
        &self.enums
    }

    pub fn flags(&self) -> &[FlagsRef<'a>] {
        &self.flags
    }

    pub fn nodes(&self) -> &[NodeRef<'a>] {
        &self.nodes
    }

    pub fn lists(&self) -> &[ListRef<'a>] {
        &self.lists
    }

    pub fn node_like_items(&self) -> &[NodeLikeRef<'a>] {
        &self.node_like_items
    }
}
