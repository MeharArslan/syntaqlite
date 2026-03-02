// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::marker::PhantomData;

use syntaqlite_parser::DialectNodeType;
use syntaqlite_parser::RawDialect;
use syntaqlite_parser::RawNodeReader;
use syntaqlite_parser::TypedList;
use syntaqlite_parser::ast_traits::*;

use crate::semantic::functions::{FunctionCatalog, FunctionCheckResult};

use super::ValidationConfig;
use super::checks::{check_column_ref, check_table_ref};
use super::fuzzy::best_suggestion;
use super::scope::ScopeStack;
use super::types::{Diagnostic, DiagnosticMessage, Help};

pub(super) struct Walker<'a, 'd, A: AstTypes<'a>> {
    reader: RawNodeReader<'a>,
    dialect: RawDialect<'d>,
    catalog: &'a FunctionCatalog,
    config: &'a ValidationConfig,
    diagnostics: Vec<Diagnostic>,
    _ast: PhantomData<A>,
}

impl<'a, 'd, A: AstTypes<'a>> Walker<'a, 'd, A> {
    pub(super) fn run(
        reader: RawNodeReader<'a>,
        stmt: A::Stmt,
        dialect: RawDialect<'d>,
        scope: &mut ScopeStack,
        catalog: &'a FunctionCatalog,
        config: &'a ValidationConfig,
    ) -> Vec<Diagnostic> {
        let mut walker: Walker<'_, '_, A> = Walker {
            reader,
            dialect,
            catalog,
            config,
            diagnostics: Vec::new(),
            _ast: PhantomData,
        };
        walker.walk_stmt(stmt, scope);
        walker.diagnostics
    }

    /// Compute the byte offset of a string slice within the source.
    fn str_offset(&self, s: &str) -> usize {
        s.as_ptr() as usize - self.reader.source().as_ptr() as usize
    }

    fn walk_stmt(&mut self, stmt: A::Stmt, scope: &mut ScopeStack) {
        match stmt.kind() {
            StmtKind::SelectStmt(s) => self.walk_select_stmt(s, scope),
            StmtKind::CompoundSelect(c) => self.walk_compound_select(c, scope),
            StmtKind::WithClause(w) => self.walk_with_clause(w, scope),
            StmtKind::InsertStmt(i) => self.walk_insert_stmt(i, scope),
            StmtKind::UpdateStmt(u) => self.walk_update_stmt(u, scope),
            StmtKind::DeleteStmt(d) => self.walk_delete_stmt(d, scope),
            StmtKind::CreateTableStmt(ct) => {
                if let Some(select) = ct.as_select() {
                    self.walk_select(select, scope);
                }
            }
            StmtKind::CreateViewStmt(cv) => {
                if let Some(select) = cv.select() {
                    self.walk_select(select, scope);
                }
            }
            StmtKind::CreateTriggerStmt(t) => self.walk_trigger_stmt(t, scope),
            StmtKind::Other(node) => self.walk_other_node(node, scope),
            _ => {}
        }
    }

    fn walk_select_stmt(&mut self, select: A::SelectStmt, scope: &mut ScopeStack) {
        // FROM first — populates scope before column refs are checked.
        if let Some(from) = select.from_clause() {
            self.walk_table_source(from, scope);
        }
        if let Some(cols) = select.columns() {
            for rc in cols.iter() {
                if let Some(expr) = rc.expr() {
                    self.walk_expr(expr, scope);
                }
            }
        }
        self.walk_opt_expr(select.where_clause(), scope);
        if let Some(groupby) = select.groupby() {
            self.walk_expr_list(groupby, scope);
        }
        self.walk_opt_expr(select.having(), scope);
        if let Some(orderby) = select.orderby() {
            for term in orderby.iter() {
                self.walk_opt_expr(term.expr(), scope);
            }
        }
        if let Some(limit) = select.limit_clause() {
            self.walk_opt_expr(limit.limit(), scope);
            self.walk_opt_expr(limit.offset(), scope);
        }
    }

    fn walk_compound_select(&mut self, compound: A::CompoundSelect, scope: &mut ScopeStack) {
        if let Some(left) = compound.left() {
            self.walk_select(left, scope);
        }
        if let Some(right) = compound.right() {
            self.walk_select(right, scope);
        }
    }

    fn walk_with_clause(&mut self, with: A::WithClause, scope: &mut ScopeStack) {
        let is_recursive = with.recursive();

        if let Some(cte_list) = with.ctes() {
            for cte in cte_list.iter() {
                let cte_name = cte.cte_name();

                if is_recursive && !cte_name.is_empty() {
                    scope.add_table(cte_name, None);
                }

                scope.push();
                if let Some(select) = cte.select() {
                    self.walk_select(select, scope);
                }
                scope.pop();

                if !cte_name.is_empty() {
                    scope.add_table(cte_name, None);
                }
            }
        }

        if let Some(select) = with.select() {
            self.walk_select(select, scope);
        }
    }

    fn walk_select(&mut self, select: A::Select, scope: &mut ScopeStack) {
        match select.kind() {
            SelectKind::SelectStmt(s) => self.walk_select_stmt(s, scope),
            SelectKind::CompoundSelect(c) => self.walk_compound_select(c, scope),
            SelectKind::WithClause(w) => self.walk_with_clause(w, scope),
            SelectKind::ValuesClause(v) => {
                if let Some(rows) = v.rows() {
                    for row in rows.iter() {
                        self.walk_expr_list(row, scope);
                    }
                }
            }
            SelectKind::Other(node) => self.walk_other_node(node, scope),
        }
    }

    fn walk_insert_stmt(&mut self, insert: A::InsertStmt, scope: &mut ScopeStack) {
        if let Some(table_ref) = insert.table() {
            self.check_and_add_table_ref(&table_ref, scope);
        }
        if let Some(source) = insert.source() {
            self.walk_select(source, scope);
        }
    }

    fn walk_update_stmt(&mut self, update: A::UpdateStmt, scope: &mut ScopeStack) {
        if let Some(table_ref) = update.table() {
            self.check_and_add_table_ref(&table_ref, scope);
        }
        if let Some(from) = update.from_clause() {
            self.walk_table_source(from, scope);
        }
        if let Some(setlist) = update.setlist() {
            for clause in setlist.iter() {
                self.walk_opt_expr(clause.value(), scope);
            }
        }
        self.walk_opt_expr(update.where_clause(), scope);
    }

    fn walk_delete_stmt(&mut self, delete: A::DeleteStmt, scope: &mut ScopeStack) {
        if let Some(table_ref) = delete.table() {
            self.check_and_add_table_ref(&table_ref, scope);
        }
        self.walk_opt_expr(delete.where_clause(), scope);
    }

    fn walk_trigger_stmt(&mut self, trigger: A::CreateTriggerStmt, scope: &mut ScopeStack) {
        scope.push();
        // OLD and NEW are pseudo-tables available in trigger body commands.
        scope.add_table("OLD", None);
        scope.add_table("NEW", None);
        self.walk_opt_expr(trigger.when_expr(), scope);
        if let Some(body) = trigger.body() {
            for node in body.iter() {
                let id = node.node_id();
                if let Some(stmt) = A::Stmt::from_arena(self.reader, id) {
                    self.walk_stmt(stmt, scope);
                }
            }
        }
        scope.pop();
    }

    fn walk_table_source(&mut self, source: A::TableSource, scope: &mut ScopeStack) {
        match source.kind() {
            TableSourceKind::TableRef(t) => {
                self.check_and_add_table_ref(&t, scope);
            }
            TableSourceKind::SubqueryTableSource(sub) => {
                scope.push();
                if let Some(select) = sub.select() {
                    self.walk_select(select, scope);
                }
                scope.pop();
                let alias = sub.alias();
                if !alias.is_empty() {
                    scope.add_table(alias, None);
                }
            }
            TableSourceKind::JoinClause(join) => {
                if let Some(left) = join.left() {
                    self.walk_table_source(left, scope);
                }
                if let Some(right) = join.right() {
                    self.walk_table_source(right, scope);
                }
                self.walk_opt_expr(join.on_expr(), scope);
            }
            TableSourceKind::JoinPrefix(jp) => {
                if let Some(src) = jp.source() {
                    self.walk_table_source(src, scope);
                }
            }
            TableSourceKind::Other(node) => self.walk_other_node(node, scope),
        }
    }

    fn check_and_add_table_ref(&mut self, table_ref: &A::TableRef, scope: &mut ScopeStack) {
        let name = table_ref.table_name();
        if name.is_empty() {
            return;
        }

        let offset = self.str_offset(name);
        if let Some(diag) = check_table_ref(name, offset, name.len(), scope, self.config) {
            self.diagnostics.push(diag);
        }

        let alias = table_ref.alias();
        let scope_name = if alias.is_empty() { name } else { alias };
        let columns = scope.ambient_columns_for_table(name);
        scope.add_table(scope_name, columns);
    }

    fn walk_expr(&mut self, expr: A::Expr, scope: &mut ScopeStack) {
        match expr.kind() {
            ExprKind::ColumnRef(col) => {
                let column = col.column();
                if column.is_empty() {
                    return;
                }
                let table = col.table();
                let offset = self.str_offset(column);
                let table_opt = if table.is_empty() { None } else { Some(table) };

                if let Some(diag) =
                    check_column_ref(table_opt, column, offset, column.len(), scope, self.config)
                {
                    self.diagnostics.push(diag);
                }
            }
            ExprKind::FunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause(), scope);
            }
            ExprKind::AggregateFunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause(), scope);
            }
            ExprKind::OrderedSetFunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause(), scope);
            }
            ExprKind::BinaryExpr(bin) => {
                self.walk_opt_expr(bin.left(), scope);
                self.walk_opt_expr(bin.right(), scope);
            }
            ExprKind::UnaryExpr(un) => {
                self.walk_opt_expr(un.operand(), scope);
            }
            ExprKind::IsExpr(is) => {
                self.walk_opt_expr(is.left(), scope);
                self.walk_opt_expr(is.right(), scope);
            }
            ExprKind::BetweenExpr(b) => {
                self.walk_opt_expr(b.operand(), scope);
                self.walk_opt_expr(b.low(), scope);
                self.walk_opt_expr(b.high(), scope);
            }
            ExprKind::InExpr(in_expr) => {
                self.walk_opt_expr(in_expr.operand(), scope);
                if let Some(source) = in_expr.source() {
                    match source.kind() {
                        InExprSourceKind::ExprList(list) => {
                            self.walk_expr_list(list, scope);
                        }
                        InExprSourceKind::SubqueryExpr(sub) => {
                            self.walk_subquery(sub.select(), scope);
                        }
                        InExprSourceKind::Other(node) => self.walk_other_node(node, scope),
                    }
                }
            }
            ExprKind::CaseExpr(case) => {
                self.walk_opt_expr(case.operand(), scope);
                if let Some(whens) = case.whens() {
                    for when in whens.iter() {
                        self.walk_opt_expr(when.when_expr(), scope);
                        self.walk_opt_expr(when.then_expr(), scope);
                    }
                }
                self.walk_opt_expr(case.else_expr(), scope);
            }
            ExprKind::SubqueryExpr(sub) => {
                self.walk_subquery(sub.select(), scope);
            }
            ExprKind::ExistsExpr(exists) => {
                self.walk_subquery(exists.select(), scope);
            }
            ExprKind::CastExpr(cast) => {
                self.walk_opt_expr(cast.expr(), scope);
            }
            ExprKind::CollateExpr(collate) => {
                self.walk_opt_expr(collate.expr(), scope);
            }
            ExprKind::LikeExpr(like) => {
                self.walk_opt_expr(like.operand(), scope);
                self.walk_opt_expr(like.pattern(), scope);
                self.walk_opt_expr(like.escape(), scope);
            }
            ExprKind::Other(node) => self.walk_other_node(node, scope),
            ExprKind::Literal(_) | ExprKind::Variable(_) | ExprKind::RaiseExpr(_) => {}
        }
    }

    fn walk_opt_expr(&mut self, expr: Option<A::Expr>, scope: &mut ScopeStack) {
        if let Some(e) = expr {
            self.walk_expr(e, scope);
        }
    }

    fn walk_subquery(&mut self, select: Option<A::Select>, scope: &mut ScopeStack) {
        if let Some(select) = select {
            scope.push();
            self.walk_select(select, scope);
            scope.pop();
        }
    }

    fn walk_function(
        &mut self,
        name: &str,
        args: Option<TypedList<'a, A::Node>>,
        filter: Option<A::Expr>,
        scope: &mut ScopeStack,
    ) {
        if !name.is_empty() {
            let offset = self.str_offset(name);
            let arg_count = args.as_ref().map_or(0, |a| a.len());
            match self.catalog.check_call(name, arg_count) {
                FunctionCheckResult::Ok => {}
                FunctionCheckResult::Unknown => {
                    let all_names = self.catalog.all_names();
                    let suggestion =
                        best_suggestion(name, &all_names, self.config.suggestion_threshold);
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
        if let Some(args) = args {
            self.walk_expr_list(args, scope);
        }
        self.walk_opt_expr(filter, scope);
    }

    fn walk_other_node(&mut self, node: A::Node, scope: &mut ScopeStack) {
        let id = node.node_id();
        if id.is_null() {
            return;
        }
        for child_id in self.reader.child_node_ids(id, &self.dialect) {
            // Check Stmt before Expr: Expr::from_arena has a catch-all Other
            // variant that matches any node (including SelectStmt), so checking
            // Expr first would route statement children through walk_expr
            // instead of walk_stmt, skipping FROM-clause table resolution.
            if let Some(stmt) = A::Stmt::from_arena(self.reader, child_id) {
                self.walk_stmt(stmt, scope);
            } else if let Some(expr) = A::Expr::from_arena(self.reader, child_id) {
                self.walk_expr(expr, scope);
            }
        }
    }

    fn walk_expr_list(&mut self, list: TypedList<'a, A::Node>, scope: &mut ScopeStack) {
        for node in list.iter() {
            let id = node.node_id();
            if let Some(expr) = A::Expr::from_arena(self.reader, id) {
                self.walk_expr(expr, scope);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::validation::{ValidationConfig, Validator};

    fn validate_sql(sql: &str) -> Vec<super::super::types::Diagnostic> {
        let mut validator = Validator::new();
        validator.validate(sql, None, &ValidationConfig::default())
    }

    #[test]
    fn create_table_as_select_unknown_table_warns() {
        let diags = validate_sql("CREATE TABLE t AS SELECT * FROM nonexistent;");
        assert!(
            !diags.is_empty(),
            "expected a diagnostic for unknown table in CREATE TABLE AS SELECT"
        );
    }

    #[test]
    fn create_table_as_select_known_table_no_diags() {
        let sql = "CREATE TABLE src (id INTEGER);\nCREATE TABLE t AS SELECT * FROM src;";
        let diags = validate_sql(sql);
        assert!(
            diags.is_empty(),
            "expected no diagnostics when source table is known: {:?}",
            diags
        );
    }

    #[test]
    fn create_view_as_select_unknown_table_warns() {
        let diags = validate_sql("CREATE VIEW v AS SELECT * FROM nonexistent;");
        assert!(
            !diags.is_empty(),
            "expected a diagnostic for unknown table in CREATE VIEW AS SELECT"
        );
    }

    #[test]
    fn create_trigger_body_unknown_table_warns() {
        let diags = validate_sql(
            "CREATE TRIGGER trg AFTER INSERT ON t \
             BEGIN SELECT * FROM nonexistent; END;",
        );
        assert!(
            !diags.is_empty(),
            "expected a diagnostic for unknown table in trigger body"
        );
    }

    #[test]
    fn create_table_as_select_literal_column_mismatch_warns() {
        // CREATE TABLE slice AS SELECT 2 → table has no named columns.
        // Referencing slice."1" should warn about unknown column.
        let dialect = syntaqlite_parser_sqlite::dialect();
        let mut parser = syntaqlite_parser::RawParser::new(dialect);
        let sql = "CREATE TABLE slice AS SELECT 2;\nSELECT slice.\"1\" FROM slice;";
        let mut cursor = parser.parse(sql);
        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr: syntaqlite_parser::NodeRef<'_>| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let catalog = crate::semantic::functions::FunctionCatalog::for_default_dialect(&dialect);
        let diags = crate::validation::validate_document(
            cursor.reader(),
            &stmt_ids,
            dialect,
            None,
            &catalog,
            &ValidationConfig::default(),
        );
        let col_diags: Vec<_> = diags
            .iter()
            .filter(|d| d.message.to_string().contains("column"))
            .collect();
        assert!(
            !col_diags.is_empty(),
            "expected a diagnostic for unknown column '1' in table 'slice'"
        );
    }

    #[test]
    fn create_trigger_body_old_new_no_diags() {
        // OLD and NEW should be available in trigger body without warnings.
        let diags = validate_sql(
            "CREATE TRIGGER trg AFTER UPDATE ON t \
             BEGIN SELECT NEW.x, OLD.y FROM t; END;",
        );
        let old_new_diags: Vec<_> = diags
            .iter()
            .filter(|d| {
                let s = d.message.to_string();
                s.contains("OLD") || s.contains("NEW")
            })
            .collect();
        assert!(
            old_new_diags.is_empty(),
            "OLD/NEW should not produce diagnostics in trigger body: {:?}",
            old_new_diags
        );
    }
}
