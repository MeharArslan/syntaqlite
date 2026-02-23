// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;

use crate::synq_parser::{Field, Storage};
use crate::util::upper_snake_to_pascal;
use crate::writers::rust_writer::RustWriter;

use super::{AstModel, NodeLikeRef};

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

fn emit_rust_node_tag_type(w: &mut RustWriter, model: &AstModel<'_>) {
    let mut tag_names: Vec<String> = Vec::new();
    let mut list_tags: Vec<String> = Vec::new();
    for item in model.node_like_items() {
        match item {
            NodeLikeRef::Node(node) => tag_names.push(node.name.to_string()),
            NodeLikeRef::List(list) => {
                tag_names.push(list.name.to_string());
                list_tags.push(list.name.to_string());
            }
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
    model: &AstModel<'_>,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
    struct_visibility: &str,
    field_visibility: &str,
    field_type: fn(&Field, &HashSet<&str>, &HashSet<&str>) -> String,
) {
    for node in model.nodes() {
        let name = node.name;
        let fields = node.fields;
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

fn emit_rust_node_tag_accessor(w: &mut RustWriter, node_like_items: &[NodeLikeRef<'_>]) {
    w.doc_comment("The node's tag.");
    w.open_block("pub fn tag(&self) -> NodeTag {");
    w.open_block("match self {");
    for item in node_like_items {
        let name = match item {
            NodeLikeRef::Node(node) => node.name,
            NodeLikeRef::List(list) => list.name,
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
    w.line("#[doc(hidden)]");
    w.open_block("pub fn from_raw(raw: u32) -> Option<TokenType> {");
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

/// Generate Rust source for the FFI layer (`ffi.rs`).
///
/// Emits `pub(crate)` `#[repr(C)]` node structs and the `Bool` enum.
/// Enum/flags types are referenced via `super::ast::`.
pub fn generate_rust_ffi_nodes(model: &AstModel<'_>) -> String {
    let enum_names = model.enum_names();
    let flags_names = model.flags_names();

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
        model,
        enum_names,
        flags_names,
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
pub fn generate_rust_ast(model: &AstModel<'_>) -> String {
    let enum_names = model.enum_names();
    let flags_names = model.flags_names();
    let node_names = model.node_names();
    let list_names = model.list_names();
    let abstract_items = model.abstract_items();

    let mut w = RustWriter::new();
    w.file_header();
    w.lines("
        pub use syntaqlite_runtime::parser::{Comment, CommentKind, FromArena, NodeId, NodeReader, SourceSpan, TypedList};
        pub(crate) use syntaqlite_runtime::parser::NodeList;
        use std::marker::PhantomData;
    ");
    w.newline();

    // Value enums (skip Bool — it lives in ffi.rs)
    for item in model.enums() {
        let name = item.name;
        let variants = item.variants;
        if name == "Bool" {
            continue;
        }
        emit_rust_value_enum(&mut w, name, variants);
    }

    // Flags types
    for item in model.flags() {
        emit_rust_flags_type(&mut w, item.name, item.flags);
    }

    // NodeTag enum
    emit_rust_node_tag_type(&mut w, model);

    // Abstract type enums (Expr, Stmt, etc.)
    for &(abs_name, members) in abstract_items {
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
    for node in model.nodes() {
        let name = node.name;
        let fields = node.fields;

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
                rust_view_return_type(field, enum_names, flags_names, node_names, list_names);
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
    for list in model.lists() {
        let name = list.name;
        let child_type = list.child_type;
        let ct = child_type;
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
    for item in model.node_like_items() {
        match item {
            NodeLikeRef::Node(node) => {
                w.line(&format!("{}({}<'a>),", node.name, node.name));
            }
            NodeLikeRef::List(list) => {
                w.doc_comment(&format!("List of {}", list.child_type));
                w.line(&format!("{}({}<'a>),", list.name, list.name));
            }
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
    for item in model.node_like_items() {
        match item {
            NodeLikeRef::Node(node) => {
                let name = node.name;
                w.line(&format!("NodeTag::{n} => Node::{n}({n} {{ raw: &*(ptr as *const crate::ffi::{n}), reader, id }}),", n = name));
            }
            NodeLikeRef::List(list) => {
                let name = list.name;
                w.line(&format!("NodeTag::{n} => Node::{n}(TypedList::new(&*(ptr as *const NodeList), reader)),", n = name));
            }
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
    emit_rust_node_tag_accessor(&mut w, model.node_like_items());

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
    for node in model.nodes() {
        w.line(&format!(
            "Node::{n}(n) => std::fmt::Display::fmt(n, f),",
            n = node.name
        ));
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
