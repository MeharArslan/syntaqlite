use syntaqlite_fmt::{render, DocArena, DocId, FormatConfig, NIL_DOC};
use syntaqlite_parser::*;

/// Build a Doc tree from a parsed AST node using the typed NodeRef API.
fn format_node<'a>(
    session: &Session<'a>,
    node_id: u32,
    arena: &mut DocArena<'a>,
) -> DocId {
    if node_id == NULL_NODE {
        return NIL_DOC;
    }
    let node_ref = match session.node(node_id) {
        Some(n) => n,
        None => return NIL_DOC,
    };
    let source = session.source();

    match node_ref.tag() {
        NodeTag::Literal => {
            let lit = node_ref.as_literal().unwrap();
            arena.text(lit.source.as_str(source))
        }
        NodeTag::ColumnRef => {
            let cr = node_ref.as_column_ref().unwrap();
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
        NodeTag::TableRef => {
            let tr = node_ref.as_table_ref().unwrap();
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
        NodeTag::Variable => {
            let v = node_ref.as_variable().unwrap();
            arena.text(v.source.as_str(source))
        }
        NodeTag::BinaryExpr => {
            let expr = node_ref.as_binary_expr().unwrap();
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
        NodeTag::UnaryExpr => {
            let expr = node_ref.as_unary_expr().unwrap();
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
        NodeTag::ResultColumn => {
            let rc = node_ref.as_result_column().unwrap();
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
        NodeTag::SelectStmt => format_select(session, &node_ref, arena),
        // List nodes: comma-separated with soft breaks
        NodeTag::ResultColumnList | NodeTag::ExprList => {
            let list = node_ref.as_list().unwrap();
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
    node_ref: &NodeRef<'a>,
    arena: &mut DocArena<'a>,
) -> DocId {
    let stmt = node_ref.as_select_stmt().unwrap();

    // SELECT <columns>
    let select_kw = arena.keyword("SELECT");
    let sp = arena.line();
    let cols = format_node(session, stmt.columns, arena);
    let cols_body = arena.cat(sp, cols);
    let cols_nested = arena.nest(4, cols_body);
    let mut parts = vec![select_kw, cols_nested];

    // FROM <table>
    if stmt.from_clause != NULL_NODE {
        let line = arena.line();
        let from_kw = arena.keyword("FROM");
        let sp = arena.line();
        let from_body = format_node(session, stmt.from_clause, arena);
        let from_inner = arena.cat(sp, from_body);
        let from_nested = arena.nest(4, from_inner);
        parts.extend([line, from_kw, from_nested]);
    }

    // WHERE <expr>
    if stmt.where_clause != NULL_NODE {
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
