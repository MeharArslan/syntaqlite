// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::marker::PhantomData;

use syntaqlite_syntax::any::AnyParsedStatement;
#[allow(clippy::wildcard_imports)]
use syntaqlite_syntax::ast_traits::*;
use syntaqlite_syntax::typed::{GrammarNodeType, TypedNodeList};

use super::ValidationConfig;
use super::catalog::CatalogStack;
use super::catalog::FunctionCheckResult;
use super::checks::{check_column_ref, check_table_ref};
use super::diagnostics::{Diagnostic, DiagnosticMessage, Help};
use super::fuzzy::best_suggestion;
use super::scope::ScopeStack;

#[derive(Clone, Copy)]
pub(crate) struct WalkContext<'a> {
    pub(crate) catalog: &'a CatalogStack<'a>,
    pub(crate) config: &'a ValidationConfig,
}

pub(crate) struct Walker<'a, A: AstTypes<'a>> {
    stmt_result: AnyParsedStatement<'a>,
    ctx: WalkContext<'a>,
    diagnostics: Vec<Diagnostic>,
    _ast: PhantomData<A>,
}

impl<'a, A: AstTypes<'a>> Walker<'a, A> {
    pub(crate) fn run(
        stmt_result: AnyParsedStatement<'a>,
        stmt: A::Stmt,
        scope: &mut ScopeStack,
        ctx: WalkContext<'a>,
    ) -> Vec<Diagnostic> {
        let mut walker: Walker<'_, A> = Walker {
            stmt_result,
            ctx,
            diagnostics: Vec::new(),
            _ast: PhantomData,
        };
        walker.walk_stmt(stmt, scope);
        walker.diagnostics
    }

    /// Compute the byte offset of a string slice within the source.
    fn str_offset(&self, s: &str) -> usize {
        s.as_ptr() as usize - self.stmt_result.source().as_ptr() as usize
    }

    fn catalog(&self) -> &CatalogStack<'a> {
        self.ctx.catalog
    }

    fn config(&self) -> &ValidationConfig {
        self.ctx.config
    }

    fn push_diag(&mut self, diag: Option<Diagnostic>) {
        if let Some(diag) = diag {
            self.diagnostics.push(diag);
        }
    }

    fn with_scope<F>(&mut self, scope: &mut ScopeStack, f: F)
    where
        F: FnOnce(&mut Self, &mut ScopeStack),
    {
        scope.push();
        f(self, scope);
        scope.pop();
    }

    fn walk_opt_select(&mut self, select: Option<A::Select>, scope: &mut ScopeStack) {
        if let Some(select) = select {
            self.walk_select(select, scope);
        }
    }

    fn walk_opt_table_source(&mut self, source: Option<A::TableSource>, scope: &mut ScopeStack) {
        if let Some(source) = source {
            self.walk_table_source(source, scope);
        }
    }

    fn walk_opt_table_ref(&mut self, table_ref: Option<A::TableRef>, scope: &mut ScopeStack) {
        if let Some(table_ref) = table_ref {
            self.check_and_add_table_ref(&table_ref, scope);
        }
    }

    fn emit_unknown_function(&mut self, name: &str, offset: usize) {
        let all_names = self.catalog().all_function_names();
        let suggestion = best_suggestion(name, &all_names, self.config().suggestion_threshold);
        self.diagnostics.push(Diagnostic {
            start_offset: offset,
            end_offset: offset + name.len(),
            message: DiagnosticMessage::UnknownFunction {
                name: name.to_string(),
            },
            severity: self.config().severity(),
            help: suggestion.map(Help::Suggestion),
        });
    }

    fn emit_wrong_arity(&mut self, name: &str, offset: usize, expected: Vec<usize>, got: usize) {
        self.diagnostics.push(Diagnostic {
            start_offset: offset,
            end_offset: offset + name.len(),
            message: DiagnosticMessage::FunctionArity {
                name: name.to_string(),
                expected,
                got,
            },
            severity: self.config().severity(),
            help: None,
        });
    }

    fn walk_stmt(&mut self, stmt: A::Stmt, scope: &mut ScopeStack) {
        match stmt.kind() {
            StmtKind::SelectStmt(s) => self.walk_select_stmt(s, scope),
            StmtKind::CompoundSelect(c) => self.walk_compound_select(c, scope),
            StmtKind::WithClause(w) => self.walk_with_clause(w, scope),
            StmtKind::InsertStmt(i) => self.walk_insert_stmt(i, scope),
            StmtKind::UpdateStmt(u) => self.walk_update_stmt(u, scope),
            StmtKind::DeleteStmt(d) => self.walk_delete_stmt(d, scope),
            StmtKind::CreateTableStmt(ct) => self.walk_opt_select(ct.as_select(), scope),
            StmtKind::CreateViewStmt(cv) => self.walk_opt_select(cv.select(), scope),
            StmtKind::CreateTriggerStmt(t) => self.walk_trigger_stmt(t, scope),
            StmtKind::Other(node) => self.walk_other_node(node, scope),
            _ => {}
        }
    }

    fn walk_select_stmt(&mut self, select: A::SelectStmt, scope: &mut ScopeStack) {
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

                self.with_scope(scope, |this, scope| {
                    this.walk_opt_select(cte.select(), scope)
                });

                if !cte_name.is_empty() {
                    scope.add_table(cte_name, None);
                }
            }
        }

        self.walk_opt_select(with.select(), scope);
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
        self.walk_opt_table_ref(insert.table(), scope);
        self.walk_opt_select(insert.source(), scope);
    }

    fn walk_update_stmt(&mut self, update: A::UpdateStmt, scope: &mut ScopeStack) {
        self.walk_opt_table_ref(update.table(), scope);
        self.walk_opt_table_source(update.from_clause(), scope);
        if let Some(setlist) = update.setlist() {
            for clause in setlist.iter() {
                self.walk_opt_expr(clause.value(), scope);
            }
        }
        self.walk_opt_expr(update.where_clause(), scope);
    }

    fn walk_delete_stmt(&mut self, delete: A::DeleteStmt, scope: &mut ScopeStack) {
        self.walk_opt_table_ref(delete.table(), scope);
        self.walk_opt_expr(delete.where_clause(), scope);
    }

    fn walk_trigger_stmt(&mut self, trigger: A::CreateTriggerStmt, scope: &mut ScopeStack) {
        self.with_scope(scope, |this, scope| {
            scope.add_table("OLD", None);
            scope.add_table("NEW", None);
            this.walk_opt_expr(trigger.when_expr(), scope);
            if let Some(body) = trigger.body() {
                for stmt in body.iter() {
                    this.walk_stmt(stmt, scope);
                }
            }
        });
    }

    fn walk_table_source(&mut self, source: A::TableSource, scope: &mut ScopeStack) {
        match source.kind() {
            TableSourceKind::TableRef(t) => {
                self.check_and_add_table_ref(&t, scope);
            }
            TableSourceKind::SubqueryTableSource(sub) => {
                self.with_scope(scope, |this, scope| {
                    this.walk_opt_select(sub.select(), scope)
                });
                let alias = sub.alias();
                if !alias.is_empty() {
                    scope.add_table(alias, None);
                }
            }
            TableSourceKind::JoinClause(join) => {
                self.walk_opt_table_source(join.left(), scope);
                self.walk_opt_table_source(join.right(), scope);
                self.walk_opt_expr(join.on_expr(), scope);
            }
            TableSourceKind::JoinPrefix(jp) => self.walk_opt_table_source(jp.source(), scope),
            TableSourceKind::Other(node) => self.walk_other_node(node, scope),
        }
    }

    fn check_and_add_table_ref(&mut self, table_ref: &A::TableRef, scope: &mut ScopeStack) {
        let name = table_ref.table_name();
        if name.is_empty() {
            return;
        }

        let offset = self.str_offset(name);
        self.push_diag(check_table_ref(
            name,
            offset,
            name.len(),
            scope,
            self.config(),
        ));

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

                self.push_diag(check_column_ref(
                    table_opt,
                    column,
                    offset,
                    column.len(),
                    scope,
                    self.config(),
                ));
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
        self.with_scope(scope, |this, scope| this.walk_opt_select(select, scope));
    }

    fn walk_function(
        &mut self,
        name: &str,
        args: Option<TypedNodeList<'a, A::Grammar, A::Expr>>,
        filter: Option<A::Expr>,
        scope: &mut ScopeStack,
    ) {
        if !name.is_empty() {
            let offset = self.str_offset(name);
            let arg_count = args.as_ref().map_or(0, TypedNodeList::len);
            match self.catalog().check_function(name, arg_count) {
                FunctionCheckResult::Ok => {}
                FunctionCheckResult::Unknown => self.emit_unknown_function(name, offset),
                FunctionCheckResult::WrongArity { expected } => {
                    self.emit_wrong_arity(name, offset, expected, arg_count);
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
        let child_ids: Vec<_> = self.stmt_result.child_node_ids(id).collect();
        for child_id in child_ids {
            // Check Stmt before Expr: Expr::from_result has a catch-all Other
            // variant that matches any node (including SelectStmt), so checking
            // Expr first would route statement children through walk_expr
            // instead of walk_stmt, skipping FROM-clause table resolution.
            if let Some(stmt) = A::Stmt::from_result(self.stmt_result, child_id) {
                self.walk_stmt(stmt, scope);
            } else if let Some(expr) = A::Expr::from_result(self.stmt_result, child_id) {
                self.walk_expr(expr, scope);
            }
        }
    }

    fn walk_expr_list(
        &mut self,
        list: TypedNodeList<'a, A::Grammar, A::Expr>,
        scope: &mut ScopeStack,
    ) {
        for expr in list.iter() {
            self.walk_expr(expr, scope);
        }
    }
}
