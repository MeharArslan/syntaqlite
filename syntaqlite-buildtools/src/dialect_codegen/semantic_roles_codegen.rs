// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Codegen for the `SemanticRole` static table.
//!
//! `generate_rust_semantic_roles` emits a `&[SemanticRole]` static indexed by
//! node tag, one entry per node-like item (node or list) in the dialect's AST
//! model. Nodes without a `semantic { ... }` annotation emit `Transparent`.

use super::AstModel;
use crate::util::rust_writer::RustWriter;

use crate::util::synq_parser::{Field, SemanticRole as SynqRole};

/// Return the 0-based index of `field_name` in the node's field list.
fn field_index(fields: &[Field], field_name: &str) -> u8 {
    fields
        .iter()
        .position(|f| f.name == field_name)
        .unwrap_or_else(|| panic!("field '{field_name}' not found in field list")) as u8
}

/// Emit a single `SemanticRole` variant expression for an annotated node.
fn emit_role(fields: &[Field], role: &SynqRole) -> String {
    let fi = |name: &str| field_index(fields, name);
    let opt = |name: &Option<String>| -> String {
        match name {
            Some(f) => format!("Some({})", fi(f)),
            None => "None".into(),
        }
    };
    match role {
        // ── Catalog roles ────────────────────────────────────────────────
        SynqRole::DefineTable {
            name,
            columns,
            select,
        } => format!(
            "SemanticRole::DefineTable {{ name: {}, columns: {}, select: {} }}",
            fi(name),
            opt(columns),
            opt(select)
        ),
        SynqRole::DefineView {
            name,
            columns,
            select,
        } => format!(
            "SemanticRole::DefineView {{ name: {}, columns: {}, select: {} }}",
            fi(name),
            opt(columns),
            fi(select)
        ),
        SynqRole::DefineFunction { name, args } => format!(
            "SemanticRole::DefineFunction {{ name: {}, args: {} }}",
            fi(name),
            opt(args)
        ),
        SynqRole::Import { module } => format!("SemanticRole::Import {{ module: {} }}", fi(module)),
        // ── Column-list items ─────────────────────────────────────────────
        SynqRole::ColumnDef {
            name,
            type_name,
            constraints,
        } => format!(
            "SemanticRole::ColumnDef {{ name: {}, type_: {}, constraints: {} }}",
            fi(name),
            opt(type_name),
            opt(constraints)
        ),
        // ── Result columns ────────────────────────────────────────────────
        SynqRole::ResultColumn { flags, alias, expr } => format!(
            "SemanticRole::ResultColumn {{ flags: {}, alias: {}, expr: {} }}",
            fi(flags),
            fi(alias),
            fi(expr)
        ),
        // ── Expressions ───────────────────────────────────────────────────
        SynqRole::Call { name, args } => format!(
            "SemanticRole::Call {{ name: {}, args: {} }}",
            fi(name),
            fi(args)
        ),
        SynqRole::ColumnRef { column, table } => format!(
            "SemanticRole::ColumnRef {{ column: {}, table: {} }}",
            fi(column),
            fi(table)
        ),
        // ── Sources ───────────────────────────────────────────────────────
        SynqRole::SourceRef { kind, name, alias } => {
            let kind_variant = match kind.as_str() {
                "table" => "RelationKind::Table",
                "view" => "RelationKind::View",
                "interval" => "RelationKind::Interval",
                "tree" => "RelationKind::Tree",
                "graph" => "RelationKind::Graph",
                other => panic!("unknown RelationKind literal '{other}' in source_ref"),
            };
            format!(
                "SemanticRole::SourceRef {{ kind: {kind_variant}, name: {}, alias: {} }}",
                fi(name),
                fi(alias)
            )
        }
        SynqRole::ScopedSource { body, alias } => format!(
            "SemanticRole::ScopedSource {{ body: {}, alias: {} }}",
            fi(body),
            fi(alias)
        ),
        // ── Scope structure ───────────────────────────────────────────────
        SynqRole::Query {
            from,
            columns,
            where_clause,
            groupby,
            having,
            orderby,
            limit_clause,
        } => format!(
            "SemanticRole::Query {{ from: {}, columns: {}, where_clause: {}, groupby: {}, having: {}, orderby: {}, limit_clause: {} }}",
            fi(from),
            fi(columns),
            fi(where_clause),
            fi(groupby),
            fi(having),
            fi(orderby),
            fi(limit_clause)
        ),
        SynqRole::CteBinding { name, body } => format!(
            "SemanticRole::CteBinding {{ name: {}, body: {} }}",
            fi(name),
            fi(body)
        ),
        SynqRole::CteScope {
            recursive,
            bindings,
            body,
        } => format!(
            "SemanticRole::CteScope {{ recursive: {}, bindings: {}, body: {} }}",
            fi(recursive),
            fi(bindings),
            fi(body)
        ),
        SynqRole::TriggerScope { target, when, body } => format!(
            "SemanticRole::TriggerScope {{ target: {}, when: {}, body: {} }}",
            fi(target),
            fi(when),
            fi(body)
        ),
    }
}

/// Generate a Rust source file containing the `{PREFIX}_SEMANTIC_ROLES` static.
///
/// `prefix` should be uppercase (e.g. `"SQLITE"`), matching the naming
/// convention used by the other formatter statics.
pub(crate) fn generate_rust_semantic_roles(model: &AstModel, prefix: &str) -> String {
    let mut w = RustWriter::new();
    w.file_header();

    w.lines(&format!(
        "use crate::dialect::schema::{{RelationKind, SemanticRole}};\n\
         \n\
         /// Semantic role table for the `{prefix}` dialect, indexed by node tag.\n\
         /// Tags are 1-based; index 0 is an unused sentinel.\n\
         pub(crate) static {prefix}_SEMANTIC_ROLES: &[SemanticRole] = &["
    ));

    // Index 0 is unused — node tags start at 1.
    w.lines("    SemanticRole::Transparent, // (index 0 — unused sentinel)");

    for node_like in model.node_like_items() {
        use super::NodeLikeRef;
        let (item_name, semantic) = match node_like {
            NodeLikeRef::Node(n) => (n.name, n.semantic),
            NodeLikeRef::List(l) => (l.name, None),
        };

        let role_expr = match semantic.map(|a| &a.role) {
            Some(role) => {
                // Get the fields for this node (lists have no semantic annotation).
                let fields = match node_like {
                    NodeLikeRef::Node(n) => n.fields,
                    NodeLikeRef::List(_) => unreachable!("lists never have semantic annotations"),
                };
                emit_role(fields, role)
            }
            None => "SemanticRole::Transparent".into(),
        };

        w.lines(&format!("    {role_expr}, // {item_name}"));
    }

    w.lines("];");
    w.finish()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dialect_codegen::AstModel;
    use crate::util::synq_parser::{Item, parse_synq_file};

    fn model_from(synq: &str) -> Vec<Item> {
        parse_synq_file(synq).expect("parse failed")
    }

    #[test]
    fn transparent_for_node_without_annotation() {
        let items = model_from("node Foo { x: inline SyntaqliteSourceSpan }");
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "TEST");
        assert!(out.contains("SemanticRole::Transparent"), "got:\n{out}");
        assert!(
            out.contains("// Foo"),
            "expected node name comment, got:\n{out}"
        );
    }

    #[test]
    fn define_table_with_correct_field_indices() {
        let items = model_from(
            r"node CreateTableStmt {
                table_name: inline SyntaqliteSourceSpan
                schema: inline SyntaqliteSourceSpan
                columns: index ColumnDefList
                as_select: index Select
                semantic { define_table(name: table_name, columns: columns, select: as_select) }
            }",
        );
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "TEST");
        // table_name = field 0, columns = field 2, as_select = field 3
        assert!(
            out.contains(
                "SemanticRole::DefineTable { name: 0, columns: Some(2), select: Some(3) }"
            ),
            "got:\n{out}"
        );
        assert!(
            out.contains("// CreateTableStmt"),
            "expected node name comment, got:\n{out}"
        );
    }

    #[test]
    fn define_table_optional_fields_are_none_when_absent() {
        let items = model_from(
            r"node CreateTableStmt {
                table_name: inline SyntaqliteSourceSpan
                semantic { define_table(name: table_name) }
            }",
        );
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "TEST");
        assert!(
            out.contains("SemanticRole::DefineTable { name: 0, columns: None, select: None }"),
            "got:\n{out}"
        );
    }

    #[test]
    fn define_view_with_correct_field_indices() {
        let items = model_from(
            r"node CreateViewStmt {
                view_name: inline SyntaqliteSourceSpan
                schema: inline SyntaqliteSourceSpan
                select: index Select
                semantic { define_view(name: view_name, select: select) }
            }",
        );
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "TEST");
        // view_name = 0, select = 2, no columns
        assert!(
            out.contains("SemanticRole::DefineView { name: 0, columns: None, select: 2 }"),
            "got:\n{out}"
        );
    }

    #[test]
    fn define_view_with_explicit_columns() {
        let items = model_from(
            r"node CreatePerfettoViewStmt {
                view_name: inline SyntaqliteSourceSpan
                schema: index PerfettoArgDefList
                select: index Select
                semantic { define_view(name: view_name, columns: schema, select: select) }
            }",
        );
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "TEST");
        // view_name = 0, schema = 1, select = 2
        assert!(
            out.contains("SemanticRole::DefineView { name: 0, columns: Some(1), select: 2 }"),
            "got:\n{out}"
        );
    }

    #[test]
    fn define_function_with_optional_args() {
        let items = model_from(
            r"node CreateFunctionStmt {
                func_name: inline SyntaqliteSourceSpan
                args: index ArgList
                semantic { define_function(name: func_name, args: args) }
            }",
        );
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "TEST");
        assert!(
            out.contains("SemanticRole::DefineFunction { name: 0, args: Some(1) }"),
            "got:\n{out}"
        );
    }

    #[test]
    fn import_with_correct_field_index() {
        let items = model_from(
            r"node IncludeModuleStmt {
                module_name: inline SyntaqliteSourceSpan
                semantic { import(module: module_name) }
            }",
        );
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "TEST");
        assert!(
            out.contains("SemanticRole::Import { module: 0 }"),
            "got:\n{out}"
        );
    }

    #[test]
    fn list_always_emits_transparent() {
        let items = model_from(
            r"node Foo { x: inline SyntaqliteSourceSpan }
               list FooList { Foo }",
        );
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "TEST");
        // Three entries: sentinel at index 0, Foo (Transparent), FooList (Transparent)
        let count = out.matches("SemanticRole::Transparent").count();
        assert_eq!(
            count, 3,
            "expected 3 Transparent entries (1 sentinel + 2 nodes), got:\n{out}"
        );
    }

    #[test]
    fn static_name_uses_prefix() {
        let items = model_from("node Foo { x: inline SyntaqliteSourceSpan }");
        let model = AstModel::new(&items);
        let out = generate_rust_semantic_roles(&model, "SQLITE");
        assert!(
            out.contains("SQLITE_SEMANTIC_ROLES: &[SemanticRole]"),
            "got:\n{out}"
        );
    }
}
