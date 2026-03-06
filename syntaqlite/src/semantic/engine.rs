// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Data-driven semantic engine: interprets `SemanticRole` annotations to
//! validate expressions, resolve names, and enforce scope structure.
//!
//! The engine operates in two passes over a statement list:
//!
//! 1. **Accumulation pass** — reads catalog roles (`DefineTable`, `DefineView`,
//!    `DefineFunction`, `Import`) and populates the document layer of the
//!    catalog. Handled by [`Catalog::accumulate_ddl`].
//!
//! 2. **Validation pass** — visits every node in the statement, dispatching on
//!    its [`SemanticRole`] to validate function calls, column references, and
//!    scope structure.

use syntaqlite_syntax::any::{AnyNodeId, AnyParsedStatement, FieldValue, NodeFields};

use crate::dialect::Dialect;
use crate::dialect::schema::SemanticRole;

use super::ValidationConfig;
use super::catalog::{Catalog, ColumnResolution, FunctionCheckResult};
use super::diagnostics::{Diagnostic, DiagnosticMessage, Help};
use super::fuzzy::best_suggestion;

/// Data-driven semantic engine for validating a single parsed statement.
///
/// Reads the dialect's [`SemanticRole`] table and dispatches node visits to
/// role-specific handlers.
pub(crate) struct SemanticEngine<'a> {
    stmt: AnyParsedStatement<'a>,
    roles: &'static [SemanticRole],
    catalog: &'a mut Catalog,
    config: &'a ValidationConfig,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> SemanticEngine<'a> {
    /// Run the validation pass over `root` and return all diagnostics.
    ///
    /// The accumulation pass (catalog population via [`Catalog::accumulate_ddl`])
    /// must have already run for this statement before calling this method.
    pub(crate) fn run(
        stmt: AnyParsedStatement<'a>,
        root: AnyNodeId,
        dialect: Dialect,
        catalog: &'a mut Catalog,
        config: &'a ValidationConfig,
    ) -> Vec<Diagnostic> {
        let roles = dialect.roles();
        let mut engine = SemanticEngine {
            stmt,
            roles,
            catalog,
            config,
            diagnostics: Vec::new(),
        };
        engine.visit(root);
        engine.diagnostics
    }

    // ── Core visitor ─────────────────────────────────────────────────────────

    fn visit(&mut self, node_id: AnyNodeId) {
        if node_id.is_null() {
            return;
        }
        // List nodes have no role — visit their elements directly.
        if let Some(children) = self.stmt.list_children(node_id) {
            let children: Vec<AnyNodeId> = children.to_vec();
            for child in children {
                if !child.is_null() {
                    self.visit(child);
                }
            }
            return;
        }
        let Some((tag, fields)) = self.stmt.extract_fields(node_id) else {
            return;
        };
        let idx = u32::from(tag) as usize;
        let role = self
            .roles
            .get(idx)
            .copied()
            .unwrap_or(SemanticRole::Transparent);

        match role {
            // Catalog roles are handled in the accumulation pass; skip here.
            SemanticRole::DefineTable { .. }
            | SemanticRole::DefineView { .. }
            | SemanticRole::DefineFunction { .. }
            | SemanticRole::Import { .. } => {}

            // Transparent: recurse into children without special handling.
            // ColumnDef and ResultColumn have no validation logic yet — child
            // expressions are reached via transparent traversal.
            SemanticRole::Transparent
            | SemanticRole::ColumnDef { .. }
            | SemanticRole::ResultColumn { .. }
            | SemanticRole::CteBinding { .. } => self.visit_children(node_id),

            SemanticRole::Call { name, args } => self.visit_call(node_id, &fields, name, args),
            SemanticRole::ColumnRef { column, table } => {
                self.visit_column_ref(&fields, column, table);
            }
            SemanticRole::SourceRef { name, alias, .. } => {
                self.visit_source_ref(&fields, name, alias);
            }
            SemanticRole::ScopedSource { body, alias } => {
                self.visit_scoped_source(&fields, body, alias);
            }
            SemanticRole::Query {
                from,
                columns,
                where_clause,
                groupby,
                having,
                orderby,
                limit_clause,
            } => self.visit_query(
                &fields,
                from,
                columns,
                where_clause,
                groupby,
                having,
                orderby,
                limit_clause,
            ),
            SemanticRole::CteScope {
                recursive,
                bindings,
                body,
            } => self.visit_cte_scope(&fields, recursive, bindings, body),
            SemanticRole::TriggerScope { target: _, when, body } => {
                self.visit_trigger_scope(&fields, when, body);
            }
        }
    }

    fn visit_children(&mut self, node_id: AnyNodeId) {
        let children: Vec<AnyNodeId> = self.stmt.child_node_ids(node_id).collect();
        for child in children {
            if !child.is_null() {
                self.visit(child);
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn span_offset(&self, s: &str) -> usize {
        s.as_ptr() as usize - self.stmt.source().as_ptr() as usize
    }

    fn field_node_id(fields: &NodeFields<'_>, idx: u8) -> Option<AnyNodeId> {
        match fields[idx as usize] {
            FieldValue::NodeId(id) if !id.is_null() => Some(id),
            _ => None,
        }
    }

    fn visit_opt(&mut self, id: Option<AnyNodeId>) {
        if let Some(id) = id {
            self.visit(id);
        }
    }

    /// Extract source text from a `Name` node (`IdentName` or `Error`).
    /// Both node kinds store their span at field 0.
    fn name_text(&self, node_id: Option<AnyNodeId>) -> &'a str {
        let Some(node_id) = node_id else {
            return "";
        };
        let Some((_, fields)) = self.stmt.extract_fields(node_id) else {
            return "";
        };
        if fields.is_empty() {
            return "";
        }
        match fields[0] {
            FieldValue::Span(s) => s,
            _ => "",
        }
    }

    // ── Role handlers ─────────────────────────────────────────────────────────

    /// Validate a table reference and register it in the current query scope.
    fn visit_source_ref(&mut self, fields: &NodeFields<'a>, name_idx: u8, alias_idx: u8) {
        let name = match fields[name_idx as usize] {
            FieldValue::Span(s) => s,
            _ => return,
        };
        if name.is_empty() {
            return;
        }
        let offset = self.span_offset(name);

        let is_known = self.catalog.resolve_relation(name)
            || self.catalog.resolve_table_function(name);
        if !is_known {
            let mut candidates = self.catalog.all_relation_names();
            candidates.extend(self.catalog.all_table_function_names());
            let suggestion =
                best_suggestion(name, &candidates, self.config.suggestion_threshold);
            self.diagnostics.push(Diagnostic {
                start_offset: offset,
                end_offset: offset + name.len(),
                message: DiagnosticMessage::UnknownTable {
                    name: name.to_string(),
                },
                severity: self.config.severity(),
                help: suggestion.map(Help::Suggestion),
            });
        }

        let alias = self.name_text(Self::field_node_id(fields, alias_idx));
        let scope_name = if alias.is_empty() { name } else { alias };
        let columns = self.catalog.columns_for_table_source(name);
        self.catalog.add_query_table(scope_name, columns);
    }

    /// Validate a function call and recurse into its arguments.
    fn visit_call(
        &mut self,
        node_id: AnyNodeId,
        fields: &NodeFields<'a>,
        name_idx: u8,
        args_idx: u8,
    ) {
        if let FieldValue::Span(name) = fields[name_idx as usize] {
            if !name.is_empty() {
                let offset = self.span_offset(name);
                let args_id = Self::field_node_id(fields, args_idx);
                let arg_count = args_id
                    .and_then(|id| self.stmt.list_children(id))
                    .map_or(0, |c| c.len());
                match self.catalog.check_function(name, arg_count) {
                    FunctionCheckResult::Ok => {}
                    FunctionCheckResult::Unknown => {
                        let candidates = self.catalog.all_function_names();
                        let suggestion = best_suggestion(
                            name,
                            &candidates,
                            self.config.suggestion_threshold,
                        );
                        self.diagnostics.push(Diagnostic {
                            start_offset: offset,
                            end_offset: offset + name.len(),
                            message: DiagnosticMessage::UnknownFunction {
                                name: name.to_string(),
                            },
                            severity: self.config.severity(),
                            help: suggestion.map(Help::Suggestion),
                        });
                    }
                    FunctionCheckResult::WrongArity { expected } => {
                        self.diagnostics.push(Diagnostic {
                            start_offset: offset,
                            end_offset: offset + name.len(),
                            message: DiagnosticMessage::FunctionArity {
                                name: name.to_string(),
                                expected,
                                got: arg_count,
                            },
                            severity: self.config.severity(),
                            help: None,
                        });
                    }
                }
            }
        }
        // Recurse into args, filter_clause, over_clause, etc.
        self.visit_children(node_id);
    }

    /// Validate a column reference against the current scope.
    fn visit_column_ref(&mut self, fields: &NodeFields<'a>, column_idx: u8, table_idx: u8) {
        let column = match fields[column_idx as usize] {
            FieldValue::Span(s) => s,
            _ => return,
        };
        if column.is_empty() {
            return;
        }
        let table = match fields[table_idx as usize] {
            FieldValue::Span(s) if !s.is_empty() => Some(s),
            _ => None,
        };
        let offset = self.span_offset(column);

        match self.catalog.resolve_column(table, column) {
            ColumnResolution::Found | ColumnResolution::TableNotFound => {}
            ColumnResolution::TableFoundColumnMissing => {
                let tbl = table.expect("qualifier present when TableFoundColumnMissing");
                let candidates = self.catalog.all_column_names(Some(tbl));
                let suggestion =
                    best_suggestion(column, &candidates, self.config.suggestion_threshold);
                self.diagnostics.push(Diagnostic {
                    start_offset: offset,
                    end_offset: offset + column.len(),
                    message: DiagnosticMessage::UnknownColumn {
                        column: column.to_string(),
                        table: Some(tbl.to_string()),
                    },
                    severity: self.config.severity(),
                    help: suggestion.map(Help::Suggestion),
                });
            }
            ColumnResolution::NotFound => {
                let candidates = self.catalog.all_column_names(None);
                let suggestion =
                    best_suggestion(column, &candidates, self.config.suggestion_threshold);
                self.diagnostics.push(Diagnostic {
                    start_offset: offset,
                    end_offset: offset + column.len(),
                    message: DiagnosticMessage::UnknownColumn {
                        column: column.to_string(),
                        table: None,
                    },
                    severity: self.config.severity(),
                    help: suggestion.map(Help::Suggestion),
                });
            }
        }
    }

    /// Visit a subquery source in a fresh scope, then register its alias.
    fn visit_scoped_source(&mut self, fields: &NodeFields<'a>, body_idx: u8, alias_idx: u8) {
        self.catalog.push_query_scope();
        self.visit_opt(Self::field_node_id(fields, body_idx));
        self.catalog.pop_query_scope();

        let alias = self.name_text(Self::field_node_id(fields, alias_idx));
        if !alias.is_empty() {
            self.catalog.add_query_table(alias, None);
        }
    }

    /// Visit a SELECT: FROM clause first (to build scope), then expressions.
    #[allow(clippy::too_many_arguments)]
    fn visit_query(
        &mut self,
        fields: &NodeFields<'a>,
        from: u8,
        columns: u8,
        where_clause: u8,
        groupby: u8,
        having: u8,
        orderby: u8,
        limit_clause: u8,
    ) {
        // FROM first so table bindings are in scope for expression checks.
        self.visit_opt(Self::field_node_id(fields, from));
        for idx in [columns, where_clause, groupby, having, orderby, limit_clause] {
            self.visit_opt(Self::field_node_id(fields, idx));
        }
    }

    /// Visit a WITH clause, processing each CTE binding in order.
    fn visit_cte_scope(
        &mut self,
        fields: &NodeFields<'a>,
        recursive_idx: u8,
        bindings_idx: u8,
        body_idx: u8,
    ) {
        let is_recursive = matches!(fields[recursive_idx as usize], FieldValue::Bool(true));
        let cte_ids: Vec<AnyNodeId> = Self::field_node_id(fields, bindings_idx)
            .and_then(|id| self.stmt.list_children(id))
            .map(|s| s.to_vec())
            .unwrap_or_default();

        for cte_id in cte_ids {
            if cte_id.is_null() {
                continue;
            }
            let Some((cte_tag, cte_fields)) = self.stmt.extract_fields(cte_id) else {
                continue;
            };
            let cte_tag_idx = u32::from(cte_tag) as usize;
            let cte_role = self
                .roles
                .get(cte_tag_idx)
                .copied()
                .unwrap_or(SemanticRole::Transparent);

            let SemanticRole::CteBinding {
                name: cte_name_idx,
                body: cte_body_idx,
            } = cte_role
            else {
                continue;
            };

            let cte_name = match cte_fields[cte_name_idx as usize] {
                FieldValue::Span(s) => s,
                _ => "",
            };
            let cte_body_id = Self::field_node_id(&cte_fields, cte_body_idx);

            if is_recursive && !cte_name.is_empty() {
                self.catalog.add_query_table(cte_name, None);
            }
            self.catalog.push_query_scope();
            self.visit_opt(cte_body_id);
            self.catalog.pop_query_scope();
            if !cte_name.is_empty() {
                self.catalog.add_query_table(cte_name, None);
            }
        }

        self.visit_opt(Self::field_node_id(fields, body_idx));
    }

    /// Visit a trigger body with OLD/NEW pseudo-tables in scope.
    fn visit_trigger_scope(&mut self, fields: &NodeFields<'a>, when_idx: u8, body_idx: u8) {
        self.catalog.push_query_scope();
        self.catalog.add_query_table("OLD", None);
        self.catalog.add_query_table("NEW", None);
        self.visit_opt(Self::field_node_id(fields, when_idx));
        self.visit_opt(Self::field_node_id(fields, body_idx));
        self.catalog.pop_query_scope();
    }
}
