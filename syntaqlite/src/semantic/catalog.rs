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

use syntaqlite_parser::RawDialect;

use super::functions::{FunctionCatalog, FunctionCheckResult, FunctionDef};
use super::relations::{ColumnDef, RelationDef, RelationKind};

// ── DatabaseCatalog (public, caller-provided) ────────────────────────

/// What exists in the user's database. Symmetric — both relations and functions.
///
/// Callers populate it however they want: introspecting a live DB,
/// parsing CREATE statements, loading from a config file, etc.
///
/// This replaces the old `SessionContext`.
#[derive(Default)]
pub struct DatabaseCatalog {
    pub relations: Vec<RelationDef>,
    pub functions: Vec<FunctionDef>,
}

impl DatabaseCatalog {
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

    /// Build a `DatabaseCatalog` from parsed DDL statement roots.
    ///
    /// Processes statements in order, building up a schema incrementally.
    /// `SELECT *` and `SELECT t.*` in `CREATE TABLE … AS SELECT` or
    /// `CREATE VIEW … AS SELECT` are expanded using tables/views defined
    /// by earlier statements in the same input.
    pub fn from_stmts<'a>(
        reader: syntaqlite_parser::RawNodeReader<'a>,
        stmt_ids: &[syntaqlite_parser::NodeId],
        dialect: RawDialect<'_>,
    ) -> Self {
        let mut doc = DocumentCatalog::new();
        for &id in stmt_ids {
            doc.accumulate(reader, id, dialect, None);
        }
        DatabaseCatalog {
            relations: doc.relations,
            functions: doc.functions,
        }
    }

    /// Build a `DatabaseCatalog` from a DDL source string.
    ///
    /// Creates a temporary parser, parses the source, and builds the schema
    /// from the resulting DDL statements. This is a convenience wrapper for
    /// cases like WASM where you have raw DDL text.
    pub fn from_ddl(
        dialect: RawDialect<'_>,
        source: &str,
        dialect_config: Option<syntaqlite_parser::DialectConfig>,
    ) -> Self {
        let mut parser = syntaqlite_parser::RawParser::with_config(
            dialect,
            &syntaqlite_parser::ParserConfig {
                dialect_config,
                ..syntaqlite_parser::ParserConfig::default()
            },
        );
        let mut cursor = parser.parse(source);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            if let Ok(node_ref) = result {
                stmt_ids.push(node_ref.id());
            }
        }

        Self::from_stmts(cursor.reader(), &stmt_ids, dialect)
    }

    /// Build a `DatabaseCatalog` from a JSON string.
    ///
    /// The JSON format is:
    /// ```json
    /// {
    ///   "tables": [{"name": "t", "columns": ["id", "name"]}],
    ///   "views":  [{"name": "v", "columns": ["id"]}],
    ///   "functions": [{"name": "my_func", "args": 2}]
    /// }
    /// ```
    /// All top-level keys are optional and default to empty.
    /// Column entries are bare strings; function `args` is `null` for variadic.
    #[cfg(feature = "json")]
    pub fn from_json(s: &str) -> Result<Self, String> {
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

/// Dialect-builtin functions and relations. Built once at
/// [`SemanticAnalyzer::new()`](super::SemanticAnalyzer::new), immutable thereafter.
pub(crate) struct StaticCatalog {
    pub(crate) functions: FunctionCatalog,
    pub(crate) relations: Vec<RelationDef>,
}

impl StaticCatalog {
    /// Build from a dialect and explicit configuration.
    pub(crate) fn for_dialect(
        dialect: &RawDialect<'_>,
        config: &syntaqlite_parser::DialectConfig,
    ) -> Self {
        StaticCatalog {
            functions: FunctionCatalog::for_dialect(dialect, config),
            relations: Vec::new(), // TODO: add static relations via C FFI
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
///
/// Rebuilt each analysis pass. Owned by the analyzer as a reusable
/// scratch buffer (cleared, not reallocated).
///
/// This replaces the old `DocumentContext`.
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
}

/// Read a `NodeId` field from a raw node pointer at the given metadata offset.
///
/// # Safety
/// `ptr` must point to a valid node struct; `meta.offset` must be a valid
/// offset to a `u32` (NodeId) field within that struct.
unsafe fn read_node_id(
    ptr: *const u8,
    meta: &syntaqlite_parser::FieldMeta,
) -> syntaqlite_parser::NodeId {
    unsafe { syntaqlite_parser::NodeId(*(ptr.add(meta.offset as usize) as *const u32)) }
}

/// Read a `SourceSpan` field from a raw node pointer, returning its text
/// (or `""` if the span is empty).
///
/// # Safety
/// `ptr` must point to a valid node struct; `meta.offset` must be a valid
/// offset to a `SourceSpan` field within that struct.
unsafe fn read_span<'a>(
    ptr: *const u8,
    meta: &syntaqlite_parser::FieldMeta,
    source: &'a str,
) -> &'a str {
    unsafe {
        let span = &*(ptr.add(meta.offset as usize) as *const syntaqlite_parser::SourceSpan);
        if span.is_empty() {
            ""
        } else {
            span.as_str(source)
        }
    }
}

impl DocumentCatalog {
    /// Process one DDL statement and update the document schema.
    ///
    /// Uses the dialect's schema contribution metadata to determine which
    /// node types define tables/views/functions, and which fields hold the
    /// name, column list, and AS SELECT clause. This works for any dialect
    /// that declares `schema { ... }` annotations in its `.synq` files.
    ///
    /// `database` is consulted for `*` expansion so that
    /// `CREATE TABLE t AS SELECT * FROM db_table` resolves correctly when
    /// `db_table` lives in the database (live DB) context.
    pub(crate) fn accumulate(
        &mut self,
        reader: syntaqlite_parser::RawNodeReader<'_>,
        stmt_id: syntaqlite_parser::NodeId,
        dialect: RawDialect<'_>,
        database: Option<&DatabaseCatalog>,
    ) {
        use syntaqlite_parser::DialectNodeType;
        use syntaqlite_parser::SchemaKind;
        use syntaqlite_parser::{FIELD_NODE_ID, FIELD_SPAN};

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
        let name_str = unsafe { read_span(ptr, name_meta, source) };
        if name_str.is_empty() {
            return;
        }
        let name = name_str.to_string();

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
                    let col_list_id = unsafe { read_node_id(ptr, col_meta) };
                    if !col_list_id.is_null() {
                        has_columns = true;
                        columns_from_column_list(&reader, col_list_id, &dialect, &mut columns);
                    }
                }

                // Fall back to AS SELECT for column inference.
                if !has_columns && let Some(sel_field_idx) = contrib.select_field {
                    let sel_meta = &meta[sel_field_idx as usize];
                    debug_assert_eq!(sel_meta.kind, FIELD_NODE_ID);
                    // SAFETY: ptr is a valid arena pointer; sel_meta.offset is from
                    // codegen metadata, and kind == FIELD_NODE_ID (debug-asserted above).
                    let sel_id = unsafe { read_node_id(ptr, sel_meta) };
                    if !sel_id.is_null()
                        && let Some(select) =
                            syntaqlite_parser_sqlite::ast::Select::from_arena(reader, sel_id)
                    {
                        columns_from_select(&select, &self.known, database, &mut columns);
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
                    let args_id = unsafe { read_node_id(ptr, args_meta) };
                    if args_id.is_null() {
                        return None;
                    }
                    // Count children of the args list node.
                    let list = reader.resolve_list(args_id)?;
                    Some(list.children().len())
                });
                self.functions.push(FunctionDef { name, args });
            }
            SchemaKind::Import => {
                // Future: resolve from DatabaseCatalog.modules
            }
        }
    }
}

/// Extract column definitions from a column definition list node.
fn columns_from_column_list(
    reader: &syntaqlite_parser::RawNodeReader<'_>,
    list_id: syntaqlite_parser::NodeId,
    dialect: &RawDialect<'_>,
    out: &mut Vec<ColumnDef>,
) {
    use syntaqlite_parser::NodeId;
    use syntaqlite_parser::{FIELD_NODE_ID, FIELD_SPAN};

    let Some(list) = reader.resolve_list(list_id) else {
        return;
    };
    let source = reader.source();

    for &child_id in list.children() {
        if child_id.is_null() {
            continue;
        }
        let Some((child_ptr, child_tag)) = reader.node_ptr(child_id) else {
            continue;
        };
        let child_meta = dialect.field_meta(child_tag);

        let mut col_name = None;
        let mut type_name = None;
        let mut constraints_id = NodeId::NULL;

        for fm in child_meta {
            // SAFETY: fm is from dialect.field_meta() which returns static
            // codegen data; the name pointer is valid for 'd.
            let field_name = unsafe { fm.name_str() };
            match (fm.kind, field_name) {
                (FIELD_SPAN, "column_name") => {
                    // SAFETY: child_ptr is valid; fm.offset is from codegen metadata.
                    let s = unsafe { read_span(child_ptr, fm, source) };
                    if !s.is_empty() {
                        col_name = Some(s.to_string());
                    }
                }
                (FIELD_SPAN, "type_name") => {
                    // SAFETY: child_ptr is valid; fm.offset is from codegen metadata.
                    let s = unsafe { read_span(child_ptr, fm, source) };
                    if !s.is_empty() {
                        type_name = Some(s.to_string());
                    }
                }
                (FIELD_NODE_ID, "constraints") => {
                    // SAFETY: child_ptr is valid; fm.offset is from codegen metadata.
                    constraints_id = unsafe { read_node_id(child_ptr, fm) };
                }
                _ => {}
            }
        }

        let Some(name) = col_name else { continue };

        // Walk constraints to find PRIMARY KEY and NOT NULL.
        let mut is_primary_key = false;
        let mut is_nullable = true;
        if !constraints_id.is_null() {
            extract_column_constraints(
                reader,
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
    reader: &syntaqlite_parser::RawNodeReader<'_>,
    list_id: syntaqlite_parser::NodeId,
    dialect: &RawDialect<'_>,
    is_primary_key: &mut bool,
    is_nullable: &mut bool,
) {
    use syntaqlite_parser::FIELD_ENUM;

    let Some(list) = reader.resolve_list(list_id) else {
        return;
    };

    for &constraint_id in list.children() {
        if constraint_id.is_null() {
            continue;
        }
        let Some((cptr, ctag)) = reader.node_ptr(constraint_id) else {
            continue;
        };
        let meta = dialect.field_meta(ctag);
        for fm in meta {
            if fm.kind == FIELD_ENUM {
                let field_name = unsafe { fm.name_str() };
                if field_name == "kind" {
                    let ordinal = unsafe { *(cptr.add(fm.offset as usize) as *const u32) };
                    if let Some(display) = unsafe { fm.display_name(ordinal as usize) } {
                        match display {
                            "NOT_NULL" => *is_nullable = false,
                            "PRIMARY_KEY" => {
                                *is_primary_key = true;
                                *is_nullable = false;
                            }
                            _ => {}
                        }
                    }
                    break;
                }
            }
        }
    }
}

/// Known schema passed through select resolution — maps lowercase table/view
/// name to its columns.
type KnownSchema = std::collections::HashMap<String, Vec<ColumnDef>>;

/// Best-effort column extraction from a SELECT, expanding `*` and `t.*`
/// against previously defined tables/views.
fn columns_from_select(
    select: &syntaqlite_parser_sqlite::ast::Select<'_>,
    known: &KnownSchema,
    database: Option<&DatabaseCatalog>,
    out: &mut Vec<ColumnDef>,
) {
    use syntaqlite_parser_sqlite::ast::{Expr, Select};

    let stmt = match select {
        Select::SelectStmt(s) => s,
        Select::CompoundSelect(cs) => {
            if let Some(s) = cs.left() {
                return columns_from_select(&s, known, database, out);
            }
            return;
        }
        Select::WithClause(wc) => {
            if let Some(s) = wc.select() {
                return columns_from_select(&s, known, database, out);
            }
            return;
        }
        _ => return,
    };

    // Collect FROM sources so we can expand `*`.
    let from_sources = stmt
        .from_clause()
        .map(|ts| collect_from_sources(&ts, known, database))
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
struct FromSource {
    qualifier: String,
    columns: Vec<ColumnDef>,
}

/// Walk a `TableSource` tree, resolving each leaf's columns eagerly.
fn collect_from_sources(
    source: &syntaqlite_parser_sqlite::ast::TableSource<'_>,
    known: &KnownSchema,
    database: Option<&DatabaseCatalog>,
) -> Vec<FromSource> {
    use syntaqlite_parser_sqlite::ast::TableSource;

    let mut out = Vec::new();
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
                columns_from_select(&select, known, database, &mut columns);
            }
            out.push(FromSource {
                qualifier: sq.alias().to_string(),
                columns,
            });
        }
        TableSource::JoinClause(jc) => {
            if let Some(left) = jc.left() {
                out.extend(collect_from_sources(&left, known, database));
            }
            if let Some(right) = jc.right() {
                out.extend(collect_from_sources(&right, known, database));
            }
        }
        TableSource::JoinPrefix(jp) => {
            if let Some(s) = jp.source() {
                out.extend(collect_from_sources(&s, known, database));
            }
        }
        _ => {}
    }
    out
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

/// Flat composition of all three catalog levels. One struct holds references
/// to all three levels; lookup methods check document → database → static.
pub(crate) struct CatalogStack<'a> {
    pub(crate) static_: &'a StaticCatalog,
    pub(crate) database: &'a DatabaseCatalog,
    pub(crate) document: &'a DocumentCatalog,
}

impl CatalogStack<'_> {
    /// Case-insensitive function check: document → database → static.
    pub(crate) fn check_function(&self, name: &str, arg_count: usize) -> FunctionCheckResult {
        // Document-defined functions first.
        if let Some(func) = self.document.find_function(name) {
            return match func.args {
                None => FunctionCheckResult::Ok,
                Some(n) if n == arg_count => FunctionCheckResult::Ok,
                Some(n) => FunctionCheckResult::WrongArity { expected: vec![n] },
            };
        }

        // Database functions.
        if let Some(func) = self
            .database
            .functions
            .iter()
            .find(|f| f.name.eq_ignore_ascii_case(name))
        {
            return match func.args {
                None => FunctionCheckResult::Ok,
                Some(n) if n == arg_count => FunctionCheckResult::Ok,
                Some(n) => FunctionCheckResult::WrongArity { expected: vec![n] },
            };
        }

        // Static (dialect builtins + extensions). FunctionCatalog already
        // handles arity encoding and session functions.
        self.static_.functions.check_call(name, arg_count)
    }

    /// Case-insensitive relation resolution: document → database → static.
    pub(crate) fn resolve_relation(&self, name: &str) -> bool {
        self.document.find_relation(name).is_some()
            || self
                .database
                .relations
                .iter()
                .any(|r| r.name.eq_ignore_ascii_case(name))
            || self.static_.find_relation(name).is_some()
    }

    /// Look up columns for a relation by name (document → database → static).
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

    /// All known relation names across all three levels (for fuzzy matching).
    pub(crate) fn all_relation_names(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut names = Vec::new();
        for r in &self.document.relations {
            let lower = r.name.to_ascii_lowercase();
            if seen.insert(lower.clone()) {
                names.push(lower);
            }
        }
        for r in &self.database.relations {
            let lower = r.name.to_ascii_lowercase();
            if seen.insert(lower.clone()) {
                names.push(lower);
            }
        }
        for r in &self.static_.relations {
            let lower = r.name.to_ascii_lowercase();
            if seen.insert(lower.clone()) {
                names.push(lower);
            }
        }
        names
    }

    /// All known column names, optionally filtered by table (for fuzzy matching).
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

    /// All known function names across all three levels (for fuzzy matching).
    pub(crate) fn all_function_names(&self) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        let mut names = Vec::new();
        for f in &self.document.functions {
            if seen.insert(f.name.to_ascii_lowercase()) {
                names.push(f.name.clone());
            }
        }
        for f in &self.database.functions {
            if seen.insert(f.name.to_ascii_lowercase()) {
                names.push(f.name.clone());
            }
        }
        // Static function names from the FunctionCatalog.
        for name in self.static_.functions.all_names() {
            if seen.insert(name.to_ascii_lowercase()) {
                names.push(name);
            }
        }
        names
    }
}

#[cfg(test)]
#[cfg(feature = "sqlite")]
mod tests {
    use super::*;

    #[test]
    fn from_stmts_creates_database_catalog() {
        let dialect = crate::dialect::sqlite();
        let mut parser = syntaqlite_parser::RawParser::new(dialect);
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let catalog = DatabaseCatalog::from_stmts(cursor.reader(), &stmt_ids, dialect);

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
    fn from_stmts_star_expands_from_earlier_table() {
        let dialect = crate::dialect::sqlite();
        let mut parser = syntaqlite_parser::RawParser::new(dialect);
        let sql = "\
            CREATE TABLE slice (order_id INTEGER, status TEXT);\n\
            CREATE TABLE orders AS SELECT * FROM slice;\n";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let catalog = DatabaseCatalog::from_stmts(cursor.reader(), &stmt_ids, dialect);

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
        let static_ = StaticCatalog::for_dialect(
            &crate::dialect::sqlite(),
            &syntaqlite_parser::DialectConfig::default(),
        );
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
