// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// A diagnostic message associated with a source range.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Byte offset of the start of the diagnostic range.
    pub start_offset: usize,
    /// Byte offset of the end of the diagnostic range.
    pub end_offset: usize,
    /// Human-readable diagnostic message.
    pub message: String,
    /// Severity level.
    pub severity: Severity,
}

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
    Hint,
}

/// Database schema context for analysis.
///
/// Callers populate it however they want: introspecting a live DB,
/// parsing CREATE statements, loading from a config file, etc.
pub struct SessionContext {
    pub tables: Vec<TableDef>,
    pub views: Vec<ViewDef>,
    pub functions: Vec<FunctionDef>,
}

#[cfg(feature = "sqlite")]
impl SessionContext {
    /// Build a `SessionContext` from parsed DDL statement roots.
    ///
    /// Processes statements in order, building up a schema incrementally.
    /// `SELECT *` and `SELECT t.*` in `CREATE TABLE … AS SELECT` or
    /// `CREATE VIEW … AS SELECT` are expanded using tables/views defined
    /// by earlier statements in the same input.
    pub fn from_stmts<'a>(reader: &'a crate::parser::NodeReader<'a>, stmt_ids: &[crate::parser::NodeId]) -> Self {
        use crate::parser::FromArena;
        use crate::sqlite::ast::{ColumnConstraintKind, Stmt};

        let mut tables = Vec::new();
        let mut views = Vec::new();
        // Accumulated schema for resolving `*` — maps lowercase name → columns.
        let mut known: std::collections::HashMap<String, Vec<ColumnDef>> =
            std::collections::HashMap::new();

        for &id in stmt_ids {
            let Some(stmt) = Stmt::from_arena(reader, id) else {
                continue;
            };
            match stmt {
                Stmt::CreateTableStmt(ct) => {
                    let table_name = ct.table_name().to_string();
                    let mut columns = Vec::new();

                    if let Some(col_list) = ct.columns() {
                        for col in col_list.iter() {
                            let name = col.column_name().to_string();
                            let raw_type = col.type_name();
                            let type_name = if raw_type.is_empty() {
                                None
                            } else {
                                Some(raw_type.to_string())
                            };

                            let mut is_primary_key = false;
                            let mut is_not_null = false;

                            if let Some(constraints) = col.constraints() {
                                for c in constraints.iter() {
                                    match c.kind() {
                                        ColumnConstraintKind::PrimaryKey => is_primary_key = true,
                                        ColumnConstraintKind::NotNull => is_not_null = true,
                                        _ => {}
                                    }
                                }
                            }

                            columns.push(ColumnDef {
                                name,
                                type_name,
                                is_primary_key,
                                is_nullable: !is_primary_key && !is_not_null,
                            });
                        }
                    } else if let Some(select) = ct.as_select() {
                        columns_from_select(&select, &known, &mut columns);
                    }

                    known.insert(table_name.to_ascii_lowercase(), columns.clone());
                    tables.push(TableDef {
                        name: table_name,
                        columns,
                    });
                }
                Stmt::CreateViewStmt(cv) => {
                    let view_name = cv.view_name().to_string();
                    let mut columns = Vec::new();
                    if let Some(select) = cv.select() {
                        columns_from_select(&select, &known, &mut columns);
                    }
                    known.insert(view_name.to_ascii_lowercase(), columns.clone());
                    views.push(ViewDef {
                        name: view_name,
                        columns,
                    });
                }
                _ => {}
            }
        }

        SessionContext {
            tables,
            views,
            functions: vec![],
        }
    }
}

/// Known schema passed through select resolution — maps lowercase table/view
/// name to its columns.
#[cfg(feature = "sqlite")]
type KnownSchema = std::collections::HashMap<String, Vec<ColumnDef>>;

/// Best-effort column extraction from a SELECT, expanding `*` and `t.*`
/// against previously defined tables/views.
#[cfg(feature = "sqlite")]
fn columns_from_select(
    select: &crate::sqlite::ast::Select<'_>,
    known: &KnownSchema,
    out: &mut Vec<ColumnDef>,
) {
    use crate::sqlite::ast::{Expr, Select};

    let stmt = match select {
        Select::SelectStmt(s) => s,
        Select::CompoundSelect(cs) => {
            if let Some(s) = cs.left() {
                return columns_from_select(&s, known, out);
            }
            return;
        }
        Select::WithClause(wc) => {
            if let Some(s) = wc.select() {
                return columns_from_select(&s, known, out);
            }
            return;
        }
        _ => return,
    };

    // Collect FROM sources so we can expand `*`.
    let from_sources = stmt
        .from_clause()
        .map(|ts| collect_from_sources(&ts, known))
        .unwrap_or_default();

    let Some(cols) = stmt.columns() else { return };
    for rc in cols.iter() {
        if rc.flags().star() {
            let qualifier = rc.alias(); // "t" for `SELECT t.*`, empty for `SELECT *`
            expand_star(&from_sources, qualifier, out);
            continue;
        }
        let alias = rc.alias();
        let name = if !alias.is_empty() {
            alias.to_string()
        } else if let Some(Expr::ColumnRef(cr)) = rc.expr() {
            cr.column().to_string()
        } else {
            continue;
        };
        out.push(ColumnDef {
            name,
            type_name: None,
            is_primary_key: false,
            is_nullable: true,
        });
    }
}

/// A resolved FROM source: qualifier for `t.*` matching + pre-resolved columns.
#[cfg(feature = "sqlite")]
struct FromSource {
    /// Alias if present, otherwise the table/view name. Used for `t.*` matching.
    qualifier: String,
    /// Columns resolved from the known schema (for table refs) or by
    /// recursively resolving the subquery (for subquery table sources).
    columns: Vec<ColumnDef>,
}

/// Walk a `TableSource` tree, resolving each leaf's columns eagerly.
#[cfg(feature = "sqlite")]
fn collect_from_sources(
    source: &crate::sqlite::ast::TableSource<'_>,
    known: &KnownSchema,
) -> Vec<FromSource> {
    use crate::sqlite::ast::TableSource;

    let mut out = Vec::new();
    match source {
        TableSource::TableRef(tr) => {
            let name = tr.table_name();
            let alias = tr.alias();
            let qualifier = if alias.is_empty() { name } else { alias };
            let columns = known
                .get(&name.to_ascii_lowercase())
                .cloned()
                .unwrap_or_default();
            out.push(FromSource {
                qualifier: qualifier.to_string(),
                columns,
            });
        }
        TableSource::SubqueryTableSource(sq) => {
            let mut columns = Vec::new();
            if let Some(select) = sq.select() {
                columns_from_select(&select, known, &mut columns);
            }
            out.push(FromSource {
                qualifier: sq.alias().to_string(),
                columns,
            });
        }
        TableSource::JoinClause(jc) => {
            if let Some(left) = jc.left() {
                out.extend(collect_from_sources(&left, known));
            }
            if let Some(right) = jc.right() {
                out.extend(collect_from_sources(&right, known));
            }
        }
        TableSource::JoinPrefix(jp) => {
            if let Some(s) = jp.source() {
                out.extend(collect_from_sources(&s, known));
            }
        }
        _ => {}
    }
    out
}

/// Expand `*` or `qualifier.*` using pre-resolved FROM sources.
#[cfg(feature = "sqlite")]
fn expand_star(
    from_sources: &[FromSource],
    qualifier: &str,
    out: &mut Vec<ColumnDef>,
) {
    for src in from_sources {
        if !qualifier.is_empty() && !src.qualifier.eq_ignore_ascii_case(qualifier) {
            continue;
        }
        out.extend(src.columns.iter().map(|c| ColumnDef {
            name: c.name.clone(),
            type_name: c.type_name.clone(),
            is_primary_key: false,
            is_nullable: true,
        }));
    }
}

/// Deprecated: renamed to [`SessionContext`].
pub type AmbientContext = SessionContext;

pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

#[derive(Clone)]
pub struct ColumnDef {
    pub name: String,
    /// SQLite is flexible with types.
    pub type_name: Option<String>,
    pub is_primary_key: bool,
    pub is_nullable: bool,
}

pub struct ViewDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

#[derive(Clone)]
pub struct FunctionDef {
    pub name: String,
    /// None = variadic.
    pub args: Option<usize>,
    pub description: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_stmts_creates_session_context() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);";
        let mut cursor = parser.parse(sql);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }

        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        assert_eq!(ctx.tables.len(), 1);
        let table = &ctx.tables[0];
        assert_eq!(table.name, "users");
        assert_eq!(table.columns.len(), 2);

        let id_col = &table.columns[0];
        assert_eq!(id_col.name, "id");
        assert_eq!(id_col.type_name.as_deref(), Some("INTEGER"));
        assert!(id_col.is_primary_key);
        assert!(!id_col.is_nullable);

        let name_col = &table.columns[1];
        assert_eq!(name_col.name, "name");
        assert_eq!(name_col.type_name.as_deref(), Some("TEXT"));
        assert!(!name_col.is_primary_key);
        assert!(!name_col.is_nullable);

        assert!(ctx.views.is_empty());
        assert!(ctx.functions.is_empty());
    }

    #[test]
    fn from_stmts_create_table_as_select() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "CREATE TABLE orders AS SELECT order_id, total AS amount FROM src;";
        let mut cursor = parser.parse(sql);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }

        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        assert_eq!(ctx.tables.len(), 1);
        let table = &ctx.tables[0];
        assert_eq!(table.name, "orders");
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.columns[0].name, "order_id");
        assert_eq!(table.columns[1].name, "amount"); // alias wins
    }

    #[test]
    fn from_stmts_star_expands_from_earlier_table() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "\
            CREATE TABLE slice (order_id INTEGER, status TEXT);\n\
            CREATE TABLE orders AS SELECT * FROM slice;\n";
        let mut cursor = parser.parse(sql);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }

        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        assert_eq!(ctx.tables.len(), 2);
        let orders = &ctx.tables[1];
        assert_eq!(orders.name, "orders");
        assert_eq!(orders.columns.len(), 2);
        assert_eq!(orders.columns[0].name, "order_id");
        assert_eq!(orders.columns[1].name, "status");
    }

    #[test]
    fn from_stmts_qualified_star_expands_correct_table() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "\
            CREATE TABLE a (x INTEGER);\n\
            CREATE TABLE b (y TEXT);\n\
            CREATE TABLE c AS SELECT a.* FROM a JOIN b ON 1;\n";
        let mut cursor = parser.parse(sql);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }

        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let c = &ctx.tables[2];
        assert_eq!(c.name, "c");
        assert_eq!(c.columns.len(), 1);
        assert_eq!(c.columns[0].name, "x");
    }

    #[test]
    fn from_stmts_star_with_alias() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "\
            CREATE TABLE src (id INTEGER, val TEXT);\n\
            CREATE TABLE dst AS SELECT t.* FROM src AS t;\n";
        let mut cursor = parser.parse(sql);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }

        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let dst = &ctx.tables[1];
        assert_eq!(dst.name, "dst");
        assert_eq!(dst.columns.len(), 2);
        assert_eq!(dst.columns[0].name, "id");
        assert_eq!(dst.columns[1].name, "val");
    }

    #[test]
    fn from_stmts_star_through_subquery() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "\
            CREATE TABLE slice (order_id INTEGER, customer_id TEXT);\n\
            CREATE TABLE orders AS SELECT * FROM (SELECT * FROM slice);\n";
        let mut cursor = parser.parse(sql);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }

        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        assert_eq!(ctx.tables.len(), 2);
        let orders = &ctx.tables[1];
        assert_eq!(orders.name, "orders");
        assert_eq!(orders.columns.len(), 2);
        assert_eq!(orders.columns[0].name, "order_id");
        assert_eq!(orders.columns[1].name, "customer_id");
    }

    #[test]
    fn from_stmts_handles_views() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "CREATE VIEW active_users AS SELECT id, name FROM users WHERE active = 1;";
        let mut cursor = parser.parse(sql);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }

        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        assert!(ctx.tables.is_empty());
        assert_eq!(ctx.views.len(), 1);
        assert_eq!(ctx.views[0].name, "active_users");
        assert_eq!(ctx.views[0].columns.len(), 2);
        assert_eq!(ctx.views[0].columns[0].name, "id");
        assert_eq!(ctx.views[0].columns[1].name, "name");
    }

    #[test]
    fn from_stmts_view_star_expands_from_table() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "\
            CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);\n\
            CREATE VIEW all_users AS SELECT * FROM users;\n";
        let mut cursor = parser.parse(sql);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            stmt_ids.push(result.expect("parse failed"));
        }

        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        assert_eq!(ctx.views.len(), 1);
        let view = &ctx.views[0];
        assert_eq!(view.name, "all_users");
        assert_eq!(view.columns.len(), 2);
        assert_eq!(view.columns[0].name, "id");
        assert_eq!(view.columns[1].name, "name");
    }
}
