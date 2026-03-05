// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Three-level catalog system for semantic analysis.
//!
//! Lookup priority: document → database → static. Identical pattern for
//! both functions and relations.
//!
//! - [`StaticCatalog`] — built from dialect data at construction. Immutable.
//! - [`DatabaseCatalog`] — provided by the caller. The user's live database.
//! - [`DocumentCatalog`] — accumulated from DDL during analysis. Internal scratch.
//! - [`CatalogStack`] — flat composition that holds references to all three.

use syntaqlite_syntax::any::FieldKind;
use syntaqlite_syntax::any::{AnyNodeId, AnyParsedStatement, FieldValue};
use syntaqlite_syntax::typed::GrammarNodeType;

use crate::dialect::Dialect;

use super::functions::{FunctionCatalog, FunctionCheckResult, FunctionDef};
use super::relations::{ColumnDef, RelationDef, RelationKind};

// ── DatabaseCatalog (public, caller-provided) ────────────────────────

/// What exists in the user's database. Symmetric — both relations and functions.
///
/// Callers populate it however they want: introspecting a live DB,
/// parsing CREATE statements, loading from a config file, etc.
#[derive(Default)]
pub(crate) struct DatabaseCatalog {
    /// Relations visible to semantic analysis (tables and views).
    pub relations: Vec<RelationDef>,
    /// User-defined or database-defined functions visible to analysis.
    pub functions: Vec<FunctionDef>,
}

impl DatabaseCatalog {
    /// Iterate table relations only.
    pub(crate) fn tables(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.relations
            .iter()
            .filter(|r| r.kind == RelationKind::Table)
    }

    /// Iterate view relations only.
    pub(crate) fn views(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.relations
            .iter()
            .filter(|r| r.kind == RelationKind::View)
    }

    /// Build a `DatabaseCatalog` from a DDL source string.
    ///
    /// Creates a temporary parser, parses the source, and builds the schema
    /// from the resulting DDL statements.
    #[cfg(feature = "sqlite")]
    pub(crate) fn from_ddl(dialect: Dialect, source: &str) -> Self {
        let parser = syntaqlite_syntax::Parser::new();
        let mut session = parser.parse(source);
        let mut doc = DocumentCatalog::new();

        while let Some(stmt) = session.next() {
            let stmt = match stmt {
                Ok(stmt) => stmt,
                Err(_) => continue,
            };

            if let Some(root) = stmt.root() {
                let root_id: AnyNodeId = root.node_id().into();
                let any_result = stmt.erase();
                doc.accumulate(any_result, root_id, dialect, None);
            }
        }

        DatabaseCatalog {
            relations: doc.relations,
            functions: doc.functions,
        }
    }

    /// Build a `DatabaseCatalog` from a JSON string.
    #[cfg(feature = "json")]
    pub(crate) fn from_json(s: &str) -> Result<Self, String> {
        #[derive(serde::Deserialize)]
        struct Root {
            #[serde(default)]
            tables: Vec<TableInput>,
            #[serde(default)]
            views: Vec<TableInput>,
            #[serde(default)]
            functions: Vec<FunctionInput>,
        }
        #[derive(serde::Deserialize)]
        struct TableInput {
            name: String,
            #[serde(default)]
            columns: Vec<String>,
        }
        #[derive(serde::Deserialize)]
        struct FunctionInput {
            name: String,
            args: Option<usize>,
        }

        let root: Root =
            serde_json::from_str(s).map_err(|e| format!("invalid database catalog JSON: {e}"))?;

        let make_columns = |cols: Vec<String>| -> Vec<ColumnDef> {
            cols.into_iter()
                .map(|c| ColumnDef {
                    name: c,
                    type_name: None,
                    is_primary_key: false,
                    is_nullable: true,
                })
                .collect()
        };

        let relations = root
            .tables
            .into_iter()
            .map(|t| RelationDef {
                name: t.name,
                columns: make_columns(t.columns),
                kind: RelationKind::Table,
            })
            .chain(root.views.into_iter().map(|v| RelationDef {
                name: v.name,
                columns: make_columns(v.columns),
                kind: RelationKind::View,
            }))
            .collect();

        Ok(DatabaseCatalog {
            relations,
            functions: root
                .functions
                .into_iter()
                .map(|f| FunctionDef {
                    name: f.name,
                    args: f.args,
                })
                .collect(),
        })
    }
}

// ── StaticCatalog (internal, built from dialect) ─────────────────────

/// Dialect-builtin functions and relations. Built once at analyzer construction.
pub(crate) struct StaticCatalog {
    pub(crate) functions: FunctionCatalog,
    pub(crate) relations: Vec<RelationDef>,
}

impl StaticCatalog {
    pub(crate) fn for_dialect(dialect: &Dialect) -> Self {
        StaticCatalog {
            functions: FunctionCatalog::for_dialect(dialect),
            relations: Vec::new(),
        }
    }

    pub(crate) fn find_relation(&self, name: &str) -> Option<&RelationDef> {
        self.relations
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
    }
}

// ── DocumentCatalog (internal scratch buffer) ────────────────────────

/// Schema accumulated from DDL statements during analysis.
pub(crate) struct DocumentCatalog {
    pub(crate) relations: Vec<RelationDef>,
    pub(crate) functions: Vec<FunctionDef>,
    known: KnownSchema,
}

impl DocumentCatalog {
    pub(crate) fn new() -> Self {
        DocumentCatalog {
            relations: Vec::new(),
            functions: Vec::new(),
            known: std::collections::HashMap::new(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.relations.clear();
        self.functions.clear();
        self.known.clear();
    }

    pub(crate) fn find_relation(&self, name: &str) -> Option<&RelationDef> {
        self.relations
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
    }

    pub(crate) fn find_function(&self, name: &str) -> Option<&FunctionDef> {
        self.functions
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
    }

    /// Process one DDL statement and update the document schema.
    pub(crate) fn accumulate(
        &mut self,
        stmt_result: AnyParsedStatement<'_>,
        stmt_id: AnyNodeId,
        dialect: Dialect,
        database: Option<&DatabaseCatalog>,
    ) {
        use crate::dialect::schema::SchemaKind;

        let Some((tag, fields)) = stmt_result.extract_fields(stmt_id) else {
            return;
        };

        let Some(contrib) = dialect.schema_contribution_for_tag(tag) else {
            return;
        };

        // Extract the name span.
        let name_str = match fields[contrib.name_field as usize] {
            FieldValue::Span(s) if !s.is_empty() => s,
            _ => return,
        };
        let name = name_str.to_string();

        match contrib.kind {
            SchemaKind::Table | SchemaKind::View => {
                let kind = if contrib.kind == SchemaKind::Table {
                    RelationKind::Table
                } else {
                    RelationKind::View
                };
                let mut columns = Vec::new();
                let mut has_columns = false;

                if let Some(col_field_idx) = contrib.columns_field
                    && let FieldValue::NodeId(col_list_id) = fields[col_field_idx as usize]
                    && !col_list_id.is_null()
                {
                    has_columns = true;
                    columns_from_column_list(stmt_result, col_list_id, dialect, &mut columns);
                }

                if !has_columns
                    && let Some(sel_field_idx) = contrib.select_field
                    && let FieldValue::NodeId(sel_id) = fields[sel_field_idx as usize]
                    && !sel_id.is_null()
                {
                    columns_from_select(stmt_result, sel_id, &self.known, database, &mut columns);
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
                    let FieldValue::NodeId(args_id) = fields[args_idx as usize] else {
                        return None;
                    };
                    if args_id.is_null() {
                        return None;
                    }
                    let children = stmt_result.list_children(args_id)?;
                    Some(children.len())
                });
                self.functions.push(FunctionDef { name, args });
            }
            SchemaKind::Import => {}
        }
    }
}

/// Extract column definitions from a column definition list node.
fn columns_from_column_list(
    stmt_result: AnyParsedStatement<'_>,
    list_id: AnyNodeId,
    dialect: Dialect,
    out: &mut Vec<ColumnDef>,
) {
    let Some(children) = stmt_result.list_children(list_id) else {
        return;
    };

    for &child_id in children {
        if child_id.is_null() {
            continue;
        }
        let Some((child_tag, child_fields)) = stmt_result.extract_fields(child_id) else {
            continue;
        };

        let mut col_name: Option<String> = None;
        let mut type_name: Option<String> = None;
        let mut constraints_id: Option<AnyNodeId> = None;

        for (i, meta) in dialect.field_meta(child_tag).enumerate() {
            match (meta.kind(), meta.name()) {
                (FieldKind::Span, "column_name") => {
                    if let FieldValue::Span(s) = child_fields[i]
                        && !s.is_empty()
                    {
                        col_name = Some(s.to_string());
                    }
                }
                (FieldKind::Span, "type_name") => {
                    if let FieldValue::Span(s) = child_fields[i]
                        && !s.is_empty()
                    {
                        type_name = Some(s.to_string());
                    }
                }
                (FieldKind::NodeId, "constraints") => {
                    if let FieldValue::NodeId(id) = child_fields[i] {
                        constraints_id = Some(id);
                    }
                }
                _ => {}
            }
        }

        let Some(name) = col_name else { continue };

        let mut is_primary_key = false;
        let mut is_nullable = true;
        if let Some(constraints_id) = constraints_id.filter(|id| !id.is_null()) {
            extract_column_constraints(
                stmt_result,
                constraints_id,
                dialect,
                &mut is_primary_key,
                &mut is_nullable,
            );
        }

        out.push(ColumnDef {
            name,
            type_name,
            is_primary_key,
            is_nullable,
        });
    }
}

/// Walk a constraint list to detect PRIMARY KEY and NOT NULL constraints.
fn extract_column_constraints(
    stmt_result: AnyParsedStatement<'_>,
    list_id: AnyNodeId,
    dialect: Dialect,
    is_primary_key: &mut bool,
    is_nullable: &mut bool,
) {
    let Some(children) = stmt_result.list_children(list_id) else {
        return;
    };

    for &constraint_id in children {
        if constraint_id.is_null() {
            continue;
        }
        let Some((ctag, cfields)) = stmt_result.extract_fields(constraint_id) else {
            continue;
        };

        for (i, meta) in dialect.field_meta(ctag).enumerate() {
            if meta.kind() == FieldKind::Enum && meta.name() == "kind" {
                #[cfg(feature = "sqlite")]
                if let FieldValue::Enum(ordinal) = cfields[i] {
                    use syntaqlite_syntax::nodes::ColumnConstraintType;
                    if ordinal == ColumnConstraintType::NotNull as u32 {
                        *is_nullable = false;
                    } else if ordinal == ColumnConstraintType::PrimaryKey as u32 {
                        *is_primary_key = true;
                        *is_nullable = false;
                    }
                }
                break;
            }
        }
    }
}

/// Known schema for select resolution — maps lowercase table/view name to columns.
type KnownSchema = std::collections::HashMap<String, Vec<ColumnDef>>;

/// Best-effort column extraction from a SELECT, expanding `*` and `t.*`.
#[cfg(feature = "sqlite")]
fn columns_from_select(
    stmt_result: AnyParsedStatement<'_>,
    select_id: AnyNodeId,
    known: &KnownSchema,
    database: Option<&DatabaseCatalog>,
    out: &mut Vec<ColumnDef>,
) {
    use syntaqlite_syntax::any::AnyNodeId;
    use syntaqlite_syntax::nodes::{Expr, Select};

    let Some(select) = Select::from_result(stmt_result, select_id) else {
        return;
    };

    let stmt = match select {
        Select::SelectStmt(s) => s,
        Select::CompoundSelect(cs) => {
            if let Some(left) = cs.left() {
                let id: AnyNodeId = left.node_id().into();
                return columns_from_select(stmt_result, id, known, database, out);
            }
            return;
        }
        Select::WithClause(wc) => {
            if let Some(s) = wc.select() {
                let id: AnyNodeId = s.node_id().into();
                return columns_from_select(stmt_result, id, known, database, out);
            }
            return;
        }
        _ => return,
    };

    let from_sources = stmt
        .from_clause()
        .map(|ts| {
            let id: AnyNodeId = ts.node_id().into();
            collect_from_sources(stmt_result, id, known, database)
        })
        .unwrap_or_default();

    let Some(cols) = stmt.columns() else { return };
    for rc in cols.iter() {
        if rc.flags().star() {
            let qualifier = rc.alias();
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

#[cfg(not(feature = "sqlite"))]
fn columns_from_select(
    _stmt_result: AnyParsedStatement<'_>,
    _select_id: AnyNodeId,
    _known: &KnownSchema,
    _database: Option<&DatabaseCatalog>,
    _out: &mut Vec<ColumnDef>,
) {
}

/// A resolved FROM source: qualifier for `t.*` matching + pre-resolved columns.
struct FromSource {
    qualifier: String,
    columns: Vec<ColumnDef>,
}

/// Walk a `TableSource` tree, resolving each leaf's columns eagerly.
#[cfg(feature = "sqlite")]
fn collect_from_sources(
    stmt_result: AnyParsedStatement<'_>,
    source_id: AnyNodeId,
    known: &KnownSchema,
    database: Option<&DatabaseCatalog>,
) -> Vec<FromSource> {
    use syntaqlite_syntax::any::AnyNodeId;
    use syntaqlite_syntax::nodes::TableSource;

    let mut out = Vec::new();

    let Some(source) = TableSource::from_result(stmt_result, source_id) else {
        return out;
    };

    match source {
        TableSource::TableRef(tr) => {
            let name = tr.table_name();
            let alias = tr.alias();
            let qualifier = if alias.is_empty() { name } else { alias };
            let columns = known
                .get(&name.to_ascii_lowercase())
                .cloned()
                .unwrap_or_else(|| {
                    database
                        .and_then(|db| {
                            db.relations
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
                let id: AnyNodeId = select.node_id().into();
                columns_from_select(stmt_result, id, known, database, &mut columns);
            }
            out.push(FromSource {
                qualifier: sq.alias().to_string(),
                columns,
            });
        }
        TableSource::JoinClause(jc) => {
            if let Some(left) = jc.left() {
                let id: AnyNodeId = left.node_id().into();
                out.extend(collect_from_sources(stmt_result, id, known, database));
            }
            if let Some(right) = jc.right() {
                let id: AnyNodeId = right.node_id().into();
                out.extend(collect_from_sources(stmt_result, id, known, database));
            }
        }
        TableSource::JoinPrefix(jp) => {
            if let Some(s) = jp.source() {
                let id: AnyNodeId = s.node_id().into();
                out.extend(collect_from_sources(stmt_result, id, known, database));
            }
        }
        TableSource::Other(_) => {}
    }
    out
}

#[cfg(not(feature = "sqlite"))]
fn collect_from_sources(
    _stmt_result: AnyParsedStatement<'_>,
    _source_id: AnyNodeId,
    _known: &KnownSchema,
    _database: Option<&DatabaseCatalog>,
) -> Vec<FromSource> {
    Vec::new()
}

/// Expand `*` or `qualifier.*` using pre-resolved FROM sources.
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

// ── CatalogStack (internal, composed lookup) ─────────────────────────

/// Flat composition of all three catalog levels.
pub(crate) struct CatalogStack<'a> {
    pub(crate) static_: &'a StaticCatalog,
    pub(crate) database: &'a DatabaseCatalog,
    pub(crate) document: &'a DocumentCatalog,
}

impl CatalogStack<'_> {
    pub(crate) fn check_function(&self, name: &str, arg_count: usize) -> FunctionCheckResult {
        if let Some(func) = self.document.find_function(name) {
            return check_defined_function(func, arg_count);
        }

        if let Some(func) = self
            .database
            .functions
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
        {
            return check_defined_function(func, arg_count);
        }

        self.static_.functions.check_call(name, arg_count)
    }

    pub(crate) fn resolve_relation(&self, name: &str) -> bool {
        self.document.find_relation(name).is_some()
            || self
                .database
                .relations
                .iter()
                .any(|r| r.name.eq_ignore_ascii_case(name))
            || self.static_.find_relation(name).is_some()
    }

    pub(crate) fn columns_for(&self, name: &str) -> Option<Vec<String>> {
        if let Some(r) = self.document.find_relation(name) {
            return Some(r.columns.iter().map(|c| c.name.clone()).collect());
        }
        if let Some(r) = self
            .database
            .relations
            .iter()
            .find(|r| r.name.eq_ignore_ascii_case(name))
        {
            return Some(r.columns.iter().map(|c| c.name.clone()).collect());
        }
        if let Some(r) = self.static_.find_relation(name) {
            return Some(r.columns.iter().map(|c| c.name.clone()).collect());
        }
        None
    }

    pub(crate) fn all_relation_names(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        self.document
            .relations
            .iter()
            .chain(self.database.relations.iter())
            .chain(self.static_.relations.iter())
            .filter_map(|r| push_unique_lower(&mut seen, &r.name))
            .collect()
    }

    pub(crate) fn all_column_names(&self, table: Option<&str>) -> Vec<String> {
        let mut names = Vec::new();
        let all_relations = self
            .document
            .relations
            .iter()
            .chain(self.database.relations.iter())
            .chain(self.static_.relations.iter());
        for r in all_relations {
            if table.is_none_or(|tbl| r.name.eq_ignore_ascii_case(tbl)) {
                names.extend(r.columns.iter().map(|c| c.name.to_ascii_lowercase()));
            }
        }
        names.sort_unstable();
        names.dedup();
        names
    }

    pub(crate) fn all_function_names(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut names: Vec<String> = self
            .document
            .functions
            .iter()
            .chain(self.database.functions.iter())
            .filter_map(|f| push_unique_name(&mut seen, &f.name))
            .collect();

        for name in self.static_.functions.all_names() {
            if seen.insert(name.to_ascii_lowercase()) {
                names.push(name);
            }
        }

        names
    }
}

fn check_defined_function(func: &FunctionDef, arg_count: usize) -> FunctionCheckResult {
    match func.args {
        None => FunctionCheckResult::Ok,
        Some(n) if n == arg_count => FunctionCheckResult::Ok,
        Some(n) => FunctionCheckResult::WrongArity { expected: vec![n] },
    }
}

fn push_unique_lower(seen: &mut std::collections::HashSet<String>, name: &str) -> Option<String> {
    let lower = name.to_ascii_lowercase();
    if seen.insert(lower.clone()) {
        Some(lower)
    } else {
        None
    }
}

fn push_unique_name(seen: &mut std::collections::HashSet<String>, name: &str) -> Option<String> {
    if seen.insert(name.to_ascii_lowercase()) {
        Some(name.to_string())
    } else {
        None
    }
}
#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use super::*;

    #[test]
    fn from_ddl_creates_database_catalog() {
        let dialect = crate::dialect::sqlite();
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);";
        let catalog = DatabaseCatalog::from_ddl(dialect, sql);

        let tables: Vec<_> = catalog.tables().collect();
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

        assert_eq!(catalog.views().count(), 0);
        assert!(catalog.functions.is_empty());
    }

    #[test]
    fn from_ddl_star_expands_from_earlier_table() {
        let dialect = crate::dialect::sqlite();
        let sql = "\
            CREATE TABLE slice (order_id INTEGER, status TEXT);\n\
            CREATE TABLE orders AS SELECT * FROM slice;\n";
        let catalog = DatabaseCatalog::from_ddl(dialect, sql);

        let tables: Vec<_> = catalog.tables().collect();
        assert_eq!(tables.len(), 2);
        let orders = tables[1];
        assert_eq!(orders.name, "orders");
        assert_eq!(orders.columns.len(), 2);
        assert_eq!(orders.columns[0].name, "order_id");
        assert_eq!(orders.columns[1].name, "status");
    }

    #[test]
    fn catalog_stack_resolves_document_first() {
        let static_ = StaticCatalog::for_dialect(&crate::dialect::sqlite());
        let database = DatabaseCatalog {
            relations: vec![RelationDef {
                name: "users".to_string(),
                columns: vec![ColumnDef {
                    name: "id".to_string(),
                    type_name: None,
                    is_primary_key: false,
                    is_nullable: true,
                }],
                kind: RelationKind::Table,
            }],
            functions: Vec::new(),
        };
        let mut document = DocumentCatalog::new();
        document.relations.push(RelationDef {
            name: "users".to_string(),
            columns: vec![ColumnDef {
                name: "name".to_string(),
                type_name: None,
                is_primary_key: false,
                is_nullable: true,
            }],
            kind: RelationKind::Table,
        });

        let stack = CatalogStack {
            static_: &static_,
            database: &database,
            document: &document,
        };

        assert!(stack.resolve_relation("users"));
        let cols = stack.columns_for("users").unwrap();
        assert_eq!(cols, vec!["name"]); // document wins
    }
}
