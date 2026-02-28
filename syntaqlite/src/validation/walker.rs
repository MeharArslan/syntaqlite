// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use crate::parser::NodeReader;
use crate::sqlite::ast::*;

use super::checks::{check_column_ref, check_function_call, check_table_ref};
use super::scope::ScopeStack;
use super::types::{Diagnostic, FunctionDef};
use super::ValidationConfig;

/// Walk a top-level statement and collect semantic diagnostics.
pub fn walk_stmt<'a>(
    reader: &'a NodeReader<'a>,
    stmt: Stmt<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match stmt {
        Stmt::SelectStmt(s) => walk_select_stmt(reader, s, scope, functions, config, diagnostics),
        Stmt::CompoundSelect(c) => {
            walk_compound_select(reader, c, scope, functions, config, diagnostics)
        }
        Stmt::WithClause(w) => walk_with_clause(reader, w, scope, functions, config, diagnostics),
        Stmt::InsertStmt(i) => walk_insert_stmt(reader, i, scope, functions, config, diagnostics),
        Stmt::UpdateStmt(u) => walk_update_stmt(reader, u, scope, functions, config, diagnostics),
        Stmt::DeleteStmt(d) => walk_delete_stmt(reader, d, scope, functions, config, diagnostics),
        _ => {}
    }
}

fn walk_select_stmt<'a>(
    reader: &'a NodeReader<'a>,
    select: SelectStmt<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // FROM first — populates scope before column refs are checked.
    if let Some(from) = select.from_clause() {
        walk_table_source(reader, from, scope, functions, config, diagnostics);
    }
    if let Some(cols) = select.columns() {
        for rc in cols.iter() {
            if let Some(expr) = rc.expr() {
                walk_expr(reader, expr, scope, functions, config, diagnostics);
            }
        }
    }
    walk_opt_expr(reader, select.where_clause(), scope, functions, config, diagnostics);
    if let Some(groupby) = select.groupby() {
        walk_expr_list(reader, groupby, scope, functions, config, diagnostics);
    }
    walk_opt_expr(reader, select.having(), scope, functions, config, diagnostics);
    if let Some(orderby) = select.orderby() {
        for term in orderby.iter() {
            walk_opt_expr(reader, term.expr(), scope, functions, config, diagnostics);
        }
    }
    if let Some(limit) = select.limit_clause() {
        walk_opt_expr(reader, limit.limit(), scope, functions, config, diagnostics);
        walk_opt_expr(reader, limit.offset(), scope, functions, config, diagnostics);
    }
}

fn walk_compound_select<'a>(
    reader: &'a NodeReader<'a>,
    compound: CompoundSelect<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(left) = compound.left() {
        walk_select(reader, left, scope, functions, config, diagnostics);
    }
    if let Some(right) = compound.right() {
        walk_select(reader, right, scope, functions, config, diagnostics);
    }
}

fn walk_with_clause<'a>(
    reader: &'a NodeReader<'a>,
    with: WithClause<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let is_recursive = with.recursive();

    if let Some(cte_list) = with.ctes() {
        for cte in cte_list.iter() {
            let cte_name = cte.cte_name();

            if is_recursive && !cte_name.is_empty() {
                scope.add_table(cte_name, None);
            }

            scope.push();
            if let Some(select) = cte.select() {
                walk_select(reader, select, scope, functions, config, diagnostics);
            }
            scope.pop();

            if !cte_name.is_empty() {
                scope.add_table(cte_name, None);
            }
        }
    }

    if let Some(select) = with.select() {
        walk_select(reader, select, scope, functions, config, diagnostics);
    }
}

fn walk_select<'a>(
    reader: &'a NodeReader<'a>,
    select: Select<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match select {
        Select::SelectStmt(s) => {
            walk_select_stmt(reader, s, scope, functions, config, diagnostics)
        }
        Select::CompoundSelect(c) => {
            walk_compound_select(reader, c, scope, functions, config, diagnostics)
        }
        Select::WithClause(w) => {
            walk_with_clause(reader, w, scope, functions, config, diagnostics)
        }
        Select::ValuesClause(v) => {
            if let Some(rows) = v.rows() {
                for row in rows.iter() {
                    walk_expr_list(reader, row, scope, functions, config, diagnostics);
                }
            }
        }
        Select::Other(_) => {}
    }
}

fn walk_insert_stmt<'a>(
    reader: &'a NodeReader<'a>,
    insert: InsertStmt<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(table_ref) = insert.table() {
        check_and_add_table_ref(reader, &table_ref, scope, config, diagnostics);
    }
    if let Some(source) = insert.source() {
        walk_select(reader, source, scope, functions, config, diagnostics);
    }
}

fn walk_update_stmt<'a>(
    reader: &'a NodeReader<'a>,
    update: UpdateStmt<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(table_ref) = update.table() {
        check_and_add_table_ref(reader, &table_ref, scope, config, diagnostics);
    }
    if let Some(from) = update.from_clause() {
        walk_table_source(reader, from, scope, functions, config, diagnostics);
    }
    if let Some(setlist) = update.setlist() {
        for clause in setlist.iter() {
            walk_opt_expr(reader, clause.value(), scope, functions, config, diagnostics);
        }
    }
    walk_opt_expr(reader, update.where_clause(), scope, functions, config, diagnostics);
}

fn walk_delete_stmt<'a>(
    reader: &'a NodeReader<'a>,
    delete: DeleteStmt<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(table_ref) = delete.table() {
        check_and_add_table_ref(reader, &table_ref, scope, config, diagnostics);
    }
    walk_opt_expr(reader, delete.where_clause(), scope, functions, config, diagnostics);
}

fn walk_table_source<'a>(
    reader: &'a NodeReader<'a>,
    source: TableSource<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match source {
        TableSource::TableRef(t) => {
            check_and_add_table_ref(reader, &t, scope, config, diagnostics);
        }
        TableSource::SubqueryTableSource(sub) => {
            scope.push();
            if let Some(select) = sub.select() {
                walk_select(reader, select, scope, functions, config, diagnostics);
            }
            scope.pop();
            let alias = sub.alias();
            if !alias.is_empty() {
                scope.add_table(alias, None);
            }
        }
        TableSource::JoinClause(join) => {
            if let Some(left) = join.left() {
                walk_table_source(reader, left, scope, functions, config, diagnostics);
            }
            if let Some(right) = join.right() {
                walk_table_source(reader, right, scope, functions, config, diagnostics);
            }
            walk_opt_expr(reader, join.on_expr(), scope, functions, config, diagnostics);
        }
        TableSource::JoinPrefix(jp) => {
            if let Some(src) = jp.source() {
                walk_table_source(reader, src, scope, functions, config, diagnostics);
            }
        }
        TableSource::Other(_) => {}
    }
}

fn check_and_add_table_ref<'a>(
    reader: &'a NodeReader<'a>,
    table_ref: &TableRef<'a>,
    scope: &mut ScopeStack,
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let name = table_ref.table_name();
    if name.is_empty() {
        return;
    }

    let source = reader.source();
    let offset = name.as_ptr() as usize - source.as_ptr() as usize;
    if let Some(diag) = check_table_ref(name, offset, name.len(), scope, config) {
        diagnostics.push(diag);
    }

    let alias = table_ref.alias();
    let scope_name = if alias.is_empty() { name } else { alias };
    scope.add_table(scope_name, None);
}

fn walk_expr<'a>(
    reader: &'a NodeReader<'a>,
    expr: Expr<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expr {
        Expr::ColumnRef(col) => {
            let column = col.column();
            if column.is_empty() {
                return;
            }
            let table = col.table();
            let source = reader.source();
            let offset = column.as_ptr() as usize - source.as_ptr() as usize;
            let table_opt = if table.is_empty() { None } else { Some(table) };

            if let Some(diag) =
                check_column_ref(table_opt, column, offset, column.len(), scope, config)
            {
                diagnostics.push(diag);
            }
        }
        Expr::FunctionCall(f) => {
            walk_function(reader, f.func_name(), f.args(), f.filter_clause(), scope, functions, config, diagnostics);
        }
        Expr::AggregateFunctionCall(f) => {
            walk_function(reader, f.func_name(), f.args(), f.filter_clause(), scope, functions, config, diagnostics);
        }
        Expr::OrderedSetFunctionCall(f) => {
            walk_function(reader, f.func_name(), f.args(), f.filter_clause(), scope, functions, config, diagnostics);
        }
        Expr::BinaryExpr(bin) => {
            walk_opt_expr(reader, bin.left(), scope, functions, config, diagnostics);
            walk_opt_expr(reader, bin.right(), scope, functions, config, diagnostics);
        }
        Expr::UnaryExpr(un) => {
            walk_opt_expr(reader, un.operand(), scope, functions, config, diagnostics);
        }
        Expr::IsExpr(is) => {
            walk_opt_expr(reader, is.left(), scope, functions, config, diagnostics);
            walk_opt_expr(reader, is.right(), scope, functions, config, diagnostics);
        }
        Expr::BetweenExpr(b) => {
            walk_opt_expr(reader, b.operand(), scope, functions, config, diagnostics);
            walk_opt_expr(reader, b.low(), scope, functions, config, diagnostics);
            walk_opt_expr(reader, b.high(), scope, functions, config, diagnostics);
        }
        Expr::InExpr(in_expr) => {
            walk_opt_expr(reader, in_expr.operand(), scope, functions, config, diagnostics);
            if let Some(source) = in_expr.source() {
                match source {
                    InExprSource::ExprList(list) => {
                        walk_expr_list(reader, list, scope, functions, config, diagnostics);
                    }
                    InExprSource::SubqueryExpr(sub) => {
                        walk_subquery(reader, sub.select(), scope, functions, config, diagnostics);
                    }
                    InExprSource::Other(_) => {}
                }
            }
        }
        Expr::CaseExpr(case) => {
            walk_opt_expr(reader, case.operand(), scope, functions, config, diagnostics);
            if let Some(whens) = case.whens() {
                for when in whens.iter() {
                    walk_opt_expr(reader, when.when_expr(), scope, functions, config, diagnostics);
                    walk_opt_expr(reader, when.then_expr(), scope, functions, config, diagnostics);
                }
            }
            walk_opt_expr(reader, case.else_expr(), scope, functions, config, diagnostics);
        }
        Expr::SubqueryExpr(sub) => {
            walk_subquery(reader, sub.select(), scope, functions, config, diagnostics);
        }
        Expr::ExistsExpr(exists) => {
            walk_subquery(reader, exists.select(), scope, functions, config, diagnostics);
        }
        Expr::CastExpr(cast) => {
            walk_opt_expr(reader, cast.expr(), scope, functions, config, diagnostics);
        }
        Expr::CollateExpr(collate) => {
            walk_opt_expr(reader, collate.expr(), scope, functions, config, diagnostics);
        }
        Expr::LikeExpr(like) => {
            walk_opt_expr(reader, like.operand(), scope, functions, config, diagnostics);
            walk_opt_expr(reader, like.pattern(), scope, functions, config, diagnostics);
            walk_opt_expr(reader, like.escape(), scope, functions, config, diagnostics);
        }
        Expr::Literal(_) | Expr::Variable(_) | Expr::RaiseExpr(_) | Expr::Other(_) => {}
    }
}

/// Walk an optional expression.
fn walk_opt_expr<'a>(
    reader: &'a NodeReader<'a>,
    expr: Option<Expr<'a>>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(e) = expr {
        walk_expr(reader, e, scope, functions, config, diagnostics);
    }
}

/// Walk a subquery (push/pop scope around it).
fn walk_subquery<'a>(
    reader: &'a NodeReader<'a>,
    select: Option<Select<'a>>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if let Some(select) = select {
        scope.push();
        walk_select(reader, select, scope, functions, config, diagnostics);
        scope.pop();
    }
}

/// Check a function name + arity and recurse into args/filter.
fn walk_function<'a>(
    reader: &'a NodeReader<'a>,
    name: &str,
    args: Option<ExprList<'a>>,
    filter: Option<Expr<'a>>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    if !name.is_empty() {
        let source = reader.source();
        let offset = name.as_ptr() as usize - source.as_ptr() as usize;
        let arg_count = args.as_ref().map_or(0, |a| a.len());
        if let Some(diag) =
            check_function_call(name, arg_count, offset, name.len(), functions, config)
        {
            diagnostics.push(diag);
        }
    }
    if let Some(args) = args {
        walk_expr_list(reader, args, scope, functions, config, diagnostics);
    }
    walk_opt_expr(reader, filter, scope, functions, config, diagnostics);
}

fn walk_expr_list<'a>(
    reader: &'a NodeReader<'a>,
    list: ExprList<'a>,
    scope: &mut ScopeStack,
    functions: &[FunctionDef],
    config: &ValidationConfig,
    diagnostics: &mut Vec<Diagnostic>,
) {
    for node in list.iter() {
        if let Some(expr) = node_to_expr(node) {
            walk_expr(reader, expr, scope, functions, config, diagnostics);
        }
    }
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
