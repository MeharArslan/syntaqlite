// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Layered semantic catalog.
//!
//! Precedence order is: document -> database -> static.

use std::collections::{HashMap, HashSet};

use syntaqlite_syntax::any::FieldKind;
use syntaqlite_syntax::any::{AnyNodeId, AnyParsedStatement, FieldValue};
use syntaqlite_syntax::ast_traits::{
    AstTypes, ColumnConstraintTypeKind, ExprKind, SelectKind, TableSourceKind,
};
use syntaqlite_syntax::typed::GrammarNodeType;

use crate::dialect::Dialect;
use crate::dialect::catalog::{FunctionCategory as DialectFunctionCategory, is_function_available};

use super::schema::{ColumnDef, FunctionDef, RelationDef, RelationKind};

// -- Core catalog types ------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum FunctionCategory {
    Scalar,
    Aggregate,
    Window,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum AritySpec {
    Exact(usize),
    AtLeast(usize),
    Any,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct FunctionOverload {
    pub category: FunctionCategory,
    pub arity: AritySpec,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FunctionSet {
    name: String,
    overloads: Vec<FunctionOverload>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct RelationEntry {
    name: String,
    columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableFunctionSet {
    name: String,
    overloads: Vec<FunctionOverload>,
    output_columns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FunctionCheckResult {
    Ok,
    Unknown,
    WrongArity { expected: Vec<usize> },
}

#[derive(Debug, Default, Clone)]
pub(crate) struct CatalogLayer {
    relations: HashMap<String, RelationEntry>,
    functions: HashMap<String, FunctionSet>,
    table_functions: HashMap<String, TableFunctionSet>,
}

impl CatalogLayer {
    pub(crate) fn clear(&mut self) {
        self.relations.clear();
        self.functions.clear();
        self.table_functions.clear();
    }

    pub(crate) fn insert_relation(&mut self, name: impl Into<String>, columns: Vec<String>) {
        let name = name.into();
        self.relations
            .insert(canonical_name(&name), RelationEntry { name, columns });
    }

    pub(crate) fn insert_function_overload(
        &mut self,
        name: impl Into<String>,
        category: FunctionCategory,
        arity: AritySpec,
    ) {
        let name = name.into();
        let key = canonical_name(&name);
        self.functions
            .entry(key)
            .and_modify(|set| {
                set.overloads.push(FunctionOverload { category, arity });
            })
            .or_insert_with(|| FunctionSet {
                name,
                overloads: vec![FunctionOverload { category, arity }],
            });
    }

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
            let arity = if a == -1 {
                AritySpec::Any
            } else if a < -1 {
                AritySpec::AtLeast(
                    usize::try_from(-i32::from(a) - 1).expect("negative arity encodes minimum"),
                )
            } else {
                AritySpec::Exact(
                    usize::try_from(i32::from(a)).expect("fixed arity must be non-negative"),
                )
            };
            self.insert_function_overload(name.clone(), category, arity);
        }
    }

    pub(crate) fn insert_table_function_overload(
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

#[derive(Debug, Clone)]
struct Catalog {
    layers: Vec<CatalogLayer>,
}

impl Catalog {
    fn new(layer_count: usize) -> Self {
        Catalog {
            layers: vec![CatalogLayer::default(); layer_count],
        }
    }

    fn replace_layer(&mut self, idx: usize, layer: &CatalogLayer) {
        self.layers[idx] = layer.clone();
    }

    fn check_function(&self, name: &str, arg_count: usize) -> FunctionCheckResult {
        let mut maybe_set = None;
        for layer in &self.layers {
            if let Some(set) = layer.function(name) {
                maybe_set = Some(set);
                break;
            }
        }
        let Some(set) = maybe_set else {
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

    fn resolve_relation(&self, name: &str) -> bool {
        for layer in &self.layers {
            if layer.relation(name).is_some() {
                return true;
            }
        }
        false
    }

    fn columns_for_relation(&self, name: &str) -> Option<Vec<String>> {
        for layer in &self.layers {
            if let Some(relation) = layer.relation(name) {
                return Some(relation.columns.clone());
            }
        }
        None
    }

    fn all_relation_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in &self.layers {
            for relation in layer.relations.values() {
                push_unique_name(&mut seen, &mut out, &relation.name);
            }
        }
        out.sort_unstable_by_key(|name| canonical_name(name));
        out
    }

    fn all_column_names(&self, table: Option<&str>) -> Vec<String> {
        let mut names = Vec::new();
        for layer in &self.layers {
            for relation in layer.relations.values() {
                if table.is_none_or(|tbl| relation.name.eq_ignore_ascii_case(tbl)) {
                    names.extend(relation.columns.iter().map(|c| c.to_ascii_lowercase()));
                }
            }
        }
        names.sort_unstable();
        names.dedup();
        names
    }

    fn all_function_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in &self.layers {
            for function in layer.functions.values() {
                push_unique_name(&mut seen, &mut out, &function.name);
            }
        }
        out.sort_unstable_by_key(|name| canonical_name(name));
        out
    }

    #[allow(dead_code)]
    fn table_function(&self, name: &str) -> Option<&TableFunctionSet> {
        for layer in &self.layers {
            if let Some(table_function) = layer.table_function(name) {
                return Some(table_function);
            }
        }
        None
    }

    #[allow(dead_code)]
    fn all_table_function_names(&self) -> Vec<String> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for layer in &self.layers {
            for function in layer.table_functions.values() {
                push_unique_name(&mut seen, &mut out, &function.name);
            }
        }
        out.sort_unstable_by_key(|name| canonical_name(name));
        out
    }
}

// -- Semantic-facing catalog API --------------------------------------------

#[derive(Default, Clone)]
pub(crate) struct DatabaseCatalog {
    pub relations: Vec<RelationDef>,
    pub functions: Vec<FunctionDef>,
}

impl DatabaseCatalog {
    pub(crate) fn tables(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.relations
            .iter()
            .filter(|r| r.kind == RelationKind::Table)
    }

    pub(crate) fn views(&self) -> impl Iterator<Item = &RelationDef> + '_ {
        self.relations
            .iter()
            .filter(|r| r.kind == RelationKind::View)
    }

    fn as_layer(&self) -> CatalogLayer {
        let mut layer = CatalogLayer::default();
        add_relation_defs(&mut layer, &self.relations);
        add_function_defs(&mut layer, &self.functions);
        layer
    }

    #[cfg(feature = "sqlite")]
    pub(crate) fn from_ddl<A: for<'a> AstTypes<'a>>(dialect: Dialect, source: &str) -> Self {
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
                doc.accumulate::<A>(any_result, root_id, dialect, None);
            }
        }

        DatabaseCatalog {
            relations: doc.relations,
            functions: doc.functions,
        }
    }

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

pub(crate) struct StaticCatalog {
    layer: CatalogLayer,
}

impl StaticCatalog {
    pub(crate) fn for_dialect(dialect: &Dialect) -> Self {
        let mut layer = CatalogLayer::default();
        add_builtin_functions(&mut layer, dialect);
        StaticCatalog { layer }
    }
}

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
            known: HashMap::new(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.relations.clear();
        self.functions.clear();
        self.known.clear();
    }

    fn as_layer(&self) -> CatalogLayer {
        let mut layer = CatalogLayer::default();
        add_relation_defs(&mut layer, &self.relations);
        add_function_defs(&mut layer, &self.functions);
        layer
    }

    pub(crate) fn accumulate<'a, A: AstTypes<'a>>(
        &mut self,
        stmt_result: AnyParsedStatement<'a>,
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
                    columns_from_column_list::<A>(stmt_result, col_list_id, dialect, &mut columns);
                }

                if !has_columns
                    && let Some(sel_field_idx) = contrib.select_field
                    && let FieldValue::NodeId(sel_id) = fields[sel_field_idx as usize]
                    && !sel_id.is_null()
                {
                    columns_from_select::<A>(
                        stmt_result,
                        sel_id,
                        &self.known,
                        database,
                        &mut columns,
                    );
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

pub(crate) struct CatalogStack<'a> {
    pub(crate) static_: &'a StaticCatalog,
    pub(crate) database: &'a DatabaseCatalog,
    pub(crate) document: &'a DocumentCatalog,
}

impl CatalogStack<'_> {
    fn build_catalog(&self) -> Catalog {
        let document_layer = self.document.as_layer();
        let database_layer = self.database.as_layer();

        let mut catalog = Catalog::new(3);
        catalog.replace_layer(0, &document_layer);
        catalog.replace_layer(1, &database_layer);
        catalog.replace_layer(2, &self.static_.layer);
        catalog
    }

    pub(crate) fn check_function(&self, name: &str, arg_count: usize) -> FunctionCheckResult {
        self.build_catalog().check_function(name, arg_count)
    }

    pub(crate) fn resolve_relation(&self, name: &str) -> bool {
        self.build_catalog().resolve_relation(name)
    }

    pub(crate) fn columns_for(&self, name: &str) -> Option<Vec<String>> {
        self.build_catalog().columns_for_relation(name)
    }

    pub(crate) fn all_relation_names(&self) -> Vec<String> {
        self.build_catalog().all_relation_names()
    }

    pub(crate) fn all_column_names(&self, table: Option<&str>) -> Vec<String> {
        self.build_catalog().all_column_names(table)
    }

    pub(crate) fn all_function_names(&self) -> Vec<String> {
        self.build_catalog().all_function_names()
    }
}

// -- DDL extraction helpers --------------------------------------------------

fn columns_from_column_list<'a, A: AstTypes<'a>>(
    stmt_result: AnyParsedStatement<'a>,
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
            extract_column_constraints::<A>(
                stmt_result,
                constraints_id,
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

fn extract_column_constraints<'a, A: AstTypes<'a>>(
    stmt_result: AnyParsedStatement<'a>,
    list_id: AnyNodeId,
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
        let Some(constraint) = A::ColumnConstraint::from_result(stmt_result, constraint_id) else {
            continue;
        };

        match constraint.kind().kind() {
            Some(ColumnConstraintTypeKind::NotNull) => {
                *is_nullable = false;
            }
            Some(ColumnConstraintTypeKind::PrimaryKey) => {
                *is_primary_key = true;
                *is_nullable = false;
            }
            _ => {}
        }
    }
}

type KnownSchema = HashMap<String, Vec<ColumnDef>>;

fn columns_from_select<'a, A: AstTypes<'a>>(
    stmt_result: AnyParsedStatement<'a>,
    select_id: AnyNodeId,
    known: &KnownSchema,
    database: Option<&DatabaseCatalog>,
    out: &mut Vec<ColumnDef>,
) {
    let Some(select) = A::Select::from_result(stmt_result, select_id) else {
        return;
    };

    let stmt = match select.kind() {
        SelectKind::SelectStmt(s) => s,
        SelectKind::CompoundSelect(cs) => {
            if let Some(left) = cs.left() {
                let id: AnyNodeId = left.node_id().into();
                return columns_from_select::<A>(stmt_result, id, known, database, out);
            }
            return;
        }
        SelectKind::WithClause(wc) => {
            if let Some(s) = wc.select() {
                let id: AnyNodeId = s.node_id().into();
                return columns_from_select::<A>(stmt_result, id, known, database, out);
            }
            return;
        }
        _ => return,
    };

    let from_sources = stmt
        .from_clause()
        .map(|ts| {
            let id: AnyNodeId = ts.node_id().into();
            collect_from_sources::<A>(stmt_result, id, known, database)
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
        } else if let Some(expr) = rc.expr() {
            match expr.kind() {
                ExprKind::ColumnRef(cr) => cr.column().to_string(),
                _ => continue,
            }
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

struct FromSource {
    qualifier: String,
    columns: Vec<ColumnDef>,
}

fn collect_from_sources<'a, A: AstTypes<'a>>(
    stmt_result: AnyParsedStatement<'a>,
    source_id: AnyNodeId,
    known: &KnownSchema,
    database: Option<&DatabaseCatalog>,
) -> Vec<FromSource> {
    let mut out = Vec::new();

    let Some(source) = A::TableSource::from_result(stmt_result, source_id) else {
        return out;
    };

    match source.kind() {
        TableSourceKind::TableRef(tr) => {
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
        TableSourceKind::SubqueryTableSource(sq) => {
            let mut columns = Vec::new();
            if let Some(select) = sq.select() {
                let id: AnyNodeId = select.node_id().into();
                columns_from_select::<A>(stmt_result, id, known, database, &mut columns);
            }
            out.push(FromSource {
                qualifier: sq.alias().to_string(),
                columns,
            });
        }
        TableSourceKind::JoinClause(jc) => {
            if let Some(left) = jc.left() {
                let id: AnyNodeId = left.node_id().into();
                out.extend(collect_from_sources::<A>(stmt_result, id, known, database));
            }
            if let Some(right) = jc.right() {
                let id: AnyNodeId = right.node_id().into();
                out.extend(collect_from_sources::<A>(stmt_result, id, known, database));
            }
        }
        TableSourceKind::JoinPrefix(jp) => {
            if let Some(s) = jp.source() {
                let id: AnyNodeId = s.node_id().into();
                out.extend(collect_from_sources::<A>(stmt_result, id, known, database));
            }
        }
        _ => {}
    }
    out
}

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

fn add_relation_defs(layer: &mut CatalogLayer, relations: &[RelationDef]) {
    for relation in relations {
        layer.insert_relation(
            relation.name.clone(),
            relation.columns.iter().map(|c| c.name.clone()).collect(),
        );
    }
}

fn add_function_defs(layer: &mut CatalogLayer, functions: &[FunctionDef]) {
    for function in functions {
        let arity = match function.args {
            Some(n) => AritySpec::Exact(n),
            None => AritySpec::Any,
        };
        layer.insert_function_overload(function.name.clone(), FunctionCategory::Scalar, arity);
    }
}

fn add_builtin_functions(layer: &mut CatalogLayer, dialect: &Dialect) {
    #[cfg(feature = "sqlite")]
    {
        for entry in crate::sqlite::functions_catalog::SQLITE_FUNCTIONS {
            if is_function_available(entry, dialect) {
                layer.insert_function_arities(
                    entry.info.name.to_string(),
                    map_function_category(entry.info.category),
                    entry.info.arities,
                );
            }
        }
    }

    for ext in dialect.function_extensions().iter() {
        if is_function_available(ext, dialect) {
            layer.insert_function_arities(
                ext.info.name.to_string(),
                map_function_category(ext.info.category),
                ext.info.arities,
            );
        }
    }
}

fn map_function_category(category: DialectFunctionCategory) -> FunctionCategory {
    match category {
        DialectFunctionCategory::Scalar => FunctionCategory::Scalar,
        DialectFunctionCategory::Aggregate => FunctionCategory::Aggregate,
        DialectFunctionCategory::Window => FunctionCategory::Window,
    }
}

fn canonical_name(name: &str) -> String {
    name.to_ascii_lowercase()
}

fn overload_accepts(overload: FunctionOverload, arg_count: usize) -> bool {
    match overload.arity {
        AritySpec::Exact(n) => n == arg_count,
        AritySpec::AtLeast(min) => arg_count >= min,
        AritySpec::Any => true,
    }
}

fn expected_fixed_arities(set: &FunctionSet) -> Vec<usize> {
    let mut expected: Vec<usize> = set
        .overloads
        .iter()
        .filter_map(|ov| match ov.arity {
            AritySpec::Exact(n) => Some(n),
            AritySpec::AtLeast(_) | AritySpec::Any => None,
        })
        .collect();
    expected.sort_unstable();
    expected.dedup();
    expected
}

fn push_unique_name(seen: &mut HashSet<String>, out: &mut Vec<String>, name: &str) {
    let lower = canonical_name(name);
    if seen.insert(lower) {
        out.push(name.to_string());
    }
}
