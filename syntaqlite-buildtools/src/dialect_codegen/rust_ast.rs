// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::collections::HashSet;

use crate::util::pascal_case;
use crate::util::rust_writer::RustWriter;
use crate::util::synq_parser::{Field, Storage};

use super::{AstModel, NodeLikeRef};

// ── Rust codegen ────────────────────────────────────────────────────────

/// Map a field to its Rust type name (for FFI structs in ffi.rs).
fn rust_ffi_field_type(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
) -> String {
    match field.storage {
        Storage::Index => "AnyNodeId".into(),
        Storage::Inline => {
            let t = &field.type_name;
            if t == "Bool" {
                "Bool".into()
            } else if enum_names.contains(t.as_str()) || flags_names.contains(t.as_str()) {
                format!("super::ast::{t}")
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
    _enum_names: &HashSet<&str>,
    _flags_names: &HashSet<&str>,
    _node_names: &HashSet<&str>,
    _list_names: &HashSet<&str>,
) -> String {
    match field.storage {
        Storage::Index => {
            let t = field.type_name.as_str();
            format!("Option<{t}<'a>>")
        }
        Storage::Inline => {
            let t = &field.type_name;
            if t == "Bool" {
                "bool".into()
            } else if t == "SyntaqliteSourceSpan" {
                "&'a str".into()
            } else {
                t.clone()
            }
        }
    }
}

/// Generate the accessor body for a view struct field.
fn rust_view_accessor_body(field: &Field, ffi_path: &str) -> String {
    let fname = rust_field_name(&field.name);
    match field.storage {
        Storage::Index => {
            format!("GrammarNodeType::from_arena(self.reader, self.raw.{fname})")
        }
        Storage::Inline => {
            let t = &field.type_name;
            if t == "Bool" {
                format!("self.raw.{fname} == {ffi_path}::Bool::True")
            } else if t == "SyntaqliteSourceSpan" {
                format!("self.raw.{fname}.as_str(self.reader.source())")
            } else {
                format!("self.raw.{fname}")
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
        format!("r#{name}")
    } else {
        name.to_string()
    }
}

fn emit_rust_value_enum(w: &mut RustWriter, name: &str, variants: &[String]) {
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    w.line("#[repr(u32)]");
    w.open_block(&format!("pub enum {name} {{"));
    for (i, v) in variants.iter().enumerate() {
        let variant_name = pascal_case(v);
        w.line(&format!("{variant_name} = {i},"));
    }
    w.close_block("}");
    w.newline();

    w.open_block(&format!("impl {name} {{"));
    w.open_block("pub fn as_str(&self) -> &'static str {");
    w.open_block("match self {");
    for v in variants {
        let variant_name = pascal_case(v);
        w.line(&format!("{name}::{variant_name} => \"{v}\","));
    }
    w.close_block("}");
    w.close_block("}");
    w.close_block("}");
    w.newline();
}

fn emit_rust_flags_type(w: &mut RustWriter, name: &str, flags: &[(String, u32)]) {
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]");
    w.line("#[repr(transparent)]");
    w.line(&format!("pub struct {name}(pub u8);"));
    w.newline();

    w.open_block(&format!("impl {name} {{"));
    let mut sorted: Vec<_> = flags.iter().collect();
    sorted.sort_by_key(|(_, v)| *v);
    for (flag_name, bit) in &sorted {
        let method = flag_name.to_lowercase();
        w.open_block(&format!("pub fn {method}(&self) -> bool {{"));
        w.line(&format!("self.0 & {bit} != 0"));
        w.close_block("}");
    }
    w.newline();
    w.close_block("}");
    w.newline();
}

/// Emit the `FooKind` plain enum and `FooLike` trait for a value enum (base crate, `ast_traits.rs`).
///
/// `FooKind` holds only the base variants. `TypedDialectEnv` extensions that add new variants return
/// `None` from `kind()`, allowing generic code to degrade gracefully.
fn emit_rust_value_enum_like_trait(w: &mut RustWriter, name: &str, variants: &[String]) {
    // ── FooKind enum ──
    w.doc_comment(&format!(
        "Base variants of `{name}`. Used for exhaustive pattern matching in generic code."
    ));
    w.doc_comment(&format!(
        "Grammar extensions that add variants beyond this set return `None` from `{name}Like::kind`."
    ));
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    w.open_block(&format!("pub enum {name}Kind {{"));
    for v in variants {
        w.line(&format!("{},", pascal_case(v)));
    }
    w.close_block("}");
    w.newline();

    // ── FooLike trait ──
    w.doc_comment(&format!(
        "Trait for `{name}`-compatible values. Dialects may define their own type and implement this."
    ));
    w.open_block(&format!(
        "pub trait {name}Like: Copy + PartialEq + Eq + std::fmt::Debug {{"
    ));
    w.line("fn as_str(&self) -> &'static str;");
    w.doc_comment(&format!(
        "Match against base `{name}Kind` variants. Returns `None` for dialect-specific extensions."
    ));
    w.line(&format!("fn kind(&self) -> Option<{name}Kind>;"));
    w.close_block("}");
    w.newline();
}

/// Emit the `FooLike` trait for a flags type (base crate, `ast_traits.rs`).
fn emit_rust_flags_like_trait(w: &mut RustWriter, name: &str, flags: &[(String, u32)]) {
    w.doc_comment(&format!(
        "Trait for `{name}`-compatible flags. Dialects may define their own type and implement this."
    ));
    w.open_block(&format!(
        "pub trait {name}Like: Copy + PartialEq + Eq + Default + std::fmt::Debug {{"
    ));
    let mut sorted: Vec<_> = flags.iter().collect();
    sorted.sort_by_key(|(_, v)| *v);
    for (flag_name, _) in &sorted {
        let method = flag_name.to_lowercase();
        w.line(&format!("fn {method}(&self) -> bool;"));
    }
    w.close_block("}");
    w.newline();
}

/// Emit `impl FooLike for Foo` for a value enum (dialect crate, `ast.rs`).
fn emit_rust_value_enum_like_impl(
    w: &mut RustWriter,
    traits_path: &str,
    name: &str,
    variants: &[String],
) {
    w.open_block(&format!("impl {traits_path}::{name}Like for {name} {{"));

    w.open_block("fn as_str(&self) -> &'static str {");
    w.line("self.as_str()");
    w.close_block("}");

    w.open_block(&format!(
        "fn kind(&self) -> Option<{traits_path}::{name}Kind> {{"
    ));
    w.open_block("Some(match self {");
    for v in variants {
        let variant_name = pascal_case(v);
        w.line(&format!(
            "{name}::{variant_name} => {traits_path}::{name}Kind::{variant_name},"
        ));
    }
    w.close_block("})");
    w.close_block("}");

    w.close_block("}");
    w.newline();
}

/// Emit `impl FooLike for Foo` for a flags type (dialect crate, `ast.rs`).
fn emit_rust_flags_like_impl(
    w: &mut RustWriter,
    traits_path: &str,
    name: &str,
    flags: &[(String, u32)],
) {
    w.open_block(&format!("impl {traits_path}::{name}Like for {name} {{"));
    let mut sorted: Vec<_> = flags.iter().collect();
    sorted.sort_by_key(|(_, v)| *v);
    for (flag_name, _) in &sorted {
        let method = flag_name.to_lowercase();
        w.open_block(&format!("fn {method}(&self) -> bool {{"));
        w.line(&format!("self.{method}()"));
        w.close_block("}");
    }
    w.close_block("}");
    w.newline();
}

fn emit_rust_node_tag_type(w: &mut RustWriter, model: &AstModel<'_>) {
    let mut tag_names: Vec<(String, u32)> = Vec::new();
    let mut list_tags: Vec<String> = Vec::new();
    for item in model.node_like_items() {
        match item {
            NodeLikeRef::Node(node) => {
                tag_names.push((node.name.to_string(), model.tag_for(node.name)));
            }
            NodeLikeRef::List(list) => {
                tag_names.push((list.name.to_string(), model.tag_for(list.name)));
                list_tags.push(list.name.to_string());
            }
        }
    }

    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    w.line("#[repr(u32)]");
    w.open_block("pub enum NodeTag {");
    w.line("Null = 0,");
    for (name, tag) in &tag_names {
        w.line(&format!("{name} = {tag},"));
    }
    w.close_block("}");
    w.newline();

    w.open_block("impl NodeTag {");
    w.open_block("pub(crate) fn from_raw(raw: u32) -> Option<NodeTag> {");
    w.open_block("match raw {");
    w.line("0 => Some(NodeTag::Null),");
    for (name, tag) in &tag_names {
        w.line(&format!("{tag} => Some(NodeTag::{name}),"));
    }
    w.line("_ => None,");
    w.close_block("}");
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

fn emit_rust_node_tag_accessor(
    w: &mut RustWriter,
    node_like_items: &[NodeLikeRef<'_>],
    open_for_extension: bool,
) {
    w.doc_comment("The node's tag.");
    w.open_block("pub fn tag(&self) -> NodeTag {");
    w.open_block("match self {");
    for item in node_like_items {
        let name = item.name();
        w.line(&format!("Node::{name}(..) => NodeTag::{name},"));
    }
    if open_for_extension {
        w.line("Node::Other { .. } => NodeTag::Null,");
    } else {
        w.line("Node::__Phantom(_) => unreachable!(),");
    }
    w.close_block("}");
    w.close_block("}");
    w.newline();
}

/// Generate Rust source for token type enum.
pub(crate) fn generate_rust_tokens(tokens: &[(String, u32)], type_name: &str) -> String {
    let mut w = RustWriter::new();
    w.file_header();

    // TokenType enum
    w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]");
    w.line("#[repr(u32)]");
    w.open_block(&format!("pub enum {type_name} {{"));
    for (name, value) in tokens {
        let variant = pascal_case(name);
        w.line(&format!("{variant} = {value},"));
    }
    w.close_block("}");
    w.newline();

    // from_raw conversion
    w.open_block(&format!("impl {type_name} {{"));
    w.open_block(&format!(
        "pub fn from_raw(raw: u32) -> Option<{type_name}> {{"
    ));
    w.open_block("match raw {");
    for (name, value) in tokens {
        let variant = pascal_case(name);
        w.line(&format!("{value} => Some({type_name}::{variant}),"));
    }
    w.line("_ => None,");
    w.close_block("}");
    w.close_block("}");
    w.close_block("}");
    w.newline();

    w.open_block(&format!("impl From<{type_name}> for u32 {{"));
    w.open_block(&format!("fn from(t: {type_name}) -> u32 {{"));
    w.line("t as u32");
    w.close_block("}");
    w.close_block("}");
    w.newline();

    w.open_block(&format!(
        "impl crate::ast::GrammarTokenType for {type_name} {{"
    ));
    w.open_block("fn from_token_type(raw: u32) -> Option<Self> {");
    w.line("Self::from_raw(raw)");
    w.close_block("}");
    w.close_block("}");

    w.finish()
}

/// Module paths used by the Rust AST/FFI codegen.
pub(crate) struct RustAstPaths<'a> {
    /// Import prefix for the dialect crate, e.g. `"crate"` (internal) or
    /// `"syntaqlite"` (external).
    pub crate_prefix: &'a str,
    /// Path to FFI node structs, e.g. `"crate::ffi"`.
    pub ffi_path: &'a str,
    /// Path to `NodeId`/`SourceSpan`/`NodeList`, e.g. `"syntaqlite_parser::nodes"`.
    pub nodes_path: &'a str,
    /// Path to the zero-argument function that returns `TypedDialectEnv<'static>`,
    /// e.g. `"crate::dialect::dialect"`. Used in generated `Display` impls.
    pub grammar_fn_path: &'a str,
}

/// Generate Rust source for the FFI layer (`ffi.rs`).
///
/// Emits `#[repr(C)]` node structs, the `Bool` enum, and `ArenaNode` impls
/// that declare each struct's tag constant.
///
/// `crate_prefix` controls import paths: `"crate"` for the internal syntaqlite
/// crate, `"syntaqlite"` for external dialect crates.
impl AstModel<'_> {
    pub(crate) fn generate_rust_ffi_nodes(&self, paths: &RustAstPaths<'_>) -> String {
        let nodes_path = paths.nodes_path;
        let enum_names = self.enum_names();
        let flags_names = self.flags_names();

        let mut w = RustWriter::new();
        w.file_header();
        w.lines(&format!(
            "
        use {nodes_path}::{{ArenaNode, AnyNodeId, SourceSpan}};
    ",
        ));
        w.newline();

        // Bool enum — variants populated from C via FFI, not constructed in Rust.
        w.line("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
        w.line("#[repr(u32)]");
        w.open_block("pub(crate) enum Bool {");
        w.line("#[allow(dead_code)] // Populated from C FFI");
        w.line("False = 0,");
        w.line("True = 1,");
        w.close_block("}");
        w.newline();

        // Node structs with ArenaNode impls
        emit_rust_node_structs(
            &mut w,
            self,
            enum_names,
            flags_names,
            "pub(crate)",
            "pub(crate)",
            rust_ffi_field_type,
        );

        // ArenaNode impls — declare tag constant for each node struct
        for node in self.nodes() {
            let name = node.name;
            let tag = self.tag_for(name);
            w.line("// SAFETY: TAG matches the value the C parser writes into the `tag` field.");
            w.line(&format!(
                "unsafe impl ArenaNode for {name} {{ const TAG: u32 = {tag}; }}"
            ));
            w.newline();
        }

        w.finish()
    }

    /// Generate Rust source for the public AST layer (`ast.rs`).
    ///
    /// Emits enums, flags, `NodeTag`, view structs with ergonomic accessors,
    /// and the `Node<'a>` enum that wraps them.
    ///
    /// - `crate_prefix`: `"syntaqlite_parser"` for the internal `SQLite` dialect,
    ///   `"syntaqlite"` for external dialect crates.
    /// - `ffi_path`: module path to the dialect FFI structs (`crate::ffi` for both cases).
    #[allow(clippy::too_many_lines)]
    pub(crate) fn generate_rust_ast(
        &self,
        paths: &RustAstPaths<'_>,
        dialect_name: &str,
        open_for_extension: bool,
    ) -> String {
        let crate_prefix = paths.crate_prefix;
        let ffi_path = paths.ffi_path;
        let dialect_fn_path = paths.grammar_fn_path;
        let traits_path = format!("{crate_prefix}::ast_traits");
        let enum_names = self.enum_names();
        let flags_names = self.flags_names();
        let node_names = self.node_names();
        let list_names = self.list_names();
        let abstract_items = self.abstract_items();

        let mut w = RustWriter::new();
        w.file_header();
        if !open_for_extension {
            w.line("use std::marker::PhantomData;");
        }
        w.lines(&format!(
            "
        use {crate_prefix}::ast::{{TypedNodeId, AnyNodeId, AnyNode, GrammarNodeType, TypedList}};
        use {crate_prefix}::parser::AnyStatementResult;
        "
        ));
        w.newline();

        // NodeTag enum
        emit_rust_node_tag_type(&mut w, self);

        // Concrete value enums and flags — one per dialect, implementing the Like traits.
        for item in self.enums() {
            if item.name == "Bool" {
                continue;
            }
            emit_rust_value_enum(&mut w, item.name, item.variants);
        }
        for item in self.flags() {
            emit_rust_flags_type(&mut w, item.name, item.flags);
        }

        // Like-trait impls for the concrete types above.
        for item in self.enums() {
            if item.name == "Bool" {
                continue;
            }
            emit_rust_value_enum_like_impl(&mut w, &traits_path, item.name, item.variants);
        }
        for item in self.flags() {
            emit_rust_flags_like_impl(&mut w, &traits_path, item.name, item.flags);
        }

        // Abstract type enums (Expr, Stmt, etc.)
        for &(abs_name, members) in abstract_items {
            w.doc_comment(&format!(
                "Abstract `{abs_name}` — pattern-match to access the concrete type."
            ));
            w.line("#[derive(Debug, Clone, Copy)]");
            w.line(&format!("pub enum {abs_name}<'a> {{"));
            w.indent();
            for member in members {
                if node_names.contains(member.as_str()) || list_names.contains(member.as_str()) {
                    w.line(&format!("{member}({member}<'a>),"));
                }
            }
            w.doc_comment(&format!(
                "A node that doesn't match any known `{abs_name}` variant."
            ));
            w.line("Other(Node<'a>),");
            w.dedent();
            w.line("}");
            w.newline();

            // node_id() method
            w.open_block(&format!("impl<'a> {abs_name}<'a> {{"));
            w.doc_comment("The arena node ID of this node.");
            w.open_block("pub fn node_id(&self) -> AnyNodeId {");
            w.open_block("match self {");
            for member in members {
                if node_names.contains(member.as_str()) || list_names.contains(member.as_str()) {
                    w.line(&format!("{abs_name}::{member}(n) => n.node_id(),"));
                }
            }
            w.line(&format!("{abs_name}::Other(n) => n.node_id(),"));
            w.close_block("}");
            w.close_block("}");
            w.close_block("}");
            w.newline();

            // FromArena impl
            w.line(&format!(
                "impl<'a> GrammarNodeType<'a> for {abs_name}<'a> {{"
            ));
            w.indent();
            w.line(
                "fn from_arena(reader: AnyStatementResult<'a>, id: AnyNodeId) -> Option<Self> {",
            );
            w.indent();
            w.line("let node = Node::resolve(reader, id)?;");
            w.line("Some(match node {");
            w.indent();
            for member in members {
                if node_names.contains(member.as_str()) || list_names.contains(member.as_str()) {
                    w.line(&format!("Node::{member}(n) => {abs_name}::{member}(n),"));
                }
            }
            w.line(&format!("other => {abs_name}::Other(other),"));
            w.dedent();
            w.line("})");
            w.dedent();
            w.line("}");
            w.dedent();
            w.line("}");
            w.newline();

            // XxxId newtype for this abstract enum
            w.line("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]");
            w.line(&format!("pub struct {abs_name}Id(pub AnyNodeId);"));
            w.newline();
            w.open_block(&format!(
                "impl<'a> From<{abs_name}<'a>> for {abs_name}Id {{"
            ));
            w.line(&format!(
                "fn from(n: {abs_name}<'a>) -> Self {{ {abs_name}Id(n.node_id()) }}"
            ));
            w.close_block("}");
            w.newline();
            w.open_block(&format!("impl From<{abs_name}Id> for AnyNodeId {{"));
            w.line(&format!(
                "fn from(id: {abs_name}Id) -> AnyNodeId {{ id.0 }}"
            ));
            w.close_block("}");
            w.newline();
            w.open_block(&format!("impl TypedNodeId for {abs_name}Id {{"));
            w.line(&format!("type Node<'a> = {abs_name}<'a>;"));
            w.close_block("}");
            w.newline();
        }

        // View structs — ergonomic wrappers around FFI structs
        for node in self.nodes() {
            let name = node.name;
            let fields = node.fields;

            // Struct definition
            w.line("#[derive(Clone, Copy)]");
            w.line(&format!("pub struct {name}<'a> {{"));
            w.indent();
            w.line(&format!("raw: &'a {ffi_path}::{name},"));
            w.line("reader: AnyStatementResult<'a>,");
            w.line("id: AnyNodeId,");
            w.dedent();
            w.line("}");
            w.newline();

            // Debug impl — delegate to raw FFI struct
            w.line(&format!("impl std::fmt::Debug for {name}<'_> {{"));
            w.indent();
            w.line("fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {");
            w.indent();
            w.line("self.raw.fmt(f)");
            w.dedent();
            w.line("}");
            w.dedent();
            w.line("}");
            w.newline();

            // Display impl — dump via NodeRef to avoid exposing AnyStatementResult internals
            w.line(&format!("impl std::fmt::Display for {name}<'_> {{"));
            w.indent();
            w.line("fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {");
            w.indent();
            w.line("let mut buf = String::new();");
            w.line(&format!(
                "AnyNode {{ id: self.id, reader: self.reader }}.dump(&mut buf, 0);"
            ));
            w.line("f.write_str(&buf)");
            w.dedent();
            w.line("}");
            w.dedent();
            w.line("}");
            w.newline();

            // Accessor methods
            w.line(&format!("impl<'a> {name}<'a> {{"));
            w.indent();
            w.doc_comment("The arena node ID of this node.");
            w.line("pub fn node_id(&self) -> AnyNodeId { self.id }");
            for field in fields {
                let fname = rust_field_name(&field.name);
                let return_type =
                    rust_view_return_type(field, enum_names, flags_names, node_names, list_names);
                let body = rust_view_accessor_body(field, ffi_path);
                w.line(&format!("pub fn {fname}(&self) -> {return_type} {{"));
                w.indent();
                w.line(&body);
                w.dedent();
                w.line("}");
            }
            w.dedent();
            w.line("}");
            w.newline();

            // FromArena impl — resolve from arena by NodeId (tag-checked, no unsafe)
            w.line(&format!("impl<'a> GrammarNodeType<'a> for {name}<'a> {{"));
            w.indent();
            w.line(
                "fn from_arena(reader: AnyStatementResult<'a>, id: AnyNodeId) -> Option<Self> {",
            );
            w.indent();
            w.line(&format!(
                "let raw = reader.resolve_as::<{ffi_path}::{name}>(id)?;"
            ));
            w.line(&format!("Some({name} {{ raw, reader, id }})"));
            w.dedent();
            w.line("}");
            w.dedent();
            w.line("}");
            w.newline();

            // XxxId newtype for this view struct
            w.line("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]");
            w.line(&format!("pub struct {name}Id(pub AnyNodeId);"));
            w.newline();
            w.open_block(&format!("impl<'a> From<{name}<'a>> for {name}Id {{"));
            w.line(&format!(
                "fn from(n: {name}<'a>) -> Self {{ {name}Id(n.node_id()) }}"
            ));
            w.close_block("}");
            w.newline();
            w.open_block(&format!("impl From<{name}Id> for AnyNodeId {{"));
            w.line(&format!("fn from(id: {name}Id) -> AnyNodeId {{ id.0 }}"));
            w.close_block("}");
            w.newline();
            w.open_block(&format!("impl TypedNodeId for {name}Id {{"));
            w.line(&format!("type Node<'a> = {name}<'a>;"));
            w.close_block("}");
            w.newline();
        }

        // Typed list type aliases
        for list in self.lists() {
            let name = list.name;
            let child_type = list.child_type;
            let ct = child_type;
            let element_type = if node_names.contains(ct) || list_names.contains(ct) {
                format!("{ct}<'a>")
            } else {
                "Node<'a>".into()
            };
            w.doc_comment(&format!("Typed list of `{child_type}`."));
            w.line(&format!(
                "pub type {name}<'a> = TypedList<'a, {element_type}>;"
            ));
            w.newline();

            // XxxId newtype for this list alias
            w.line("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]");
            w.line(&format!("pub struct {name}Id(pub AnyNodeId);"));
            w.newline();
            w.open_block(&format!("impl<'a> From<{name}<'a>> for {name}Id {{"));
            w.line(&format!(
                "fn from(n: {name}<'a>) -> Self {{ {name}Id(n.node_id()) }}"
            ));
            w.close_block("}");
            w.newline();
            w.open_block(&format!("impl From<{name}Id> for AnyNodeId {{"));
            w.line(&format!("fn from(id: {name}Id) -> AnyNodeId {{ id.0 }}"));
            w.close_block("}");
            w.newline();
            w.open_block(&format!("impl TypedNodeId for {name}Id {{"));
            w.line(&format!("type Node<'a> = {name}<'a>;"));
            w.close_block("}");
            w.newline();
        }

        // Node<'a> enum — wraps view structs
        w.doc_comment("A typed AST node. Pattern-match to access the concrete type.");
        w.line("#[derive(Debug, Clone, Copy)]");
        w.line("pub enum Node<'a> {");
        w.indent();
        for item in self.node_like_items() {
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
        if open_for_extension {
            w.doc_comment("A node with an unknown tag from a dialect extension.");
            w.line("Other { id: AnyNodeId, tag: u32 },");
        } else {
            w.doc_comment("Placeholder for PhantomData lifetime — never constructed.");
            w.line("__Phantom(PhantomData<&'a ()>),");
        }
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
        w.line("pub(crate) unsafe fn from_raw(ptr: *const u32, reader: AnyStatementResult<'a>, id: AnyNodeId) -> Node<'a> {");
        w.indent();
        w.line("// SAFETY: caller guarantees ptr is valid for 'a with a valid tag.");
        w.line("unsafe {");
        w.line("let tag = NodeTag::from_raw(*ptr).unwrap_or(NodeTag::Null);");
        w.line("match tag {");
        w.indent();
        for item in self.node_like_items() {
            match item {
                NodeLikeRef::Node(node) => {
                    let name = node.name;
                    w.line(&format!("NodeTag::{name} => Node::{name}({name} {{ raw: &*(ptr as *const {ffi_path}::{name}), reader, id }}),"));
                }
                NodeLikeRef::List(list) => {
                    let name = list.name;
                    w.line(&format!("NodeTag::{name} => Node::{name}(TypedList::from_arena(reader, id).expect(\"list tag invariant\")),"));
                }
            }
        }
        if open_for_extension {
            w.line("_ => Node::Other { id, tag: *ptr },");
        } else {
            w.line("_ => unreachable!(\"unknown node tag\"),");
        }
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
        pub(crate) fn resolve(reader: AnyStatementResult<'a>, id: AnyNodeId) -> Option<Node<'a>> {
            let (ptr, _tag) = reader.node_ptr(id)?;
            Some(unsafe { Node::from_raw(ptr as *const u32, reader, id) })
        }
    ",
        );
        w.newline();

        // tag()
        emit_rust_node_tag_accessor(&mut w, self.node_like_items(), open_for_extension);

        // node_id() on Node<'a>
        w.doc_comment("The arena node ID of this node.");
        w.open_block("pub fn node_id(&self) -> AnyNodeId {");
        w.open_block("match self {");
        for item in self.node_like_items() {
            match item {
                NodeLikeRef::Node(node) => {
                    w.line(&format!("Node::{}(n) => n.node_id(),", node.name));
                }
                NodeLikeRef::List(list) => {
                    w.line(&format!("Node::{}(n) => n.node_id(),", list.name));
                }
            }
        }
        if open_for_extension {
            w.line("Node::Other { id, .. } => *id,");
        } else {
            w.line("Node::__Phantom(_) => unreachable!(),");
        }
        w.close_block("}");
        w.close_block("}");
        w.newline();

        w.dedent();
        w.line("}");
        w.newline();

        // FromArena impl for Node
        w.lines(
            "
        impl<'a> GrammarNodeType<'a> for Node<'a> {
            fn from_arena(reader: AnyStatementResult<'a>, id: AnyNodeId) -> Option<Self> {
                Node::resolve(reader, id)
            }
        }
    ",
        );
        w.newline();

        // NodeId — typed ID for Node<'a>
        w.line("#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]");
        w.line("pub struct NodeId(pub AnyNodeId);");
        w.newline();
        w.open_block("impl<'a> From<Node<'a>> for NodeId {");
        w.line("fn from(n: Node<'a>) -> Self { NodeId(n.node_id()) }");
        w.close_block("}");
        w.newline();
        w.open_block("impl From<NodeId> for AnyNodeId {");
        w.line("fn from(id: NodeId) -> AnyNodeId { id.0 }");
        w.close_block("}");
        w.newline();
        w.open_block("impl TypedNodeId for NodeId {");
        w.line("type Node<'a> = Node<'a>;");
        w.close_block("}");
        w.newline();

        // ── Trait impls (connecting concrete types to the generic trait layer) ──

        let marker = format!("{}Ast", pascal_case(dialect_name));

        // Marker type
        w.doc_comment(&format!(
            "Marker type for the {dialect_name} dialect's AST. Implements `AstTypes`."
        ));
        w.line(&format!("pub enum {marker} {{}}"));
        w.newline();

        // impl AstTypes for marker
        w.open_block(&format!(
            "impl<'a> {traits_path}::AstTypes<'a> for {marker} {{"
        ));
        w.line("type Node = Node<'a>;");
        for &(abs_name, _) in abstract_items {
            w.line(&format!("type {abs_name} = {abs_name}<'a>;"));
        }
        for node in self.nodes() {
            w.line(&format!("type {n} = {n}<'a>;", n = node.name));
        }
        for item in self.enums() {
            if item.name == "Bool" {
                continue;
            }
            w.line(&format!("type {n} = {n};", n = item.name));
        }
        for item in self.flags() {
            w.line(&format!("type {n} = {n};", n = item.name));
        }
        w.close_block("}");
        w.newline();

        // impl NodeLike for Node
        w.open_block(&format!(
            "impl<'a> {traits_path}::NodeLike<'a> for Node<'a> {{"
        ));
        w.line(&format!("type Ast = {marker};"));
        w.open_block("fn node_id(&self) -> AnyNodeId {");
        w.open_block("match self {");
        for item in self.node_like_items() {
            match item {
                NodeLikeRef::Node(node) => {
                    w.line(&format!("Node::{}(n) => n.node_id(),", node.name));
                }
                NodeLikeRef::List(list) => {
                    w.line(&format!("Node::{}(n) => n.node_id(),", list.name));
                }
            }
        }
        if open_for_extension {
            w.line("Node::Other { id, .. } => *id,");
        } else {
            w.line("Node::__Phantom(_) => unreachable!(),");
        }
        w.close_block("}");
        w.close_block("}");
        w.close_block("}");
        w.newline();

        // impl XxxLike for each abstract enum
        for &(abs_name, members) in abstract_items {
            w.open_block(&format!(
                "impl<'a> {traits_path}::{abs_name}Like<'a> for {abs_name}<'a> {{"
            ));
            w.line(&format!("type Ast = {marker};"));
            w.open_block(&format!(
                "fn kind(&self) -> {traits_path}::{abs_name}Kind<'a, {marker}> {{"
            ));
            w.open_block("match *self {");
            for member in members {
                if node_names.contains(member.as_str()) || list_names.contains(member.as_str()) {
                    w.line(&format!(
                        "{abs_name}::{member}(n) => {traits_path}::{abs_name}Kind::{member}(n),"
                    ));
                }
            }
            w.line(&format!(
                "{abs_name}::Other(n) => {traits_path}::{abs_name}Kind::Other(n),"
            ));
            w.close_block("}");
            w.close_block("}");
            w.close_block("}");
            w.newline();
        }

        // impl XxxView for each node view struct
        for node in self.nodes() {
            let name = node.name;
            w.open_block(&format!(
                "impl<'a> {traits_path}::{name}View<'a> for {name}<'a> {{"
            ));
            w.line(&format!("type Ast = {marker};"));
            w.line("fn node_id(&self) -> AnyNodeId { self.id }");
            for field in node.fields {
                let fname = rust_field_name(&field.name);
                let return_type =
                    rust_view_return_type(field, enum_names, flags_names, node_names, list_names);
                w.open_block(&format!("fn {fname}(&self) -> {return_type} {{"));
                w.line(&format!("self.{fname}()"));
                w.close_block("}");
            }
            w.close_block("}");
            w.newline();
        }

        w.finish()
    }
}

// ── Generic trait layer codegen ─────────────────────────────────────────

/// Resolve a list type to its generic form for use in kind-enum variants.
///
/// Uses `A::X` syntax (not `<Self::Ast as AstTypes<'a>>::X`) since kind enums
/// are parameterized on `A: AstTypes<'a>`.
fn resolve_kind_enum_list_type(
    list_name: &str,
    node_names: &HashSet<&str>,
    list_names: &HashSet<&str>,
    lists: &[super::ListRef<'_>],
) -> String {
    let list = lists
        .iter()
        .find(|l| l.name == list_name)
        .expect("list not found in model");
    let child = list.child_type;
    if node_names.contains(child) {
        format!("TypedList<'a, A::{child}>")
    } else if list_names.contains(child) {
        let inner = resolve_kind_enum_list_type(child, node_names, list_names, lists);
        format!("TypedList<'a, {inner}>")
    } else {
        // Abstract or unknown child → Node
        "TypedList<'a, A::Node>".to_string()
    }
}

/// Resolve a list's child type to its generic form for use in trait signatures.
///
/// - Concrete node child → `<Self::Ast as AstTypes<'a>>::ChildName`
/// - List child (list-of-lists) → `TypedList<'a, resolve(inner)>`
/// - Abstract child → `<Self::Ast as AstTypes<'a>>::Node`
fn resolve_generic_element_type(
    child_type: &str,
    node_names: &HashSet<&str>,
    list_names: &HashSet<&str>,
    lists: &[super::ListRef<'_>],
) -> String {
    if node_names.contains(child_type) {
        format!("<Self::Ast as AstTypes<'a>>::{child_type}")
    } else if list_names.contains(child_type) {
        let list = lists
            .iter()
            .find(|l| l.name == child_type)
            .expect("list not found in model");
        let inner = resolve_generic_element_type(list.child_type, node_names, list_names, lists);
        format!("TypedList<'a, {inner}>")
    } else {
        // Abstract child type → use Node
        "<Self::Ast as AstTypes<'a>>::Node".to_string()
    }
}

/// Map a `.synq` field to its generic return type for use in trait accessor methods.
fn trait_field_return_type(
    field: &Field,
    enum_names: &HashSet<&str>,
    flags_names: &HashSet<&str>,
    node_names: &HashSet<&str>,
    list_names: &HashSet<&str>,
    lists: &[super::ListRef<'_>],
    abstract_names: &HashSet<&str>,
) -> String {
    match field.storage {
        Storage::Index => {
            let t = field.type_name.as_str();
            if list_names.contains(t) {
                let list = lists
                    .iter()
                    .find(|l| l.name == t)
                    .expect("list not found in model");
                let element =
                    resolve_generic_element_type(list.child_type, node_names, list_names, lists);
                format!("Option<TypedList<'a, {element}>>")
            } else if node_names.contains(t) || abstract_names.contains(t) {
                format!("Option<<Self::Ast as AstTypes<'a>>::{t}>")
            } else {
                // Unknown index type — shouldn't happen, default to Node
                "Option<<Self::Ast as AstTypes<'a>>::Node>".to_string()
            }
        }
        Storage::Inline => {
            let t = &field.type_name;
            if t == "Bool" {
                "bool".into()
            } else if t == "SyntaqliteSourceSpan" {
                "&'a str".into()
            } else if enum_names.contains(t.as_str()) || flags_names.contains(t.as_str()) {
                // Enum or flags — use dialect associated type for extensibility
                format!("<Self::Ast as AstTypes<'a>>::{t}")
            } else {
                t.clone()
            }
        }
    }
}

/// Generate Rust source for the shared AST trait definitions (`ast_traits.rs`).
///
/// Emits shared value enums, flags, per-node accessor traits, variant kind enums
/// for abstracts, abstract access traits, `NodeLike`, and the `AstTypes` supertrait.
///
/// This module is always compiled (no feature gate) and lives in the syntaqlite crate.
/// `TypedDialectEnv` crates import traits from `syntaqlite::ast_traits`.
impl AstModel<'_> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn generate_ast_traits(&self) -> String {
        let enum_names = self.enum_names();
        let flags_names = self.flags_names();
        let node_names = self.node_names();
        let list_names = self.list_names();
        let abstract_items = self.abstract_items();
        let abstract_names: HashSet<&str> = abstract_items.iter().map(|(name, _)| *name).collect();

        let mut w = RustWriter::new();
        w.file_header();

        w.lines(
            "
        #![allow(clippy::type_complexity)]

        use crate::ast::{GrammarNodeType, AnyNodeId, TypedList};
    ",
        );
        w.newline();

        // ── Like-traits for value enums and flags ──
        // Concrete types live in each dialect's ast.rs; these traits let generic
        // code (and future dialects) work with their own extended types.
        for item in self.enums() {
            if item.name == "Bool" {
                continue;
            }
            emit_rust_value_enum_like_trait(&mut w, item.name, item.variants);
        }
        for item in self.flags() {
            emit_rust_flags_like_trait(&mut w, item.name, item.flags);
        }

        // ── NodeLike trait ──
        w.doc_comment("Trait for the generic `Node` enum wrapper.");
        w.open_block("pub trait NodeLike<'a>: Copy {");
        w.line("type Ast: AstTypes<'a>;");
        w.line("fn node_id(&self) -> AnyNodeId;");
        w.close_block("}");
        w.newline();

        // ── Per-node accessor traits ──
        for node in self.nodes() {
            let name = node.name;
            // Field names like `from_clause` and `into_expr` map to SQL syntax, not
            // Rust's From/Into conventions.  Suppress the clippy lint on affected traits.
            let needs_convention_allow = node
                .fields
                .iter()
                .any(|f| f.name.starts_with("from_") || f.name.starts_with("into_"));
            w.doc_comment(&format!("Accessor trait for `{name}` nodes."));
            if needs_convention_allow {
                w.line("#[allow(clippy::wrong_self_convention)]");
            }
            w.open_block(&format!("pub trait {name}View<'a>: Copy {{"));
            w.line("type Ast: AstTypes<'a>;");
            w.line("fn node_id(&self) -> AnyNodeId;");
            for field in node.fields {
                let fname = rust_field_name(&field.name);
                let ret = trait_field_return_type(
                    field,
                    enum_names,
                    flags_names,
                    node_names,
                    list_names,
                    self.lists(),
                    &abstract_names,
                );
                w.line(&format!("fn {fname}(&self) -> {ret};"));
            }
            w.close_block("}");
            w.newline();
        }

        // ── Variant enums for abstracts ──
        for &(abs_name, members) in abstract_items {
            w.doc_comment(&format!("Pattern-matching variants for `{abs_name}`."));
            w.line("#[derive(Clone, Copy)]");
            w.open_block(&format!("pub enum {abs_name}Kind<'a, A: AstTypes<'a>> {{"));
            for member in members {
                if node_names.contains(member.as_str()) {
                    w.line(&format!("{member}(A::{member}),"));
                } else if list_names.contains(member.as_str()) {
                    // Lists are TypedList aliases, not associated types on AstTypes.
                    let list_type =
                        resolve_kind_enum_list_type(member, node_names, list_names, self.lists());
                    w.line(&format!("{member}({list_type}),"));
                }
            }
            w.line("Other(A::Node),");
            w.close_block("}");
            w.newline();
        }

        // ── Per-abstract access traits ──
        for &(abs_name, _) in abstract_items {
            w.doc_comment(&format!("Abstract access trait for `{abs_name}`."));
            w.open_block(&format!("pub trait {abs_name}Like<'a>: Copy {{"));
            w.line("type Ast: AstTypes<'a>;");
            w.line(&format!("fn kind(&self) -> {abs_name}Kind<'a, Self::Ast>;"));
            w.close_block("}");
            w.newline();
        }

        // ── AstTypes supertrait ──
        w.doc_comment("Bundle trait associating all AST types for a dialect.");
        w.open_block("pub trait AstTypes<'a>: 'a {");
        w.line("type Node: NodeLike<'a, Ast = Self> + Copy + GrammarNodeType<'a>;");
        for &(abs_name, _) in abstract_items {
            w.line(&format!(
                "type {abs_name}: {abs_name}Like<'a, Ast = Self> + Copy + GrammarNodeType<'a>;"
            ));
        }
        for node in self.nodes() {
            let name = node.name;
            w.line(&format!(
                "type {name}: {name}View<'a, Ast = Self> + Copy + GrammarNodeType<'a>;"
            ));
        }
        // Enum and flags associated types — dialects provide their own concrete types.
        for item in self.enums() {
            if item.name == "Bool" {
                continue;
            }
            let name = item.name;
            w.line(&format!("type {name}: {name}Like;"));
        }
        for item in self.flags() {
            let name = item.name;
            w.line(&format!("type {name}: {name}Like;"));
        }
        w.close_block("}");
        w.newline();

        w.finish()
    }
}
