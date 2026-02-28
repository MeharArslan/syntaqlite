// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::parser::NodeReader;
use crate::sqlite::ast::*;

use super::ValidationConfig;
use super::checks::{check_column_ref, check_function_call, check_table_ref};
use super::scope::ScopeStack;
use super::types::{Diagnostic, FunctionDef};

pub(super) struct Walker<'a, 'd> {
    reader: &'a NodeReader<'a>,
    dialect: crate::Dialect<'d>,
    functions: &'a [FunctionDef],
    config: &'a ValidationConfig,
    diagnostics: Vec<Diagnostic>,
}

impl<'a, 'd> Walker<'a, 'd> {
    pub(super) fn run(
        reader: &'a NodeReader<'a>,
        stmt: Stmt<'a>,
        dialect: crate::Dialect<'d>,
        scope: &mut ScopeStack,
        functions: &'a [FunctionDef],
        config: &'a ValidationConfig,
    ) -> Vec<Diagnostic> {
        let mut walker = Walker {
            reader,
            dialect,
            functions,
            config,
            diagnostics: Vec::new(),
        };
        walker.walk_stmt(stmt, scope);
        walker.diagnostics
    }

    /// Compute the byte offset of a string slice within the source.
    fn str_offset(&self, s: &str) -> usize {
        s.as_ptr() as usize - self.reader.source().as_ptr() as usize
    }

    fn walk_stmt(&mut self, stmt: Stmt<'a>, scope: &mut ScopeStack) {
        match stmt {
            Stmt::SelectStmt(s) => self.walk_select_stmt(s, scope),
            Stmt::CompoundSelect(c) => self.walk_compound_select(c, scope),
            Stmt::WithClause(w) => self.walk_with_clause(w, scope),
            Stmt::InsertStmt(i) => self.walk_insert_stmt(i, scope),
            Stmt::UpdateStmt(u) => self.walk_update_stmt(u, scope),
            Stmt::DeleteStmt(d) => self.walk_delete_stmt(d, scope),
            Stmt::CreateTableStmt(ct) => {
                if let Some(select) = ct.as_select() {
                    self.walk_select(select, scope);
                }
            }
            Stmt::CreateViewStmt(cv) => {
                if let Some(select) = cv.select() {
                    self.walk_select(select, scope);
                }
            }
            Stmt::CreateTriggerStmt(t) => self.walk_trigger_stmt(t, scope),
            Stmt::Other(node) => self.walk_other_node(node, scope),
            _ => {}
        }
    }

    fn walk_select_stmt(&mut self, select: SelectStmt<'a>, scope: &mut ScopeStack) {
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

    fn walk_compound_select(&mut self, compound: CompoundSelect<'a>, scope: &mut ScopeStack) {
        if let Some(left) = compound.left() {
            self.walk_select(left, scope);
        }
        if let Some(right) = compound.right() {
            self.walk_select(right, scope);
        }
    }

    fn walk_with_clause(&mut self, with: WithClause<'a>, scope: &mut ScopeStack) {
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

    fn walk_select(&mut self, select: Select<'a>, scope: &mut ScopeStack) {
        match select {
            Select::SelectStmt(s) => self.walk_select_stmt(s, scope),
            Select::CompoundSelect(c) => self.walk_compound_select(c, scope),
            Select::WithClause(w) => self.walk_with_clause(w, scope),
            Select::ValuesClause(v) => {
                if let Some(rows) = v.rows() {
                    for row in rows.iter() {
                        self.walk_expr_list(row, scope);
                    }
                }
            }
            Select::Other(node) => self.walk_other_node(node, scope),
        }
    }

    fn walk_insert_stmt(&mut self, insert: InsertStmt<'a>, scope: &mut ScopeStack) {
        if let Some(table_ref) = insert.table() {
            self.check_and_add_table_ref(&table_ref, scope);
        }
        if let Some(source) = insert.source() {
            self.walk_select(source, scope);
        }
    }

    fn walk_update_stmt(&mut self, update: UpdateStmt<'a>, scope: &mut ScopeStack) {
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

    fn walk_delete_stmt(&mut self, delete: DeleteStmt<'a>, scope: &mut ScopeStack) {
        if let Some(table_ref) = delete.table() {
            self.check_and_add_table_ref(&table_ref, scope);
        }
        self.walk_opt_expr(delete.where_clause(), scope);
    }

    fn walk_trigger_stmt(&mut self, trigger: CreateTriggerStmt<'a>, scope: &mut ScopeStack) {
        scope.push();
        // OLD and NEW are pseudo-tables available in trigger body commands.
        scope.add_table("OLD", None);
        scope.add_table("NEW", None);
        self.walk_opt_expr(trigger.when_expr(), scope);
        if let Some(body) = trigger.body() {
            for node in body.iter() {
                if let Some(stmt) = node_to_stmt(node) {
                    self.walk_stmt(stmt, scope);
                }
            }
        }
        scope.pop();
    }

    fn walk_table_source(&mut self, source: TableSource<'a>, scope: &mut ScopeStack) {
        match source {
            TableSource::TableRef(t) => {
                self.check_and_add_table_ref(&t, scope);
            }
            TableSource::SubqueryTableSource(sub) => {
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
            TableSource::JoinClause(join) => {
                if let Some(left) = join.left() {
                    self.walk_table_source(left, scope);
                }
                if let Some(right) = join.right() {
                    self.walk_table_source(right, scope);
                }
                self.walk_opt_expr(join.on_expr(), scope);
            }
            TableSource::JoinPrefix(jp) => {
                if let Some(src) = jp.source() {
                    self.walk_table_source(src, scope);
                }
            }
            TableSource::Other(node) => self.walk_other_node(node, scope),
        }
    }

    fn check_and_add_table_ref(&mut self, table_ref: &TableRef<'a>, scope: &mut ScopeStack) {
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

    fn walk_expr(&mut self, expr: Expr<'a>, scope: &mut ScopeStack) {
        match expr {
            Expr::ColumnRef(col) => {
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
            Expr::FunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause(), scope);
            }
            Expr::AggregateFunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause(), scope);
            }
            Expr::OrderedSetFunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause(), scope);
            }
            Expr::BinaryExpr(bin) => {
                self.walk_opt_expr(bin.left(), scope);
                self.walk_opt_expr(bin.right(), scope);
            }
            Expr::UnaryExpr(un) => {
                self.walk_opt_expr(un.operand(), scope);
            }
            Expr::IsExpr(is) => {
                self.walk_opt_expr(is.left(), scope);
                self.walk_opt_expr(is.right(), scope);
            }
            Expr::BetweenExpr(b) => {
                self.walk_opt_expr(b.operand(), scope);
                self.walk_opt_expr(b.low(), scope);
                self.walk_opt_expr(b.high(), scope);
            }
            Expr::InExpr(in_expr) => {
                self.walk_opt_expr(in_expr.operand(), scope);
                if let Some(source) = in_expr.source() {
                    match source {
                        InExprSource::ExprList(list) => {
                            self.walk_expr_list(list, scope);
                        }
                        InExprSource::SubqueryExpr(sub) => {
                            self.walk_subquery(sub.select(), scope);
                        }
                        InExprSource::Other(node) => self.walk_other_node(node, scope),
                    }
                }
            }
            Expr::CaseExpr(case) => {
                self.walk_opt_expr(case.operand(), scope);
                if let Some(whens) = case.whens() {
                    for when in whens.iter() {
                        self.walk_opt_expr(when.when_expr(), scope);
                        self.walk_opt_expr(when.then_expr(), scope);
                    }
                }
                self.walk_opt_expr(case.else_expr(), scope);
            }
            Expr::SubqueryExpr(sub) => {
                self.walk_subquery(sub.select(), scope);
            }
            Expr::ExistsExpr(exists) => {
                self.walk_subquery(exists.select(), scope);
            }
            Expr::CastExpr(cast) => {
                self.walk_opt_expr(cast.expr(), scope);
            }
            Expr::CollateExpr(collate) => {
                self.walk_opt_expr(collate.expr(), scope);
            }
            Expr::LikeExpr(like) => {
                self.walk_opt_expr(like.operand(), scope);
                self.walk_opt_expr(like.pattern(), scope);
                self.walk_opt_expr(like.escape(), scope);
            }
            Expr::Other(node) => self.walk_other_node(node, scope),
            Expr::Literal(_) | Expr::Variable(_) | Expr::RaiseExpr(_) => {}
        }
    }

    fn walk_opt_expr(&mut self, expr: Option<Expr<'a>>, scope: &mut ScopeStack) {
        if let Some(e) = expr {
            self.walk_expr(e, scope);
        }
    }

    fn walk_subquery(&mut self, select: Option<Select<'a>>, scope: &mut ScopeStack) {
        if let Some(select) = select {
            scope.push();
            self.walk_select(select, scope);
            scope.pop();
        }
    }

    fn walk_function(
        &mut self,
        name: &str,
        args: Option<ExprList<'a>>,
        filter: Option<Expr<'a>>,
        scope: &mut ScopeStack,
    ) {
        if !name.is_empty() {
            let offset = self.str_offset(name);
            let arg_count = args.as_ref().map_or(0, |a| a.len());
            if let Some(diag) = check_function_call(
                name,
                arg_count,
                offset,
                name.len(),
                self.functions,
                self.config,
            ) {
                self.diagnostics.push(diag);
            }
        }
        if let Some(args) = args {
            self.walk_expr_list(args, scope);
        }
        self.walk_opt_expr(filter, scope);
    }

    fn walk_other_node(&mut self, node: Node<'a>, scope: &mut ScopeStack) {
        let id = match node {
            Node::Other { id, .. } => id,
            _ => return,
        };
        for child_id in self.reader.child_node_ids(id, &self.dialect) {
            let child: Option<Node<'a>> = FromArena::from_arena(self.reader, child_id);
            if let Some(child) = child {
                if let Some(expr) = node_to_expr(child) {
                    self.walk_expr(expr, scope);
                }
            }
        }
    }

    fn walk_expr_list(&mut self, list: ExprList<'a>, scope: &mut ScopeStack) {
        for node in list.iter() {
            if let Some(expr) = node_to_expr(node) {
                self.walk_expr(expr, scope);
            }
        }
    }
}

fn node_to_stmt<'a>(node: Node<'a>) -> Option<Stmt<'a>> {
    Some(match node {
        Node::SelectStmt(n) => Stmt::SelectStmt(n),
        Node::CompoundSelect(n) => Stmt::CompoundSelect(n),
        Node::ValuesClause(n) => Stmt::ValuesClause(n),
        Node::WithClause(n) => Stmt::WithClause(n),
        Node::InsertStmt(n) => Stmt::InsertStmt(n),
        Node::UpdateStmt(n) => Stmt::UpdateStmt(n),
        Node::DeleteStmt(n) => Stmt::DeleteStmt(n),
        _ => return None,
    })
}

fn node_to_expr<'a>(node: Node<'a>) -> Option<Expr<'a>> {
    Some(match node {
        Node::BinaryExpr(n) => Expr::BinaryExpr(n),
        Node::UnaryExpr(n) => Expr::UnaryExpr(n),
        Node::Literal(n) => Expr::Literal(n),
        Node::ColumnRef(n) => Expr::ColumnRef(n),
        Node::Variable(n) => Expr::Variable(n),
        Node::FunctionCall(n) => Expr::FunctionCall(n),
        Node::AggregateFunctionCall(n) => Expr::AggregateFunctionCall(n),
        Node::OrderedSetFunctionCall(n) => Expr::OrderedSetFunctionCall(n),
        Node::CastExpr(n) => Expr::CastExpr(n),
        Node::CollateExpr(n) => Expr::CollateExpr(n),
        Node::CaseExpr(n) => Expr::CaseExpr(n),
        Node::IsExpr(n) => Expr::IsExpr(n),
        Node::BetweenExpr(n) => Expr::BetweenExpr(n),
        Node::LikeExpr(n) => Expr::LikeExpr(n),
        Node::InExpr(n) => Expr::InExpr(n),
        Node::SubqueryExpr(n) => Expr::SubqueryExpr(n),
        Node::ExistsExpr(n) => Expr::ExistsExpr(n),
        Node::RaiseExpr(n) => Expr::RaiseExpr(n),
        _ => return None,
    })
}

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use crate::validation::{validate_statement, ValidationConfig};

    fn validate_sql(sql: &str) -> Vec<super::super::types::Diagnostic> {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let mut cursor = parser.parse(sql);
        let mut diags = Vec::new();
        while let Some(result) = cursor.next_statement() {
            let Ok(stmt_id) = result else { break };
            diags.extend(validate_statement(
                cursor.reader(),
                stmt_id,
                *dialect,
                None,
                None,
                &[],
                &ValidationConfig::default(),
            ));
        }
        diags
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
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "CREATE TABLE src (id INTEGER);\nCREATE TABLE t AS SELECT * FROM src;";
        let mut cursor = parser.parse(sql);
        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }
        let ctx = crate::validation::SessionContext::from_stmts(cursor.reader(), &stmt_ids);
        let diags: Vec<_> = stmt_ids
            .iter()
            .flat_map(|&id| {
                validate_statement(
                    cursor.reader(),
                    id,
                    *dialect,
                    Some(&ctx),
                    None,
                    &[],
                    &ValidationConfig::default(),
                )
            })
            .collect();
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
    fn create_trigger_body_old_new_no_diags() {
        // OLD and NEW should be available in trigger body without warnings.
        let diags = validate_sql(
            "CREATE TRIGGER trg AFTER UPDATE ON t \
             BEGIN SELECT NEW.x, OLD.y FROM t; END;",
        );
        let old_new_diags: Vec<_> = diags
            .iter()
            .filter(|d| {
                d.message.contains("OLD") || d.message.contains("NEW")
            })
            .collect();
        assert!(
            old_new_diags.is_empty(),
            "OLD/NEW should not produce diagnostics in trigger body: {:?}",
            old_new_diags
        );
    }
}
