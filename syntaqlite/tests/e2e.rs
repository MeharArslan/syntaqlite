use syntaqlite::ast::*;
use syntaqlite::{NodeId, Parser, Session};
use syntaqlite_runtime::fmt::{render, DocArena, DocId, FormatConfig, NIL_DOC};

/// Build a Doc tree from a parsed AST node using the typed Node API.
fn format_node<'a>(
    session: &Session<'a>,
    node_id: NodeId,
    arena: &mut DocArena<'a>,
) -> DocId {
    if node_id.is_null() {
        return NIL_DOC;
    }
    let node = match session.node(node_id) {
        Some(n) => n,
        None => return NIL_DOC,
    };
    let source = session.source();

    match node {
        Node::Literal(lit) => {
            arena.text(lit.source.as_str(source))
        }
        Node::ColumnRef(cr) => {
            let mut parts = Vec::new();
            if !cr.schema.is_empty() {
                parts.push(arena.text(cr.schema.as_str(source)));
                parts.push(arena.text("."));
            }
            if !cr.table.is_empty() {
                parts.push(arena.text(cr.table.as_str(source)));
                parts.push(arena.text("."));
            }
            parts.push(arena.text(cr.column.as_str(source)));
            arena.cats(&parts)
        }
        Node::TableRef(tr) => {
            let mut parts = Vec::new();
            if !tr.schema.is_empty() {
                parts.push(arena.text(tr.schema.as_str(source)));
                parts.push(arena.text("."));
            }
            parts.push(arena.text(tr.table_name.as_str(source)));
            if !tr.alias.is_empty() {
                parts.push(arena.text(" "));
                parts.push(arena.keyword("AS"));
                parts.push(arena.text(" "));
                parts.push(arena.text(tr.alias.as_str(source)));
            }
            arena.cats(&parts)
        }
        Node::Variable(v) => {
            arena.text(v.source.as_str(source))
        }
        Node::BinaryExpr(expr) => {
            let left = format_node(session, expr.left, arena);
            let op_str = binary_op_str(expr.op);
            let op_doc = if matches!(expr.op, BinaryOp::And | BinaryOp::Or) {
                arena.keyword(op_str)
            } else {
                arena.text(op_str)
            };
            let right = format_node(session, expr.right, arena);
            let sp1 = arena.text(" ");
            let sp2 = arena.text(" ");
            arena.cats(&[left, sp1, op_doc, sp2, right])
        }
        Node::UnaryExpr(expr) => {
            let op_str = match expr.op {
                UnaryOp::Minus => "-",
                UnaryOp::Plus => "+",
                UnaryOp::Bitnot => "~",
                UnaryOp::Not => "NOT ",
            };
            let op_doc = arena.text(op_str);
            let operand = format_node(session, expr.operand, arena);
            arena.cat(op_doc, operand)
        }
        Node::ResultColumn(rc) => {
            let mut doc = if rc.flags.star() {
                arena.text("*")
            } else {
                format_node(session, rc.expr, arena)
            };
            let alias = rc.alias.as_str(source);
            if !alias.is_empty() {
                let sp = arena.text(" ");
                let as_kw = arena.keyword("AS");
                let sp2 = arena.text(" ");
                let alias_doc = arena.text(alias);
                doc = arena.cats(&[doc, sp, as_kw, sp2, alias_doc]);
            }
            doc
        }
        Node::SelectStmt(stmt) => format_select(session, stmt, arena),
        // List nodes: comma-separated with soft breaks
        Node::ResultColumnList(list) | Node::ExprList(list) => {
            let children = list.children();
            let mut parts = Vec::new();
            for (i, &child_id) in children.iter().enumerate() {
                if i > 0 {
                    parts.push(arena.text(","));
                    parts.push(arena.line());
                }
                parts.push(format_node(session, child_id, arena));
            }
            arena.cats(&parts)
        }
        _ => arena.text("<unsupported>"),
    }
}

fn format_select<'a>(
    session: &Session<'a>,
    stmt: &SelectStmt,
    arena: &mut DocArena<'a>,
) -> DocId {
    let select_kw = arena.keyword("SELECT");
    let sp = arena.line();
    let cols = format_node(session, stmt.columns, arena);
    let cols_body = arena.cat(sp, cols);
    let cols_nested = arena.nest(4, cols_body);
    let mut parts = vec![select_kw, cols_nested];

    if !stmt.from_clause.is_null() {
        let line = arena.line();
        let from_kw = arena.keyword("FROM");
        let sp = arena.line();
        let from_body = format_node(session, stmt.from_clause, arena);
        let from_inner = arena.cat(sp, from_body);
        let from_nested = arena.nest(4, from_inner);
        parts.extend([line, from_kw, from_nested]);
    }

    if !stmt.where_clause.is_null() {
        let line = arena.line();
        let where_kw = arena.keyword("WHERE");
        let sp = arena.line();
        let where_body = format_node(session, stmt.where_clause, arena);
        let where_inner = arena.cat(sp, where_body);
        let where_nested = arena.nest(4, where_inner);
        parts.extend([line, where_kw, where_nested]);
    }

    let inner = arena.cats(&parts);
    arena.group(inner)
}

fn binary_op_str(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Plus => "+",
        BinaryOp::Minus => "-",
        BinaryOp::Star => "*",
        BinaryOp::Slash => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Lt => "<",
        BinaryOp::Gt => ">",
        BinaryOp::Le => "<=",
        BinaryOp::Ge => ">=",
        BinaryOp::Eq => "=",
        BinaryOp::Ne => "!=",
        BinaryOp::And => "AND",
        BinaryOp::Or => "OR",
        BinaryOp::Bitand => "&",
        BinaryOp::Bitor => "|",
        BinaryOp::Lshift => "<<",
        BinaryOp::Rshift => ">>",
        BinaryOp::Concat => "||",
        BinaryOp::Ptr => "->",
    }
}

// -- Test helpers --

fn format_sql(sql: &str) -> String {
    format_sql_with(sql, &FormatConfig::default())
}

fn format_sql_with(sql: &str, config: &FormatConfig) -> String {
    let mut parser = Parser::new();
    let mut session = parser.parse(sql);
    let root = session.next_statement().unwrap().unwrap();
    let mut arena = DocArena::new();
    let doc = format_node(&session, root, &mut arena);
    render(&arena, doc, config)
}

// -- Tests --

#[test]
fn format_literal() {
    assert_eq!(format_sql("SELECT 42"), "SELECT 42");
}

#[test]
fn format_column_ref() {
    assert_eq!(format_sql("SELECT a FROM t"), "SELECT a FROM t");
}

#[test]
fn format_binary_expr() {
    assert_eq!(format_sql("SELECT 1 + 2"), "SELECT 1 + 2");
}

#[test]
fn format_select_with_where() {
    assert_eq!(
        format_sql("SELECT a FROM t WHERE x = 1"),
        "SELECT a FROM t WHERE x = 1"
    );
}

#[test]
fn format_multiple_columns() {
    assert_eq!(
        format_sql("SELECT a, b, c FROM t"),
        "SELECT a, b, c FROM t"
    );
}

#[test]
fn format_long_select_breaks() {
    let config = FormatConfig {
        line_width: 20,
        ..Default::default()
    };
    let result =
        format_sql_with("SELECT column_one, column_two FROM very_long_table", &config);
    assert_eq!(
        result,
        "SELECT\n    column_one,\n    column_two\nFROM\n    very_long_table"
    );
}
