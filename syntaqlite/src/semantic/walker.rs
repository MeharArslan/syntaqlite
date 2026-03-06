// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! AST walker: traverses a parsed statement, resolves names against the
//! catalog, and collects diagnostics.

use std::marker::PhantomData;

use syntaqlite_syntax::any::AnyParsedStatement;
#[allow(clippy::wildcard_imports)]
use syntaqlite_syntax::ast_traits::*;
use syntaqlite_syntax::typed::TypedNodeList;

use super::ValidationConfig;
use super::catalog::{Catalog, ColumnResolution, FunctionCheckResult};
use super::diagnostics::{Diagnostic, DiagnosticMessage, Help};
use super::fuzzy::best_suggestion;

/// Context threaded through the walker. Holds a mutable catalog reference so
/// scopes can be pushed/popped in-place without rebuilding on each lookup.
pub(crate) struct WalkContext<'a> {
    pub catalog: &'a mut Catalog,
    pub config: &'a ValidationConfig,
}

pub(crate) struct Walker<'a, A: AstTypes<'a>> {
    stmt: AnyParsedStatement<'a>,
    ctx: WalkContext<'a>,
    diagnostics: Vec<Diagnostic>,
    _ast: PhantomData<A>,
}

impl<'a, A: AstTypes<'a>> Walker<'a, A> {
    /// Run the walker over a single parsed statement and return all diagnostics.
    pub(crate) fn run(
        stmt: AnyParsedStatement<'a>,
        root: A::Stmt,
        ctx: WalkContext<'a>,
    ) -> Vec<Diagnostic> {
        let mut walker: Walker<'_, A> = Walker {
            stmt,
            ctx,
            diagnostics: Vec::new(),
            _ast: PhantomData,
        };
        walker.walk_stmt(root);
        walker.diagnostics
    }

    fn str_offset(&self, s: &str) -> usize {
        s.as_ptr() as usize - self.stmt.source().as_ptr() as usize
    }

    /// Run `f` inside a pushed/popped query scope.
    fn with_scope<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Self),
    {
        self.ctx.catalog.push_query_scope();
        f(self);
        self.ctx.catalog.pop_query_scope();
    }

    fn walk_opt_select(&mut self, select: Option<A::Select>) {
        if let Some(s) = select {
            self.walk_select(s);
        }
    }

    fn walk_opt_table_source(&mut self, source: Option<A::TableSource>) {
        if let Some(s) = source {
            self.walk_table_source(s);
        }
    }

    fn walk_opt_expr(&mut self, expr: Option<A::Expr>) {
        if let Some(e) = expr {
            self.walk_expr(e);
        }
    }

    fn walk_stmt(&mut self, stmt: A::Stmt) {
        match stmt.kind() {
            StmtKind::SelectStmt(s) => self.walk_select_stmt(s),
            StmtKind::CompoundSelect(c) => self.walk_compound_select(c),
            StmtKind::WithClause(w) => self.walk_with_clause(w),
            StmtKind::InsertStmt(i) => self.walk_insert_stmt(i),
            StmtKind::UpdateStmt(u) => self.walk_update_stmt(u),
            StmtKind::DeleteStmt(d) => self.walk_delete_stmt(d),
            StmtKind::CreateTableStmt(ct) => self.walk_opt_select(ct.as_select()),
            StmtKind::CreateViewStmt(cv) => self.walk_opt_select(cv.select()),
            StmtKind::CreateTriggerStmt(t) => self.walk_trigger_stmt(t),
            _ => {}
        }
    }

    fn walk_select(&mut self, select: A::Select) {
        match select.kind() {
            SelectKind::SelectStmt(s) => self.walk_select_stmt(s),
            SelectKind::CompoundSelect(c) => self.walk_compound_select(c),
            SelectKind::WithClause(w) => self.walk_with_clause(w),
            SelectKind::ValuesClause(v) => {
                if let Some(rows) = v.rows() {
                    for row in rows.iter() {
                        self.walk_expr_list(row);
                    }
                }
            }
        }
    }

    fn walk_select_stmt(&mut self, s: A::SelectStmt) {
        if let Some(from) = s.from_clause() {
            self.walk_table_source(from);
        }
        if let Some(cols) = s.columns() {
            for rc in cols.iter() {
                self.walk_opt_expr(rc.expr());
            }
        }
        self.walk_opt_expr(s.where_clause());
        if let Some(gb) = s.groupby() {
            self.walk_expr_list(gb);
        }
        self.walk_opt_expr(s.having());
        if let Some(ob) = s.orderby() {
            for term in ob.iter() {
                self.walk_opt_expr(term.expr());
            }
        }
        if let Some(lim) = s.limit_clause() {
            self.walk_opt_expr(lim.limit());
            self.walk_opt_expr(lim.offset());
        }
    }

    fn walk_compound_select(&mut self, c: A::CompoundSelect) {
        if let Some(left) = c.left() {
            self.walk_select(left);
        }
        if let Some(right) = c.right() {
            self.walk_select(right);
        }
    }

    fn walk_with_clause(&mut self, w: A::WithClause) {
        let is_recursive = w.recursive();
        if let Some(ctes) = w.ctes() {
            for cte in ctes.iter() {
                let name = cte.cte_name();
                if is_recursive && !name.is_empty() {
                    self.ctx.catalog.add_query_table(name, None);
                }
                self.with_scope(|this| this.walk_opt_select(cte.select()));
                if !name.is_empty() {
                    self.ctx.catalog.add_query_table(name, None);
                }
            }
        }
        self.walk_opt_select(w.select());
    }

    fn walk_insert_stmt(&mut self, i: A::InsertStmt) {
        if let Some(t) = i.table() {
            self.check_and_add_table_ref(t);
        }
        self.walk_opt_select(i.source());
    }

    fn walk_update_stmt(&mut self, u: A::UpdateStmt) {
        if let Some(t) = u.table() {
            self.check_and_add_table_ref(t);
        }
        self.walk_opt_table_source(u.from_clause());
        if let Some(set) = u.setlist() {
            for clause in set.iter() {
                self.walk_opt_expr(clause.value());
            }
        }
        self.walk_opt_expr(u.where_clause());
    }

    fn walk_delete_stmt(&mut self, d: A::DeleteStmt) {
        if let Some(t) = d.table() {
            self.check_and_add_table_ref(t);
        }
        self.walk_opt_expr(d.where_clause());
    }

    fn walk_trigger_stmt(&mut self, t: A::CreateTriggerStmt) {
        self.with_scope(|this| {
            this.ctx.catalog.add_query_table("OLD", None);
            this.ctx.catalog.add_query_table("NEW", None);
            this.walk_opt_expr(t.when_expr());
            if let Some(body) = t.body() {
                for stmt in body.iter() {
                    this.walk_stmt(stmt);
                }
            }
        });
    }

    fn walk_table_source(&mut self, source: A::TableSource) {
        match source.kind() {
            TableSourceKind::TableRef(tr) => {
                self.check_and_add_table_ref(tr);
            }
            TableSourceKind::SubqueryTableSource(sq) => {
                self.with_scope(|this| this.walk_opt_select(sq.select()));
                let alias = name_str::<A>(sq.alias());
                if !alias.is_empty() {
                    self.ctx.catalog.add_query_table(alias, None);
                }
            }
            TableSourceKind::JoinClause(jc) => {
                self.walk_opt_table_source(jc.left());
                self.walk_opt_table_source(jc.right());
                self.walk_opt_expr(jc.on_expr());
            }
            TableSourceKind::JoinPrefix(jp) => self.walk_opt_table_source(jp.source()),
        }
    }

    fn check_and_add_table_ref(&mut self, tr: A::TableRef) {
        let name = tr.table_name();
        if name.is_empty() {
            return;
        }
        let offset = self.str_offset(name);

        let is_known = self.ctx.catalog.resolve_relation(name)
            || self.ctx.catalog.resolve_table_function(name);
        if !is_known {
            let mut candidates = self.ctx.catalog.all_relation_names();
            candidates.extend(self.ctx.catalog.all_table_function_names());
            let suggestion =
                best_suggestion(name, &candidates, self.ctx.config.suggestion_threshold);
            self.diagnostics.push(Diagnostic {
                start_offset: offset,
                end_offset: offset + name.len(),
                message: DiagnosticMessage::UnknownTable {
                    name: name.to_string(),
                },
                severity: self.ctx.config.severity(),
                help: suggestion.map(Help::Suggestion),
            });
        }

        let alias = name_str::<A>(tr.alias());
        let scope_name = if alias.is_empty() { name } else { alias };
        let columns = self.ctx.catalog.columns_for_table_source(name);
        self.ctx.catalog.add_query_table(scope_name, columns);
    }

    fn check_column_ref(&mut self, cr: A::ColumnRef) {
        let column = cr.column();
        if column.is_empty() {
            return;
        }
        let table = cr.table();
        let table = if table.is_empty() { None } else { Some(table) };
        let offset = self.str_offset(column);

        match self.ctx.catalog.resolve_column(table, column) {
            ColumnResolution::Found | ColumnResolution::TableNotFound => {}
            ColumnResolution::TableFoundColumnMissing => {
                let tbl = table.expect("qualifier present when TableFoundColumnMissing");
                let candidates = self.ctx.catalog.all_column_names(Some(tbl));
                let suggestion =
                    best_suggestion(column, &candidates, self.ctx.config.suggestion_threshold);
                self.diagnostics.push(Diagnostic {
                    start_offset: offset,
                    end_offset: offset + column.len(),
                    message: DiagnosticMessage::UnknownColumn {
                        column: column.to_string(),
                        table: Some(tbl.to_string()),
                    },
                    severity: self.ctx.config.severity(),
                    help: suggestion.map(Help::Suggestion),
                });
            }
            ColumnResolution::NotFound => {
                let candidates = self.ctx.catalog.all_column_names(None);
                let suggestion =
                    best_suggestion(column, &candidates, self.ctx.config.suggestion_threshold);
                self.diagnostics.push(Diagnostic {
                    start_offset: offset,
                    end_offset: offset + column.len(),
                    message: DiagnosticMessage::UnknownColumn {
                        column: column.to_string(),
                        table: None,
                    },
                    severity: self.ctx.config.severity(),
                    help: suggestion.map(Help::Suggestion),
                });
            }
        }
    }

    fn walk_function(
        &mut self,
        name: &str,
        args: Option<TypedNodeList<'a, A::Grammar, A::Expr>>,
        filter: Option<A::Expr>,
    ) {
        if !name.is_empty() {
            let offset = self.str_offset(name);
            let arg_count = args.as_ref().map_or(0, TypedNodeList::len);
            match self.ctx.catalog.check_function(name, arg_count) {
                FunctionCheckResult::Ok => {}
                FunctionCheckResult::Unknown => {
                    let candidates = self.ctx.catalog.all_function_names();
                    let suggestion =
                        best_suggestion(name, &candidates, self.ctx.config.suggestion_threshold);
                    self.diagnostics.push(Diagnostic {
                        start_offset: offset,
                        end_offset: offset + name.len(),
                        message: DiagnosticMessage::UnknownFunction {
                            name: name.to_string(),
                        },
                        severity: self.ctx.config.severity(),
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
                        severity: self.ctx.config.severity(),
                        help: None,
                    });
                }
            }
        }
        if let Some(args) = args {
            self.walk_expr_list(args);
        }
        self.walk_opt_expr(filter);
    }

    fn walk_expr(&mut self, expr: A::Expr) {
        match expr.kind() {
            ExprKind::ColumnRef(cr) => self.check_column_ref(cr),
            ExprKind::FunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause());
            }
            ExprKind::AggregateFunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause());
            }
            ExprKind::OrderedSetFunctionCall(f) => {
                self.walk_function(f.func_name(), f.args(), f.filter_clause());
            }
            ExprKind::BinaryExpr(bin) => {
                self.walk_opt_expr(bin.left());
                self.walk_opt_expr(bin.right());
            }
            ExprKind::UnaryExpr(un) => self.walk_opt_expr(un.operand()),
            ExprKind::IsExpr(is) => {
                self.walk_opt_expr(is.left());
                self.walk_opt_expr(is.right());
            }
            ExprKind::BetweenExpr(b) => {
                self.walk_opt_expr(b.operand());
                self.walk_opt_expr(b.low());
                self.walk_opt_expr(b.high());
            }
            ExprKind::InExpr(ie) => {
                self.walk_opt_expr(ie.operand());
                if let Some(src) = ie.source() {
                    match src.kind() {
                        InExprSourceKind::ExprList(list) => self.walk_expr_list(list),
                        InExprSourceKind::SubqueryExpr(sub) => {
                            self.with_scope(|this| this.walk_opt_select(sub.select()));
                        }
                    }
                }
            }
            ExprKind::CaseExpr(case) => {
                self.walk_opt_expr(case.operand());
                if let Some(whens) = case.whens() {
                    for when in whens.iter() {
                        self.walk_opt_expr(when.when_expr());
                        self.walk_opt_expr(when.then_expr());
                    }
                }
                self.walk_opt_expr(case.else_expr());
            }
            ExprKind::SubqueryExpr(sub) => {
                self.with_scope(|this| this.walk_opt_select(sub.select()));
            }
            ExprKind::ExistsExpr(exists) => {
                self.with_scope(|this| this.walk_opt_select(exists.select()));
            }
            ExprKind::CastExpr(cast) => self.walk_opt_expr(cast.expr()),
            ExprKind::CollateExpr(col) => self.walk_opt_expr(col.expr()),
            ExprKind::LikeExpr(like) => {
                self.walk_opt_expr(like.operand());
                self.walk_opt_expr(like.pattern());
                self.walk_opt_expr(like.escape());
            }
            ExprKind::Error(_)
            | ExprKind::Literal(_)
            | ExprKind::Variable(_)
            | ExprKind::RaiseExpr(_) => {}
        }
    }

    fn walk_expr_list(&mut self, list: TypedNodeList<'a, A::Grammar, A::Expr>) {
        for expr in list.iter() {
            self.walk_expr(expr);
        }
    }
}

/// Extract the source text from an optional `Name` node, returning `""` if absent.
fn name_str<'a, A: AstTypes<'a>>(name: Option<A::Name>) -> &'a str {
    match name {
        Some(n) => match n.kind() {
            NameKind::IdentName(ident) => ident.source(),
            NameKind::Error(_) => "",
        },
        None => "",
    }
}
