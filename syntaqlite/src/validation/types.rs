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

/// Whether a [`RelationDef`] represents a base table or a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationKind {
    Table,
    View,
}

/// A table or view in the schema.
#[derive(Clone)]
pub struct RelationDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub kind: RelationKind,
}

#[derive(Clone)]
pub struct ColumnDef {
    pub name: String,
    /// SQLite is flexible with types.
    pub type_name: Option<String>,
    pub is_primary_key: bool,
    pub is_nullable: bool,
}

#[derive(Clone)]
pub struct FunctionDef {
    pub name: String,
    /// None = variadic.
    pub args: Option<usize>,
    pub description: Option<String>,
}

/// Database schema context for analysis.
///
/// Callers populate it however they want: introspecting a live DB,
/// parsing CREATE statements, loading from a config file, etc.
pub struct SessionContext {
    pub relations: Vec<RelationDef>,
    pub functions: Vec<FunctionDef>,
}

impl SessionContext {
    pub fn tables(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.relations
            .iter()
            .filter(|r| r.kind == RelationKind::Table)
    }

    pub fn views(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.relations
            .iter()
            .filter(|r| r.kind == RelationKind::View)
    }
}

#[cfg(feature = "sqlite")]
impl SessionContext {
    /// Build a `SessionContext` from parsed DDL statement roots.
    ///
    /// Processes statements in order, building up a schema incrementally.
    /// `SELECT *` and `SELECT t.*` in `CREATE TABLE … AS SELECT` or
    /// `CREATE VIEW … AS SELECT` are expanded using tables/views defined
    /// by earlier statements in the same input.
    pub fn from_stmts<'a>(
        reader: &'a crate::parser::NodeReader<'a>,
        stmt_ids: &[crate::parser::NodeId],
        dialect: crate::Dialect<'_>,
    ) -> Self {
        let mut doc = DocumentContext::new();
        for &id in stmt_ids {
            doc.accumulate(reader, id, dialect, None);
        }
        SessionContext {
            relations: doc.relations,
            functions: doc.functions,
        }
    }
}

/// Schema accumulated from DDL statements earlier in the document being validated.
///
/// Built incrementally (one statement at a time) so forward references are rejected.
/// Pass to [`validate_statement`] alongside the session context so each statement only
/// sees tables/views that were *defined before it* in the document.
pub struct DocumentContext {
    pub relations: Vec<RelationDef>,
    pub functions: Vec<FunctionDef>,
    #[cfg(feature = "sqlite")]
    known: KnownSchema,
}

impl DocumentContext {
    pub fn new() -> Self {
        DocumentContext {
            relations: vec![],
            functions: vec![],
            #[cfg(feature = "sqlite")]
            known: std::collections::HashMap::new(),
        }
    }

    pub fn tables(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.relations
            .iter()
            .filter(|r| r.kind == RelationKind::Table)
    }

    pub fn views(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.relations
            .iter()
            .filter(|r| r.kind == RelationKind::View)
    }
}

impl Default for DocumentContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "sqlite")]
impl DocumentContext {
    /// Process one DDL statement and update the document schema.
    ///
    /// Uses the dialect's schema contribution metadata to determine which
    /// node types define tables/views/functions, and which fields hold the
    /// name, column list, and AS SELECT clause. This works for any dialect
    /// that declares `schema { ... }` annotations in its `.synq` files.
    ///
    /// `session` is consulted for `*` expansion so that
    /// `CREATE TABLE t AS SELECT * FROM db_table` resolves correctly when
    /// `db_table` lives in the session (live DB) context.
    pub fn accumulate(
        &mut self,
        reader: &crate::parser::NodeReader<'_>,
        stmt_id: crate::parser::NodeId,
        dialect: crate::Dialect<'_>,
        session: Option<&SessionContext>,
    ) {
        use crate::dialect::SchemaKind;
        use crate::dialect::ffi::{FIELD_NODE_ID, FIELD_SPAN};
        use crate::parser::{FromArena, NodeId, SourceSpan};

        let Some((ptr, tag)) = reader.node_ptr(stmt_id) else {
            return;
        };

        let Some(contrib) = dialect.schema_contribution_for_tag(tag) else {
            return;
        };

        let meta = dialect.field_meta(tag);
        let source = reader.source();

        // Extract the name span.
        let name_meta = &meta[contrib.name_field as usize];
        debug_assert_eq!(name_meta.kind, FIELD_SPAN);
        // SAFETY: ptr is a valid arena pointer from node_ptr(); name_meta.offset
        // is from codegen metadata, and kind == FIELD_SPAN (debug-asserted above).
        let name = unsafe {
            let span = &*(ptr.add(name_meta.offset as usize) as *const SourceSpan);
            if span.is_empty() {
                return;
            }
            span.as_str(source).to_string()
        };

        match contrib.kind {
            SchemaKind::Table | SchemaKind::View => {
                let kind = if contrib.kind == SchemaKind::Table {
                    RelationKind::Table
                } else {
                    RelationKind::View
                };
                let mut columns = Vec::new();

                // Try explicit column list first (e.g., ColumnDefList).
                let mut has_columns = false;
                if let Some(col_field_idx) = contrib.columns_field {
                    let col_meta = &meta[col_field_idx as usize];
                    debug_assert_eq!(col_meta.kind, FIELD_NODE_ID);
                    // SAFETY: ptr is a valid arena pointer; col_meta.offset is from
                    // codegen metadata, and kind == FIELD_NODE_ID (debug-asserted above).
                    let col_list_id =
                        unsafe { NodeId(*(ptr.add(col_meta.offset as usize) as *const u32)) };
                    if !col_list_id.is_null() {
                        has_columns = true;
                        columns_from_column_list(reader, col_list_id, dialect, &mut columns);
                    }
                }

                // Fall back to AS SELECT for column inference.
                if !has_columns {
                    if let Some(sel_field_idx) = contrib.select_field {
                        let sel_meta = &meta[sel_field_idx as usize];
                        debug_assert_eq!(sel_meta.kind, FIELD_NODE_ID);
                        // SAFETY: ptr is a valid arena pointer; sel_meta.offset is from
                        // codegen metadata, and kind == FIELD_NODE_ID (debug-asserted above).
                        let sel_id =
                            unsafe { NodeId(*(ptr.add(sel_meta.offset as usize) as *const u32)) };
                        if !sel_id.is_null() {
                            if let Some(select) =
                                crate::sqlite::ast::Select::from_arena(reader, sel_id)
                            {
                                columns_from_select(&select, &self.known, session, &mut columns);
                            }
                        }
                    }
                }

                self.known
                    .insert(name.to_ascii_lowercase(), columns.clone());
                self.relations.push(RelationDef {
                    name,
                    columns,
                    kind,
                });
            }
            SchemaKind::Function => {
                let args = contrib.args_field.and_then(|args_idx| {
                    let args_meta = &meta[args_idx as usize];
                    debug_assert_eq!(args_meta.kind, FIELD_NODE_ID);
                    // SAFETY: ptr is a valid arena pointer; args_meta.offset is from
                    // codegen metadata, and kind == FIELD_NODE_ID (debug-asserted above).
                    let args_id =
                        unsafe { NodeId(*(ptr.add(args_meta.offset as usize) as *const u32)) };
                    if args_id.is_null() {
                        return None;
                    }
                    // Count children of the args list node.
                    let (args_ptr, args_tag) = reader.node_ptr(args_id)?;
                    if !dialect.is_list(args_tag) {
                        return None;
                    }
                    // SAFETY: args_ptr is a valid arena pointer and is_list(args_tag)
                    // confirmed it has NodeList layout (tag, count, children[count]).
                    let list = unsafe { &*(args_ptr as *const crate::parser::NodeList) };
                    Some(list.children().len())
                });
                self.functions.push(FunctionDef {
                    name,
                    args,
                    description: None,
                });
            }
            SchemaKind::Import => {
                // Future: resolve from SessionContext.modules
            }
        }
    }
}

/// Extract column names from a column definition list node.
///
/// Walks list children, looking for a `column_name` span field in each child
/// node's field metadata. This is generic over any column-definition-like
/// node that has a field named `column_name`.
#[cfg(feature = "sqlite")]
fn columns_from_column_list(
    reader: &crate::parser::NodeReader<'_>,
    list_id: crate::parser::NodeId,
    dialect: crate::Dialect<'_>,
    out: &mut Vec<ColumnDef>,
) {
    use crate::dialect::ffi::FIELD_SPAN;
    use crate::parser::SourceSpan;

    let Some((list_ptr, list_tag)) = reader.node_ptr(list_id) else {
        return;
    };
    if !dialect.is_list(list_tag) {
        return;
    }
    // SAFETY: list_ptr is a valid arena pointer and is_list(list_tag) confirmed
    // it has NodeList layout (tag, count, children[count]).
    let list = unsafe { &*(list_ptr as *const crate::parser::NodeList) };
    let source = reader.source();

    for &child_id in list.children() {
        if child_id.is_null() {
            continue;
        }
        let Some((child_ptr, child_tag)) = reader.node_ptr(child_id) else {
            continue;
        };
        let child_meta = dialect.field_meta(child_tag);

        // Find the first SPAN field named "column_name".
        for fm in child_meta {
            if fm.kind == FIELD_SPAN {
                let field_name = fm.name_str();
                if field_name == "column_name" {
                    // SAFETY: child_ptr is a valid arena pointer from node_ptr();
                    // fm.offset is from codegen metadata, and kind == FIELD_SPAN.
                    let span =
                        unsafe { &*(child_ptr.add(fm.offset as usize) as *const SourceSpan) };
                    if !span.is_empty() {
                        out.push(ColumnDef {
                            name: span.as_str(source).to_string(),
                            type_name: None,
                            is_primary_key: false,
                            is_nullable: true,
                        });
                    }
                    break;
                }
            }
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
    session: Option<&SessionContext>,
    out: &mut Vec<ColumnDef>,
) {
    use crate::sqlite::ast::{Expr, Select};

    let stmt = match select {
        Select::SelectStmt(s) => s,
        Select::CompoundSelect(cs) => {
            if let Some(s) = cs.left() {
                return columns_from_select(&s, known, session, out);
            }
            return;
        }
        Select::WithClause(wc) => {
            if let Some(s) = wc.select() {
                return columns_from_select(&s, known, session, out);
            }
            return;
        }
        _ => return,
    };

    // Collect FROM sources so we can expand `*`.
    let from_sources = stmt
        .from_clause()
        .map(|ts| collect_from_sources(&ts, known, session))
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
    session: Option<&SessionContext>,
) -> Vec<FromSource> {
    use crate::sqlite::ast::TableSource;

    let mut out = Vec::new();
    match source {
        TableSource::TableRef(tr) => {
            let name = tr.table_name();
            let alias = tr.alias();
            let qualifier = if alias.is_empty() { name } else { alias };
            // Look up in doc-so-far first, then fall back to session.
            let columns = known
                .get(&name.to_ascii_lowercase())
                .cloned()
                .unwrap_or_else(|| {
                    session
                        .and_then(|s| {
                            s.relations
                                .iter()
                                .find(|r| r.name.eq_ignore_ascii_case(name))
                                .map(|r| r.columns.clone())
                        })
                        .unwrap_or_default()
                });
            out.push(FromSource {
                qualifier: qualifier.to_string(),
                columns,
            });
        }
        TableSource::SubqueryTableSource(sq) => {
            let mut columns = Vec::new();
            if let Some(select) = sq.select() {
                columns_from_select(&select, known, session, &mut columns);
            }
            out.push(FromSource {
                qualifier: sq.alias().to_string(),
                columns,
            });
        }
        TableSource::JoinClause(jc) => {
            if let Some(left) = jc.left() {
                out.extend(collect_from_sources(&left, known, session));
            }
            if let Some(right) = jc.right() {
                out.extend(collect_from_sources(&right, known, session));
            }
        }
        TableSource::JoinPrefix(jp) => {
            if let Some(s) = jp.source() {
                out.extend(collect_from_sources(&s, known, session));
            }
        }
        _ => {}
    }
    out
}

/// Expand `*` or `qualifier.*` using pre-resolved FROM sources.
#[cfg(feature = "sqlite")]
fn expand_star(from_sources: &[FromSource], qualifier: &str, out: &mut Vec<ColumnDef>) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_stmts_creates_session_context() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let tables: Vec<_> = ctx.tables().collect();
        assert_eq!(tables.len(), 1);
        let table = tables[0];
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

        assert_eq!(ctx.views().count(), 0);
        assert!(ctx.functions.is_empty());
    }

    #[test]
    fn from_stmts_create_table_as_select() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "CREATE TABLE orders AS SELECT order_id, total AS amount FROM src;";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let tables: Vec<_> = ctx.tables().collect();
        assert_eq!(tables.len(), 1);
        let table = tables[0];
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

        let stmt_ids: Vec<_> = (&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let tables: Vec<_> = ctx.tables().collect();
        assert_eq!(tables.len(), 2);
        let orders = tables[1];
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

        let stmt_ids: Vec<_> = (&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let tables: Vec<_> = ctx.tables().collect();
        let c = tables[2];
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

        let stmt_ids: Vec<_> = (&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let tables: Vec<_> = ctx.tables().collect();
        let dst = tables[1];
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

        let stmt_ids: Vec<_> = (&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let tables: Vec<_> = ctx.tables().collect();
        assert_eq!(tables.len(), 2);
        let orders = tables[1];
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

        let stmt_ids: Vec<_> = (&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        assert_eq!(ctx.tables().count(), 0);
        let views: Vec<_> = ctx.views().collect();
        assert_eq!(views.len(), 1);
        assert_eq!(views[0].name, "active_users");
        assert_eq!(views[0].columns.len(), 2);
        assert_eq!(views[0].columns[0].name, "id");
        assert_eq!(views[0].columns[1].name, "name");
    }

    #[test]
    fn from_stmts_view_star_expands_from_table() {
        let dialect = crate::sqlite::low_level::dialect();
        let mut parser = crate::Parser::with_dialect(&dialect);
        let sql = "\
            CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);\n\
            CREATE VIEW all_users AS SELECT * FROM users;\n";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids);

        let views: Vec<_> = ctx.views().collect();
        assert_eq!(views.len(), 1);
        let view = views[0];
        assert_eq!(view.name, "all_users");
        assert_eq!(view.columns.len(), 2);
        assert_eq!(view.columns[0].name, "id");
        assert_eq!(view.columns[1].name, "name");
    }
}
