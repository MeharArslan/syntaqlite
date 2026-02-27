// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;

use crate::util::synq_parser::{Field, Fmt, Item};

mod c_dialect;
mod c_meta_codegen;
mod c_nodes_codegen;
pub(crate) mod fmt_compiler;
mod rust_ast;
mod rust_dialect;

pub use c_dialect::{
    generate_dialect_c, generate_dialect_dispatch_h, generate_dialect_h, generate_parse_h,
    generate_token_categories_header, generate_tokenize_h,
};
pub use c_meta_codegen::{
    CFmtCodegenError, CMetaCodegenError, generate_c_field_metadata, generate_c_fmt_tables,
};
pub use c_nodes_codegen::{generate_ast_builder_header, generate_ast_nodes_header};
pub use rust_ast::{generate_rust_ast, generate_rust_ffi_nodes, generate_rust_tokens};
pub use rust_dialect::{generate_rust_lib, generate_rust_wrappers};

pub use crate::util::pascal_case;

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
        let mut enum_names: HashSet<&str> = HashSet::new();
        let mut flags_names: HashSet<&str> = HashSet::new();
        let mut node_names: HashSet<&str> = HashSet::new();
        let mut list_names: HashSet<&str> = HashSet::new();
        let mut enums: Vec<EnumRef<'a>> = Vec::new();
        let mut flags: Vec<FlagsRef<'a>> = Vec::new();
        let mut nodes: Vec<NodeRef<'a>> = Vec::new();
        let mut lists: Vec<ListRef<'a>> = Vec::new();
        let mut node_like_items: Vec<NodeLikeRef<'a>> = Vec::new();
        let mut abstract_items: Vec<(&str, &[String])> = Vec::new();

        for item in items {
            match item {
                Item::Enum { name, variants } => {
                    let name = name.as_str();
                    enum_names.insert(name);
                    enums.push(EnumRef {
                        name,
                        variants: variants.as_slice(),
                    });
                }
                Item::Flags {
                    name,
                    flags: values,
                } => {
                    let name = name.as_str();
                    flags_names.insert(name);
                    flags.push(FlagsRef {
                        name,
                        flags: values.as_slice(),
                    });
                }
                Item::Node { name, fields, fmt } => {
                    let node = NodeRef {
                        name: name.as_str(),
                        fields: fields.as_slice(),
                        fmt: fmt.as_deref(),
                    };
                    node_names.insert(node.name);
                    nodes.push(node);
                    node_like_items.push(NodeLikeRef::Node(node));
                }
                Item::List {
                    name,
                    child_type,
                    fmt,
                } => {
                    let list = ListRef {
                        name: name.as_str(),
                        child_type: child_type.as_str(),
                        fmt: fmt.as_deref(),
                    };
                    list_names.insert(list.name);
                    lists.push(list);
                    node_like_items.push(NodeLikeRef::List(list));
                }
                Item::Abstract { name, members } => {
                    abstract_items.push((name.as_str(), members.as_slice()));
                }
            }
        }

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
