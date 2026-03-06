// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Layered semantic catalog.
//!
//! Resolution order: query (innermost frame first) → document → connection → database → dialect.

use std::collections::{HashMap, HashSet};

use syntaqlite_syntax::any::{AnyNodeId, AnyParsedStatement, FieldValue, NodeFields};

use crate::dialect::Dialect;
use crate::dialect::{
    FunctionCategory as DialectFunctionCategory, SemanticRole, is_function_available,
};

// ── Core layer types ─────────────────────────────────────────────────────────

/// The category of a catalog function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
}

/// Describes how many arguments a function overload accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AritySpec {
    Exact(usize),
    AtLeast(usize),
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FunctionOverload {
    pub category: FunctionCategory,
    pub arity: AritySpec,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct FunctionSet {
    name: String,
    overloads: Vec<FunctionOverload>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct RelationEntry {
    name: String,
    /// `None` = table is known to exist but column list is not tracked.
    /// Column references against it are conservatively accepted.
    columns: Option<Vec<String>>,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
struct TableFunctionSet {
    name: String,
    overloads: Vec<FunctionOverload>,
    /// Empty = output columns unknown; suppress column errors.
    output_columns: Vec<String>,
}

// ── Resolution result types ───────────────────────────────────────────────────

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum ColumnResolution {
    /// Column found (or table has unknown columns — conservatively accepted).
    Found,
    /// Table is in scope but this column is not in its known list.
    TableFoundColumnMissing,
    /// The qualifier table is not in scope — table check already reported this.
    TableNotFound,
    /// Unqualified column not found in any table in scope.
    NotFound,
}

#[allow(dead_code)]
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FunctionCheckResult {
    Ok,
    Unknown,
    WrongArity { expected: Vec<usize> },
}

// ── CatalogLayerContents ──────────────────────────────────────────────────────

/// The data stored in a single catalog layer.
///
/// Callers obtain a mutable reference via [`Catalog::layer_mut`] and call
/// `insert_*` methods to populate it.
#[derive(Debug, Default, Clone)]
pub struct CatalogLayerContents {
    relations: HashMap<String, RelationEntry>,
    functions: HashMap<String, FunctionSet>,
    table_functions: HashMap<String, TableFunctionSet>,
}

#[allow(dead_code)]
impl CatalogLayerContents {
    /// Remove all entries from this layer.
    fn clear(&mut self) {
        self.relations = HashMap::default();
        self.functions = HashMap::default();
        self.table_functions = HashMap::default();
    }

    /// Insert a relation. `columns = None` means the table exists but its
    /// column list is not tracked (column refs against it are suppressed).
    pub fn insert_relation(
        &mut self,
        name: impl Into<String>,
        columns: Option<Vec<String>>,
    ) {
        let name = name.into();
        self.relations
            .insert(canonical_name(&name), RelationEntry { name, columns });
    }

    /// Insert a single function overload.
    pub fn insert_function_overload(
        &mut self,
        name: impl Into<String>,
        category: FunctionCategory,
        arity: AritySpec,
    ) {
        let name = name.into();
        let key = canonical_name(&name);
        self.functions
            .entry(key)
            .and_modify(|set| set.overloads.push(FunctionOverload { category, arity }))
            .or_insert_with(|| FunctionSet {
                name,
                overloads: vec![FunctionOverload { category, arity }],
            });
    }

    /// Insert multiple arities for a function (dialect codegen helper).
    pub(crate) fn insert_function_arities(
        &mut self,
        name: impl Into<String>,
        category: FunctionCategory,
        arities: &[i16],
    ) {
        let name = name.into();
        if arities.is_empty() {
            self.insert_function_overload(name, category, AritySpec::Any);
            return;
        }
        for &a in arities {
            let arity = match a.cmp(&-1) {
                std::cmp::Ordering::Equal => AritySpec::Any,
                std::cmp::Ordering::Less => AritySpec::AtLeast(
                    usize::try_from(-i32::from(a) - 1).expect("negative arity encodes minimum"),
                ),
                std::cmp::Ordering::Greater => AritySpec::Exact(
                    usize::try_from(i32::from(a)).expect("fixed arity must be non-negative"),
                ),
            };
            self.insert_function_overload(name.clone(), category, arity);
        }
    }

    /// Insert a table-valued function.
    pub fn insert_table_function(
        &mut self,
        name: impl Into<String>,
        arity: AritySpec,
        output_columns: Vec<String>,
    ) {
        let name = name.into();
        let key = canonical_name(&name);
        self.table_functions
            .entry(key)
            .and_modify(|set| {
                set.overloads.push(FunctionOverload {
                    category: FunctionCategory::Scalar,
                    arity,
                });
            })
            .or_insert_with(|| TableFunctionSet {
                name,
                overloads: vec![FunctionOverload {
                    category: FunctionCategory::Scalar,
                    arity,
                }],
                output_columns,
            });
    }

    fn relation(&self, name: &str) -> Option<&RelationEntry> {
        self.relations.get(&canonical_name(name))
    }

    fn function(&self, name: &str) -> Option<&FunctionSet> {
        self.functions.get(&canonical_name(name))
    }

    fn table_function(&self, name: &str) -> Option<&TableFunctionSet> {
        self.table_functions.get(&canonical_name(name))
    }
}

// ── CatalogLayer enum ─────────────────────────────────────────────────────────

/// Identifies a fixed layer in the [`Catalog`].
///
/// Use [`Catalog::layer`] / [`Catalog::layer_mut`] to access the corresponding
/// [`CatalogLayerContents`] directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CatalogLayer {
    /// Dialect built-ins — populated at construction, never mutated.
    Dialect,
    /// Persistent user schema (cross-connection): tables, views, functions.
    Database,
    /// Connection-scoped schema (attached databases, session overrides).
    Connection,
    /// DDL accumulated from the current source document — cleared each pass.
    Document,
}

impl CatalogLayer {
    fn index(self) -> usize {
        match self {
            Self::Dialect => LAYER_DIALECT,
            Self::Database => LAYER_DATABASE,
            Self::Connection => LAYER_CONNECTION,
            Self::Document => LAYER_DOCUMENT,
        }
    }
}

// ── Layer index constants ─────────────────────────────────────────────────────

const LAYER_DIALECT: usize = 0;
const LAYER_DATABASE: usize = 1;
const LAYER_CONNECTION: usize = 2;
const LAYER_DOCUMENT: usize = 3;
/// Number of fixed layers that are always present.
const FIXED_LAYER_COUNT: usize = 4;

// ── Public Catalog ────────────────────────────────────────────────────────────

/// Layered semantic catalog.
///
/// Layers are stored in a single `Vec` indexed by priority (lowest first):
///
/// ```text
/// index 0  CatalogLayer::Dialect    — dialect built-ins (never mutated)
/// index 1  CatalogLayer::Database   — persistent user schema
/// index 2  CatalogLayer::Connection — connection-scoped schema
/// index 3  CatalogLayer::Document   — DDL from the current source
/// index 4+ query scopes             — pushed/popped during AST traversal
/// ```
///
/// Resolution iterates layers from highest index to lowest, so the priority
/// order is: innermost query scope → document → connection → database → dialect.
///
/// The `Vec` is never shorter than `FIXED_LAYER_COUNT`.
///
/// # Populating layers
///
/// Obtain a mutable reference to any fixed layer and call `insert_*` on it:
///
/// ```ignore
/// catalog.layer_mut(CatalogLayer::Database)
///        .insert_relation("users", Some(vec!["id".into(), "name".into()]));
/// ```
#[allow(dead_code)]
pub struct Catalog {
    layers: Vec<CatalogLayerContents>,
}

#[allow(dead_code)]
impl Catalog {
    /// Create a catalog for `dialect`. The dialect's built-in functions are
    /// loaded immediately into the dialect layer.
    pub fn new(dialect: Dialect) -> Self {
        let mut layers = vec![CatalogLayerContents::default(); FIXED_LAYER_COUNT];
        build_dialect_layer(&mut layers[LAYER_DIALECT], &dialect);
        Self { layers }
    }

    // ── Direct layer access ───────────────────────────────────────────────────

    /// Borrow a fixed layer immutably.
    pub fn layer(&self, which: CatalogLayer) -> &CatalogLayerContents {
        &self.layers[which.index()]
    }

    /// Borrow a fixed layer mutably.
    ///
    /// Use the returned [`CatalogLayerContents`] to insert relations, functions,
    /// or table-valued functions into the chosen layer.
    pub fn layer_mut(&mut self, which: CatalogLayer) -> &mut CatalogLayerContents {
        &mut self.layers[which.index()]
    }

    // ── Lifecycle convenience methods ─────────────────────────────────────────

    /// Switch to a new database.
    ///
    /// Clears the Database, Connection, and Document layers and discards all
    /// query scopes. Use this when the connected database changes entirely.
    pub fn new_database(&mut self) {
        self.layers.truncate(FIXED_LAYER_COUNT);
        for i in LAYER_DATABASE..FIXED_LAYER_COUNT {
            self.layers[i].clear();
        }
    }

    /// Switch to a new connection on the same database.
    ///
    /// Resets the Connection and Document layers and discards all query scopes.
    pub fn new_connection(&mut self) {
        self.layers.truncate(FIXED_LAYER_COUNT);
        for i in LAYER_CONNECTION..FIXED_LAYER_COUNT {
            self.layers[i].clear();
        }
    }

    /// Start a new document analysis pass.
    ///
    /// Resets the Document layer and discards all query scopes.
    /// Call this at the start of each analysis pass before accumulating DDL.
    pub fn new_document(&mut self) {
        self.layers.truncate(FIXED_LAYER_COUNT);
        self.layers[LAYER_DOCUMENT].clear();
    }

    // ── Convenience constructors ──────────────────────────────────────────────

    /// Parse DDL statements from `source` and populate the database layer.
    #[cfg(feature = "sqlite")]
    pub(crate) fn from_ddl(dialect: Dialect, source: &str) -> Self {
        use syntaqlite_syntax::ParseOutcome;
        let mut catalog = Catalog::new(dialect.clone());
        let parser = syntaqlite_syntax::Parser::new();
        let mut session = parser.parse(source);
        loop {
            let stmt = match session.next() {
                ParseOutcome::Ok(stmt) => stmt,
                ParseOutcome::Done => break,
                ParseOutcome::Err(_) => continue,
            };
            let root = stmt.root();
            let root_id: AnyNodeId = root.node_id().into();
            catalog.accumulate_ddl(CatalogLayer::Database, stmt.erase(), root_id, dialect.clone());
        }
        catalog
    }

    /// Parse a JSON schema description into the database layer.
    ///
    /// Expected format:
    /// ```json
    /// {
    ///   "tables":    [{ "name": "users",        "columns": ["id", "name"] }],
    ///   "views":     [{ "name": "active_users", "columns": ["id"] }],
    ///   "functions": [{ "name": "my_func",      "args": 2 }]
    /// }
    /// ```
    #[cfg(feature = "json")]
    pub(crate) fn from_json(dialect: Dialect, s: &str) -> Result<Self, String> {
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
            serde_json::from_str(s).map_err(|e| format!("invalid catalog JSON: {e}"))?;

        let mut catalog = Catalog::new(dialect);
        let db = catalog.layer_mut(CatalogLayer::Database);
        for t in root.tables {
            let cols = t.columns.iter().map(|c| c.to_ascii_lowercase()).collect();
            db.insert_relation(t.name, Some(cols));
        }
        for v in root.views {
            let cols = v.columns.iter().map(|c| c.to_ascii_lowercase()).collect();
            db.insert_relation(v.name, Some(cols));
        }
        for f in root.functions {
            let arity = match f.args {
                Some(n) => AritySpec::Exact(n),
                None => AritySpec::Any,
            };
            db.insert_function_overload(f.name, FunctionCategory::Scalar, arity);
        }
        Ok(catalog)
    }

    // ── DDL accumulation ──────────────────────────────────────────────────────

    /// Extract DDL contributions from a parsed statement and insert them into
    /// `target`. Temporary objects are always routed to the Connection layer.
    ///
    /// Called statement-by-statement during analysis so that later statements
    /// can reference earlier DDL. Pass `CatalogLayer::Document` for inline DDL
    /// and `CatalogLayer::Database` when pre-populating a schema.
    pub(crate) fn accumulate_ddl(
        &mut self,
        target: CatalogLayer,
        stmt: AnyParsedStatement<'_>,
        root: AnyNodeId,
        dialect: Dialect,
    ) {
        let Some((tag, fields)) = stmt.extract_fields(root) else {
            return;
        };
        let idx = u32::from(tag) as usize;
        let Some(&role) = dialect.roles().get(idx) else {
            return;
        };

        match role {
            SemanticRole::DefineTable {
                name,
                columns,
                select,
            } => {
                let name_val = match fields[name as usize] {
                    FieldValue::Span(s) if !s.is_empty() => s.to_string(),
                    _ => return,
                };
                let cols = extract_columns(stmt, &fields, columns, select, dialect.roles());
                self.layers[target.index()].insert_relation(name_val, cols);
            }
            SemanticRole::DefineView {
                name,
                columns,
                select,
            } => {
                let name_val = match fields[name as usize] {
                    FieldValue::Span(s) if !s.is_empty() => s.to_string(),
                    _ => return,
                };
                let cols = extract_columns(stmt, &fields, columns, Some(select), dialect.roles());
                self.layers[target.index()].insert_relation(name_val, cols);
            }
            SemanticRole::DefineFunction {
                name,
                args,
                return_type,
            } => {
                let name_val = match fields[name as usize] {
                    FieldValue::Span(s) if !s.is_empty() => s.to_string(),
                    _ => return,
                };
                let arity = extract_function_arity(stmt, &fields, args);
                let layer = &mut self.layers[target.index()];
                layer.insert_function_overload(
                    name_val.clone(),
                    FunctionCategory::Scalar,
                    arity,
                );
                if is_table_returning(stmt, &fields, return_type, dialect.roles()) {
                    layer.insert_table_function(name_val, AritySpec::Any, Vec::new());
                }
            }
            // Non-DDL roles are irrelevant to catalog accumulation.
            _ => {}
        }
    }

    // ── Query scope management ────────────────────────────────────────────────

    /// Push a new empty scope frame. Called on subquery / CTE entry.
    pub(crate) fn push_query_scope(&mut self) {
        self.layers.push(CatalogLayerContents::default());
    }

    /// Pop the innermost scope frame. Called on subquery / CTE exit.
    pub(crate) fn pop_query_scope(&mut self) {
        if self.layers.len() > FIXED_LAYER_COUNT {
            self.layers.pop();
        }
    }

    /// Register a table or alias in the current (innermost) query scope.
    /// `columns = None` means the table exists but its column list is unknown —
    /// column references against it are conservatively accepted.
    pub(crate) fn add_query_table(&mut self, name: &str, columns: Option<Vec<String>>) {
        if let Some(frame) = self.layers[FIXED_LAYER_COUNT..].last_mut() {
            frame.insert_relation(name, columns);
        }
    }

    // ── Schema sync (used by SemanticAnalyzer) ────────────────────────────────

    /// Copy the Database and Connection layers from `src` into this catalog.
    ///
    /// Called at the start of each analysis pass to apply the user-provided schema.
    pub(crate) fn copy_schema_layers_from(&mut self, src: &Catalog) {
        self.layers[LAYER_DATABASE] = src.layers[LAYER_DATABASE].clone();
        self.layers[LAYER_CONNECTION] = src.layers[LAYER_CONNECTION].clone();
    }

    // ── Resolution ────────────────────────────────────────────────────────────

    /// Returns `true` if `name` is a known relation in any layer.
    pub(crate) fn resolve_relation(&self, name: &str) -> bool {
        self.all_layers_ordered()
            .any(|layer| layer.relation(name).is_some())
    }

    /// Returns `true` if `name` is a known table-valued function in any layer.
    pub(crate) fn resolve_table_function(&self, name: &str) -> bool {
        self.all_layers_ordered()
            .any(|layer| layer.table_function(name).is_some())
    }

    pub(crate) fn resolve_column(&self, table: Option<&str>, column: &str) -> ColumnResolution {
        if let Some(tbl) = table {
            return self.resolve_qualified_column(tbl, column);
        }
        self.resolve_unqualified_column(column)
    }

    pub(crate) fn check_function(&self, name: &str, arg_count: usize) -> FunctionCheckResult {
        let set = self
            .all_layers_ordered()
            .find_map(|layer| layer.function(name));
        let Some(set) = set else {
            return FunctionCheckResult::Unknown;
        };
        if set
            .overloads
            .iter()
            .copied()
            .any(|ov| overload_accepts(ov, arg_count))
        {
            return FunctionCheckResult::Ok;
        }
        FunctionCheckResult::WrongArity {
            expected: expected_fixed_arities(set),
        }
    }

    /// Return the column list for a table or table-valued function.
    ///
    /// - `Some(cols)` — found with a known column list.
    /// - `None` — not found, or found but columns are unknown (suppress column errors).
    ///
    /// Used by the walker when registering a table reference in the query scope.
    pub(crate) fn columns_for_table_source(&self, name: &str) -> Option<Vec<String>> {
        for layer in self.all_layers_ordered() {
            if let Some(rel) = layer.relation(name) {
                // None means unknown — pass that through so caller suppresses errors.
                return rel.columns.clone();
            }
            if let Some(tf) = layer.table_function(name) {
                return if tf.output_columns.is_empty() {
                    None // unknown output columns
                } else {
                    Some(tf.output_columns.clone())
                };
            }
        }
        None // not found
    }

    // ── Enumeration (for fuzzy suggestions and completions) ───────────────────

    pub(crate) fn all_relation_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in self.all_layers_ordered() {
            for rel in layer.relations.values() {
                push_unique(&mut seen, &mut out, &rel.name);
            }
        }
        out.sort_unstable_by_key(|n| canonical_name(n));
        out
    }

    pub(crate) fn all_column_names(&self, table: Option<&str>) -> Vec<String> {
        let mut names = Vec::new();
        for layer in self.all_layers_ordered() {
            for rel in layer.relations.values() {
                if table.is_none_or(|t| rel.name.eq_ignore_ascii_case(t))
                    && let Some(cols) = &rel.columns
                {
                    names.extend(cols.iter().map(|c| c.to_ascii_lowercase()));
                }
            }
        }
        names.sort_unstable();
        names.dedup();
        names
    }

    pub(crate) fn all_function_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in self.all_layers_ordered() {
            for f in layer.functions.values() {
                push_unique(&mut seen, &mut out, &f.name);
            }
        }
        out.sort_unstable_by_key(|n| canonical_name(n));
        out
    }

    pub(crate) fn all_table_function_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in self.all_layers_ordered() {
            for tf in layer.table_functions.values() {
                push_unique(&mut seen, &mut out, &tf.name);
            }
        }
        out.sort_unstable_by_key(|n| canonical_name(n));
        out
    }

    // ── Private helpers ───────────────────────────────────────────────────────

    /// Iterator over all layers in resolution priority order:
    /// query (innermost first) → document → connection → database → dialect.
    fn all_layers_ordered(&self) -> impl Iterator<Item = &CatalogLayerContents> {
        self.layers.iter().rev()
    }

    fn resolve_qualified_column(&self, table: &str, column: &str) -> ColumnResolution {
        for layer in self.all_layers_ordered() {
            if let Some(rel) = layer.relation(table) {
                return match &rel.columns {
                    Some(cols) if cols.iter().any(|c| c.eq_ignore_ascii_case(column)) => {
                        ColumnResolution::Found
                    }
                    Some(_) => ColumnResolution::TableFoundColumnMissing,
                    None => ColumnResolution::Found, // unknown columns — accept conservatively
                };
            }
        }
        ColumnResolution::TableNotFound
    }

    fn resolve_unqualified_column(&self, column: &str) -> ColumnResolution {
        let mut has_unknown = false;
        for layer in self.all_layers_ordered() {
            for rel in layer.relations.values() {
                match &rel.columns {
                    Some(cols) if cols.iter().any(|c| c.eq_ignore_ascii_case(column)) => {
                        return ColumnResolution::Found;
                    }
                    Some(_) => {}
                    None => has_unknown = true,
                }
            }
        }
        if has_unknown {
            // A table with unknown columns is in scope — can't rule the column out.
            ColumnResolution::Found
        } else {
            ColumnResolution::NotFound
        }
    }
}

// ── DDL extraction helpers ────────────────────────────────────────────────────

/// Extract columns for a table/view DDL contribution.
///
/// Tries the explicit column list first; if absent, infers column names from
/// the result columns of the AS-SELECT body. Returns `None` only when
/// inference is impossible (e.g. `SELECT *`), which tells the catalog to
/// accept any column reference conservatively.
fn extract_columns<'a>(
    stmt: AnyParsedStatement<'a>,
    fields: &NodeFields<'a>,
    columns_field: Option<u8>,
    select_field: Option<u8>,
    roles: &'static [SemanticRole],
) -> Option<Vec<String>> {
    // Explicit column list takes priority.
    if let Some(col_idx) = columns_field
        && let FieldValue::NodeId(col_list_id) = fields[col_idx as usize]
        && !col_list_id.is_null()
    {
        let mut columns = Vec::new();
        columns_from_column_list(stmt, col_list_id, roles, &mut columns);
        if !columns.is_empty() {
            return Some(columns);
        }
    }

    // Fall back to inferring names from SELECT result columns.
    if let Some(sel_idx) = select_field
        && let FieldValue::NodeId(select_id) = fields[sel_idx as usize]
        && !select_id.is_null()
    {
        return columns_from_select(stmt, select_id, roles);
    }

    None
}

/// Check whether a DDL function returns a table.
fn is_table_returning<'a>(
    stmt: AnyParsedStatement<'a>,
    fields: &NodeFields<'a>,
    return_type_field: Option<u8>,
    roles: &'static [SemanticRole],
) -> bool {
    let Some(rt_idx) = return_type_field else {
        return false;
    };
    let FieldValue::NodeId(rt_id) = fields[rt_idx as usize] else {
        return false;
    };
    if rt_id.is_null() {
        return false;
    }
    let Some((rt_tag, rt_fields)) = stmt.extract_fields(rt_id) else {
        return false;
    };
    let tag_idx = u32::from(rt_tag) as usize;
    let Some(&SemanticRole::ReturnSpec { columns }) = roles.get(tag_idx) else {
        return false;
    };
    let Some(cols_idx) = columns else {
        return false;
    };
    matches!(rt_fields[cols_idx as usize], FieldValue::NodeId(id) if !id.is_null())
}

/// Extract argument count for a function DDL contribution.
fn extract_function_arity<'a>(
    stmt: AnyParsedStatement<'a>,
    fields: &NodeFields<'a>,
    args_field: Option<u8>,
) -> AritySpec {
    let Some(args_idx) = args_field else {
        return AritySpec::Any;
    };
    let FieldValue::NodeId(args_id) = fields[args_idx as usize] else {
        return AritySpec::Any;
    };
    if args_id.is_null() {
        return AritySpec::Any;
    }
    let Some(children) = stmt.list_children(args_id) else {
        return AritySpec::Any;
    };
    AritySpec::Exact(children.len())
}

fn columns_from_column_list(
    stmt: AnyParsedStatement<'_>,
    list_id: AnyNodeId,
    roles: &'static [SemanticRole],
    out: &mut Vec<String>,
) {
    let Some(children) = stmt.list_children(list_id) else {
        return;
    };

    for &child_id in children {
        if child_id.is_null() {
            continue;
        }
        let Some((child_tag, child_fields)) = stmt.extract_fields(child_id) else {
            continue;
        };

        // Use the ColumnDef role to find the name field precisely.
        let tag_idx = u32::from(child_tag) as usize;
        let Some(&SemanticRole::ColumnDef { name: name_idx, .. }) = roles.get(tag_idx) else {
            continue;
        };

        let FieldValue::NodeId(name_id) = child_fields[name_idx as usize] else {
            continue;
        };
        if name_id.is_null() {
            continue;
        }
        let Some((_, name_fields)) = stmt.extract_fields(name_id) else {
            continue;
        };
        // The first non-empty Span inside the name node is the identifier text.
        for j in 0..name_fields.len() {
            if let FieldValue::Span(s) = name_fields[j]
                && !s.is_empty()
            {
                out.push(s.to_ascii_lowercase());
                break;
            }
        }
    }
}

/// Infer column names from a SELECT statement's result column list.
///
/// Returns `Some(names)` if every result column has an inferable name:
/// - An explicit alias is used as-is.
/// - A bare `ColumnRef` with no alias uses the column name.
///
/// Returns `None` if any result column uses `*` (STAR) or has an expression
/// that cannot be named (e.g. a literal or function call without an alias).
/// A `None` return causes the caller to register the table with unknown
/// columns, conservatively accepting all column references.
pub(super) fn columns_from_select(
    stmt: AnyParsedStatement<'_>,
    select_id: AnyNodeId,
    roles: &[SemanticRole],
) -> Option<Vec<String>> {
    let (select_tag, select_fields) = stmt.extract_fields(select_id)?;
    let select_role = roles.get(u32::from(select_tag) as usize).copied()?;

    let SemanticRole::Query {
        columns: cols_idx, ..
    } = select_role
    else {
        return None;
    };

    let FieldValue::NodeId(list_id) = select_fields[cols_idx as usize] else {
        return None;
    };
    if list_id.is_null() {
        return None;
    }

    let children = stmt.list_children(list_id)?;
    let mut out = Vec::new();

    for &child_id in children {
        if child_id.is_null() {
            continue;
        }
        let (child_tag, child_fields) = stmt.extract_fields(child_id)?;
        let child_role = roles
            .get(u32::from(child_tag) as usize)
            .copied()
            .unwrap_or(SemanticRole::Transparent);

        let SemanticRole::ResultColumn {
            flags: flags_idx,
            alias: alias_idx,
            expr: expr_idx,
        } = child_role
        else {
            continue;
        };

        // STAR flag (bit 0) → wildcard: can't enumerate columns.
        if let FieldValue::Flags(f) = child_fields[flags_idx as usize]
            && f & 1 != 0
        {
            return None;
        }

        match infer_result_col_name(stmt, &child_fields, alias_idx, expr_idx, roles) {
            Some(name) => out.push(name),
            None => return None,
        }
    }

    if out.is_empty() { None } else { Some(out) }
}

/// Infer the output column name for a single result column node.
fn infer_result_col_name<'a>(
    stmt: AnyParsedStatement<'a>,
    child_fields: &NodeFields<'a>,
    alias_idx: u8,
    expr_idx: u8,
    roles: &[SemanticRole],
) -> Option<String> {
    // Try explicit alias.
    if let FieldValue::NodeId(alias_id) = child_fields[alias_idx as usize]
        && !alias_id.is_null()
        && let Some((_, alias_fields)) = stmt.extract_fields(alias_id)
    {
        for j in 0..alias_fields.len() {
            if let FieldValue::Span(s) = alias_fields[j]
                && !s.is_empty()
            {
                return Some(s.to_ascii_lowercase());
            }
        }
    }

    // Try bare ColumnRef (no alias).
    if let FieldValue::NodeId(expr_id) = child_fields[expr_idx as usize]
        && !expr_id.is_null()
        && let Some((expr_tag, expr_fields)) = stmt.extract_fields(expr_id)
    {
        let expr_role = roles
            .get(u32::from(expr_tag) as usize)
            .copied()
            .unwrap_or(SemanticRole::Transparent);
        if let SemanticRole::ColumnRef {
            column: col_idx, ..
        } = expr_role
            && let FieldValue::Span(col_span) = expr_fields[col_idx as usize]
            && !col_span.is_empty()
        {
            return Some(col_span.to_ascii_lowercase());
        }
    }

    None
}

// ── Dialect layer builder ─────────────────────────────────────────────────────

fn build_dialect_layer(layer: &mut CatalogLayerContents, dialect: &Dialect) {
    #[cfg(feature = "sqlite")]
    for entry in crate::sqlite::functions_catalog::SQLITE_FUNCTIONS {
        if !is_function_available(entry, dialect) {
            continue;
        }
        if entry.info.category == DialectFunctionCategory::TableValued {
            layer.insert_table_function(entry.info.name.to_string(), AritySpec::Any, Vec::new());
        } else {
            layer.insert_function_arities(
                entry.info.name.to_string(),
                map_function_category(entry.info.category),
                entry.info.arities,
            );
        }
    }
}

fn map_function_category(category: DialectFunctionCategory) -> FunctionCategory {
    match category {
        DialectFunctionCategory::Scalar | DialectFunctionCategory::TableValued => {
            FunctionCategory::Scalar // TableValued is unreachable via this path
        }
        DialectFunctionCategory::Aggregate => FunctionCategory::Aggregate,
        DialectFunctionCategory::Window => FunctionCategory::Window,
    }
}

// ── Utilities ─────────────────────────────────────────────────────────────────

fn canonical_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

#[allow(dead_code)]
fn overload_accepts(overload: FunctionOverload, arg_count: usize) -> bool {
    match overload.arity {
        AritySpec::Exact(n) => n == arg_count,
        AritySpec::AtLeast(min) => arg_count >= min,
        AritySpec::Any => true,
    }
}

#[allow(dead_code)]
fn expected_fixed_arities(set: &FunctionSet) -> Vec<usize> {
    let mut arities: Vec<usize> = set
        .overloads
        .iter()
        .filter_map(|ov| match ov.arity {
            AritySpec::Exact(n) => Some(n),
            AritySpec::AtLeast(_) | AritySpec::Any => None,
        })
        .collect();
    arities.sort_unstable();
    arities.dedup();
    arities
}

#[allow(dead_code)]
fn push_unique(seen: &mut HashSet<String>, out: &mut Vec<String>, name: &str) {
    let lower = canonical_name(name);
    if seen.insert(lower) {
        out.push(name.to_string());
    }
}
