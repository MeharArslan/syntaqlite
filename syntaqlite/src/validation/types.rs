// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

/// A diagnostic message associated with a source range.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    /// Byte offset of the start of the diagnostic range.
    pub start_offset: usize,
    /// Byte offset of the end of the diagnostic range.
    pub end_offset: usize,
    /// Structured diagnostic message.
    pub message: DiagnosticMessage,
    /// Severity level.
    pub severity: Severity,
    /// Optional structured help attached to the diagnostic.
    pub help: Option<Help>,
}

/// Structured diagnostic message.
///
/// Each variant carries the identifiers needed for machine-readable
/// consumption; [`fmt::Display`](std::fmt::Display) produces the human-readable form.
#[derive(Debug, Clone)]
pub enum DiagnosticMessage {
    UnknownTable {
        name: String,
    },
    UnknownColumn {
        column: String,
        table: Option<String>,
    },
    UnknownFunction {
        name: String,
    },
    FunctionArity {
        name: String,
        expected: Vec<usize>,
        got: usize,
    },
    /// Catch-all for parse errors and other unstructured messages.
    Other(String),
}

impl std::fmt::Display for DiagnosticMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownTable { name } => write!(f, "unknown table '{name}'"),
            Self::UnknownColumn {
                column,
                table: Some(t),
            } => {
                write!(f, "unknown column '{column}' in table '{t}'")
            }
            Self::UnknownColumn {
                column,
                table: None,
            } => {
                write!(f, "unknown column '{column}'")
            }
            Self::UnknownFunction { name } => write!(f, "unknown function '{name}'"),
            Self::FunctionArity {
                name,
                expected,
                got,
            } => {
                let expected_str: Vec<String> = expected.iter().map(|n| n.to_string()).collect();
                write!(
                    f,
                    "function '{name}' expects {} argument(s), got {got}",
                    expected_str.join(" or ")
                )
            }
            Self::Other(msg) => f.write_str(msg),
        }
    }
}

impl DiagnosticMessage {
    /// Returns `true` for parse errors (`Other`), `false` for semantic diagnostics.
    pub fn is_parse_error(&self) -> bool {
        matches!(self, Self::Other(_))
    }

    /// Write the structured JSON representation into `out`.
    ///
    /// This is the machine-readable detail object; callers also emit
    /// `"message"` with the [`fmt::Display`](std::fmt::Display) string alongside it.
    #[cfg(feature = "json")]
    pub fn write_json(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).expect("DiagnosticMessage serialization failed"));
    }
}

/// Structured help information attached to a diagnostic.
#[derive(Debug, Clone)]
pub enum Help {
    /// A "did you mean?" suggestion with the corrected identifier.
    Suggestion(String),
}

impl std::fmt::Display for Help {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Help::Suggestion(s) => write!(f, "did you mean '{s}'?"),
        }
    }
}

impl Help {
    /// Write the structured JSON representation into `out`.
    #[cfg(feature = "json")]
    pub fn write_json(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).expect("Help serialization failed"));
    }
}

impl Diagnostic {
    /// Write the full diagnostic as a JSON object into `out`.
    #[cfg(feature = "json")]
    pub fn write_json(&self, out: &mut String) {
        out.push_str(&serde_json::to_string(self).expect("Diagnostic serialization failed"));
    }
}

// ── JSON serialization (feature = "json") ────────────────────────────

#[cfg(feature = "json")]
impl serde::Serialize for DiagnosticMessage {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Self::Other(_) => serializer.serialize_none(),
            Self::UnknownTable { name } => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("kind", "unknown_table")?;
                m.serialize_entry("name", name)?;
                m.end()
            }
            Self::UnknownColumn { column, table } => {
                let len = if table.is_some() { 3 } else { 2 };
                let mut m = serializer.serialize_map(Some(len))?;
                m.serialize_entry("kind", "unknown_column")?;
                m.serialize_entry("column", column)?;
                if let Some(t) = table {
                    m.serialize_entry("table", t)?;
                }
                m.end()
            }
            Self::UnknownFunction { name } => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("kind", "unknown_function")?;
                m.serialize_entry("name", name)?;
                m.end()
            }
            Self::FunctionArity {
                name,
                expected,
                got,
            } => {
                let mut m = serializer.serialize_map(Some(4))?;
                m.serialize_entry("kind", "function_arity")?;
                m.serialize_entry("name", name)?;
                m.serialize_entry("expected", expected)?;
                m.serialize_entry("got", got)?;
                m.end()
            }
        }
    }
}

#[cfg(feature = "json")]
impl serde::Serialize for Help {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        match self {
            Self::Suggestion(value) => {
                let mut m = serializer.serialize_map(Some(2))?;
                m.serialize_entry("kind", "suggestion")?;
                m.serialize_entry("value", value)?;
                m.end()
            }
        }
    }
}

/// Serializes as a lowercase string (e.g. `"error"`, `"warning"`).
#[cfg(feature = "json")]
impl serde::Serialize for Severity {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
            Self::Hint => "hint",
        })
    }
}

/// Serializes with a distinct `"message"` (Display) and `"detail"` (structured)
/// field, matching the shape expected by LSP and WASM consumers.
#[cfg(feature = "json")]
impl serde::Serialize for Diagnostic {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let len = if self.help.is_some() { 7 } else { 5 };
        let mut m = serializer.serialize_map(Some(len))?;
        m.serialize_entry("startOffset", &self.start_offset)?;
        m.serialize_entry("endOffset", &self.end_offset)?;
        m.serialize_entry("message", &self.message.to_string())?;
        m.serialize_entry("detail", &self.message)?;
        m.serialize_entry("severity", &self.severity)?;
        if let Some(ref help) = self.help {
            m.serialize_entry("help", &help.to_string())?;
            m.serialize_entry("helpDetail", help)?;
        }
        m.end()
    }
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

/// Expand a [`FunctionInfo`](syntaqlite_parser::catalog::FunctionInfo) into one [`FunctionDef`] per arity.
pub(crate) fn expand_function_info(
    info: &syntaqlite_parser::catalog::FunctionInfo<'_>,
) -> Vec<FunctionDef> {
    if info.arities.is_empty() {
        vec![FunctionDef {
            name: info.name.to_string(),
            args: None,
            description: None,
        }]
    } else {
        info.arities
            .iter()
            .map(|&arity| FunctionDef {
                name: info.name.to_string(),
                args: if arity < 0 {
                    None
                } else {
                    Some(arity as usize)
                },
                description: None,
            })
            .collect()
    }
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

impl SessionContext {
    /// Build a `SessionContext` from parsed DDL statement roots.
    ///
    /// Processes statements in order, building up a schema incrementally.
    /// `SELECT *` and `SELECT t.*` in `CREATE TABLE … AS SELECT` or
    /// `CREATE VIEW … AS SELECT` are expanded using tables/views defined
    /// by earlier statements in the same input.
    pub fn from_stmts<'a>(
        reader: crate::parser::session::RawNodeReader<'a>,
        stmt_ids: &[syntaqlite_parser::nodes::NodeId],
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

    /// Build a `SessionContext` from a DDL source string.
    ///
    /// Creates a temporary parser, parses the source, and builds the schema
    /// from the resulting DDL statements. This is a convenience wrapper for
    /// cases like WASM where you have raw DDL text.
    pub fn from_ddl(
        dialect: crate::Dialect<'_>,
        source: &str,
        dialect_config: Option<syntaqlite_parser::dialect::ffi::DialectConfig>,
    ) -> Self {
        let mut builder = crate::parser::session::RawParser::builder(dialect);
        if let Some(dc) = dialect_config {
            builder = builder.dialect_config(dc);
        }
        let mut parser = builder.build();
        let mut cursor = parser.parse(source);

        let mut stmt_ids = Vec::new();
        while let Some(result) = cursor.next_statement() {
            if let Ok(node_ref) = result {
                stmt_ids.push(node_ref.id());
            }
        }

        Self::from_stmts(cursor.reader(), &stmt_ids, dialect)
    }

    /// Build a `SessionContext` from a JSON string.
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
            serde_json::from_str(s).map_err(|e| format!("invalid session context JSON: {e}"))?;

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

        Ok(SessionContext {
            relations,
            functions: root
                .functions
                .into_iter()
                .map(|f| FunctionDef {
                    name: f.name,
                    args: f.args,
                    description: None,
                })
                .collect(),
        })
    }
}

/// Schema accumulated from DDL statements earlier in the document being validated.
///
/// Built incrementally (one statement at a time) so forward references are rejected.
/// Pass to [`validate_statement`](super::validate_statement) alongside the session context so each statement only
/// sees tables/views that were *defined before it* in the document.
pub struct DocumentContext {
    pub relations: Vec<RelationDef>,
    pub functions: Vec<FunctionDef>,
    known: KnownSchema,
}

impl DocumentContext {
    pub fn new() -> Self {
        DocumentContext {
            relations: vec![],
            functions: vec![],
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

/// Read a `NodeId` field from a raw node pointer at the given metadata offset.
///
/// # Safety
/// `ptr` must point to a valid node struct; `meta.offset` must be a valid
/// offset to a `u32` (NodeId) field within that struct.
unsafe fn read_node_id(
    ptr: *const u8,
    meta: &syntaqlite_parser::dialect::ffi::FieldMeta,
) -> syntaqlite_parser::nodes::NodeId {
    unsafe { syntaqlite_parser::nodes::NodeId(*(ptr.add(meta.offset as usize) as *const u32)) }
}

/// Read a `SourceSpan` field from a raw node pointer, returning its text
/// (or `""` if the span is empty).
///
/// # Safety
/// `ptr` must point to a valid node struct; `meta.offset` must be a valid
/// offset to a `SourceSpan` field within that struct.
unsafe fn read_span<'a>(
    ptr: *const u8,
    meta: &syntaqlite_parser::dialect::ffi::FieldMeta,
    source: &'a str,
) -> &'a str {
    unsafe {
        let span = &*(ptr.add(meta.offset as usize) as *const syntaqlite_parser::nodes::SourceSpan);
        if span.is_empty() {
            ""
        } else {
            span.as_str(source)
        }
    }
}

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
        reader: crate::parser::session::RawNodeReader<'_>,
        stmt_id: syntaqlite_parser::nodes::NodeId,
        dialect: crate::Dialect<'_>,
        session: Option<&SessionContext>,
    ) {
        use crate::dialect::SchemaKind;
        use syntaqlite_parser::dialect::ffi::{FIELD_NODE_ID, FIELD_SPAN};
        use syntaqlite_parser::dialect_traits::DialectNodeType;

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
                        columns_from_select(&select, &self.known, session, &mut columns);
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

/// Extract column definitions from a column definition list node.
///
/// Walks list children, extracting `column_name`, `type_name`, and constraint
/// information (PRIMARY KEY, NOT NULL) from each child node's field metadata.
fn columns_from_column_list(
    reader: &crate::parser::session::RawNodeReader<'_>,
    list_id: syntaqlite_parser::nodes::NodeId,
    dialect: &crate::Dialect<'_>,
    out: &mut Vec<ColumnDef>,
) {
    use syntaqlite_parser::dialect::ffi::{FIELD_NODE_ID, FIELD_SPAN};
    use syntaqlite_parser::nodes::NodeId;

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
    reader: &crate::parser::session::RawNodeReader<'_>,
    list_id: syntaqlite_parser::nodes::NodeId,
    dialect: &crate::Dialect<'_>,
    is_primary_key: &mut bool,
    is_nullable: &mut bool,
) {
    use syntaqlite_parser::dialect::ffi::FIELD_ENUM;

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
        // Look for the "kind" enum field.
        for fm in meta {
            if fm.kind == FIELD_ENUM {
                let field_name = unsafe { fm.name_str() };
                if field_name == "kind" {
                    let ordinal = unsafe { *(cptr.add(fm.offset as usize) as *const u32) };
                    // Map ordinal to display name for robust matching.
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
    session: Option<&SessionContext>,
    out: &mut Vec<ColumnDef>,
) {
    use syntaqlite_parser_sqlite::ast::{Expr, Select};

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
struct FromSource {
    /// Alias if present, otherwise the table/view name. Used for `t.*` matching.
    qualifier: String,
    /// Columns resolved from the known schema (for table refs) or by
    /// recursively resolving the subquery (for subquery table sources).
    columns: Vec<ColumnDef>,
}

/// Walk a `TableSource` tree, resolving each leaf's columns eagerly.
fn collect_from_sources(
    source: &syntaqlite_parser_sqlite::ast::TableSource<'_>,
    known: &KnownSchema,
    session: Option<&SessionContext>,
) -> Vec<FromSource> {
    use syntaqlite_parser_sqlite::ast::TableSource;

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
        let dialect = crate::sqlite::dialect();
        let mut parser = crate::parser::session::RawParser::builder(dialect).build();
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids, dialect);

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
        let dialect = crate::sqlite::dialect();
        let mut parser = crate::parser::session::RawParser::builder(dialect).build();
        let sql = "CREATE TABLE orders AS SELECT order_id, total AS amount FROM src;";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids, dialect);

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
        let dialect = crate::sqlite::dialect();
        let mut parser = crate::parser::session::RawParser::builder(dialect).build();
        let sql = "\
            CREATE TABLE slice (order_id INTEGER, status TEXT);\n\
            CREATE TABLE orders AS SELECT * FROM slice;\n";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids, dialect);

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
        let dialect = crate::sqlite::dialect();
        let mut parser = crate::parser::session::RawParser::builder(dialect).build();
        let sql = "\
            CREATE TABLE a (x INTEGER);\n\
            CREATE TABLE b (y TEXT);\n\
            CREATE TABLE c AS SELECT a.* FROM a JOIN b ON 1;\n";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids, dialect);

        let tables: Vec<_> = ctx.tables().collect();
        let c = tables[2];
        assert_eq!(c.name, "c");
        assert_eq!(c.columns.len(), 1);
        assert_eq!(c.columns[0].name, "x");
    }

    #[test]
    fn from_stmts_star_with_alias() {
        let dialect = crate::sqlite::dialect();
        let mut parser = crate::parser::session::RawParser::builder(dialect).build();
        let sql = "\
            CREATE TABLE src (id INTEGER, val TEXT);\n\
            CREATE TABLE dst AS SELECT t.* FROM src AS t;\n";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids, dialect);

        let tables: Vec<_> = ctx.tables().collect();
        let dst = tables[1];
        assert_eq!(dst.name, "dst");
        assert_eq!(dst.columns.len(), 2);
        assert_eq!(dst.columns[0].name, "id");
        assert_eq!(dst.columns[1].name, "val");
    }

    #[test]
    fn from_stmts_star_through_subquery() {
        let dialect = crate::sqlite::dialect();
        let mut parser = crate::parser::session::RawParser::builder(dialect).build();
        let sql = "\
            CREATE TABLE slice (order_id INTEGER, customer_id TEXT);\n\
            CREATE TABLE orders AS SELECT * FROM (SELECT * FROM slice);\n";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids, dialect);

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
        let dialect = crate::sqlite::dialect();
        let mut parser = crate::parser::session::RawParser::builder(dialect).build();
        let sql = "CREATE VIEW active_users AS SELECT id, name FROM users WHERE active = 1;";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids, dialect);

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
        let dialect = crate::sqlite::dialect();
        let mut parser = crate::parser::session::RawParser::builder(dialect).build();
        let sql = "\
            CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT NOT NULL);\n\
            CREATE VIEW all_users AS SELECT * FROM users;\n";
        let mut cursor = parser.parse(sql);

        let stmt_ids: Vec<_> = (&mut cursor)
            .map(|r| r.map(|nr| nr.id()))
            .collect::<Result<Vec<_>, _>>()
            .expect("parse failed");
        let ctx = SessionContext::from_stmts(cursor.reader(), &stmt_ids, dialect);

        let views: Vec<_> = ctx.views().collect();
        assert_eq!(views.len(), 1);
        let view = views[0];
        assert_eq!(view.name, "all_users");
        assert_eq!(view.columns.len(), 2);
        assert_eq!(view.columns[0].name, "id");
        assert_eq!(view.columns[1].name, "name");
    }
}
