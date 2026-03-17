// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Recursive AST-walking lineage resolver.

use std::collections::HashMap;

use syntaqlite_syntax::any::{AnyNodeId, AnyParsedStatement, FieldValue};

use crate::dialect::SemanticRole;
use crate::semantic::catalog::Catalog;

use super::types::{ColumnLineage, ColumnOrigin, QueryLineage, RelationAccess, TableAccess};

/// Tracks information about a source in the FROM clause.
#[derive(Debug, Clone)]
struct SourceInfo {
    /// Known columns for this source (None = unknown).
    columns: Option<Vec<String>>,
}

/// Walks the AST to compute column lineage.
pub(super) struct LineageResolver<'a, 'b> {
    stmt: &'a AnyParsedStatement<'b>,
    catalog: &'a Catalog,
    roles: &'a [SemanticRole],
    /// CTE name -> body node ID.
    ctes: HashMap<String, AnyNodeId>,
    /// Subquery alias -> body node ID.
    subqueries: HashMap<String, AnyNodeId>,
    /// Source name -> info (populated while walking FROM).
    sources: HashMap<String, SourceInfo>,
    /// Alias -> canonical table name (for resolving aliases to tables).
    alias_to_table: HashMap<String, String>,
    /// Relations accessed directly in FROM.
    relations: Vec<RelationAccess>,
    /// Physical tables accessed (after resolving CTEs/subqueries).
    tables: Vec<TableAccess>,
    /// Whether all sources were fully resolved.
    complete: bool,
}

impl<'a, 'b> LineageResolver<'a, 'b> {
    pub(super) fn new(
        stmt: &'a AnyParsedStatement<'b>,
        catalog: &'a Catalog,
        roles: &'a [SemanticRole],
    ) -> Self {
        Self {
            stmt,
            catalog,
            roles,
            ctes: HashMap::new(),
            subqueries: HashMap::new(),
            sources: HashMap::new(),
            alias_to_table: HashMap::new(),
            relations: Vec::new(),
            tables: Vec::new(),
            complete: true,
        }
    }

    /// Entry point: find the outermost SELECT and resolve its lineage.
    pub(super) fn resolve(&mut self, root: AnyNodeId) -> Option<QueryLineage> {
        self.resolve_node(root)
    }

    fn resolve_node(&mut self, node_id: AnyNodeId) -> Option<QueryLineage> {
        if node_id.is_null() {
            return None;
        }
        let (tag, fields) = self.stmt.extract_fields(node_id)?;
        let role = self.role_for(tag);

        match role {
            SemanticRole::CteScope { bindings, body, .. } => {
                if let FieldValue::NodeId(bindings_id) = fields[bindings as usize] {
                    self.collect_cte_bindings(bindings_id);
                }
                if let FieldValue::NodeId(body_id) = fields[body as usize] {
                    return self.resolve_node(body_id);
                }
                None
            }
            SemanticRole::Query { .. } => self.resolve_select(node_id),
            SemanticRole::Transparent | SemanticRole::DmlScope => {
                for child_id in self.stmt.child_node_ids(node_id) {
                    if let Some(result) = self.resolve_node(child_id) {
                        return Some(result);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn collect_cte_bindings(&mut self, bindings_id: AnyNodeId) {
        if bindings_id.is_null() {
            return;
        }
        let Some(children) = self.stmt.list_children(bindings_id) else {
            self.try_register_cte(bindings_id);
            return;
        };
        for &child_id in children {
            self.try_register_cte(child_id);
        }
    }

    fn try_register_cte(&mut self, node_id: AnyNodeId) {
        if node_id.is_null() {
            return;
        }
        let Some((tag, fields)) = self.stmt.extract_fields(node_id) else {
            return;
        };
        let role = self.role_for(tag);

        if let SemanticRole::CteBinding { name, body, .. } = role {
            let cte_name = match fields[name as usize] {
                FieldValue::Span(s) if !s.is_empty() => Some(s.to_ascii_lowercase()),
                FieldValue::NodeId(id) if !id.is_null() => {
                    self.span_text(id).map(|s| s.to_ascii_lowercase())
                }
                _ => None,
            };
            if let Some(cte_name) = cte_name
                && let FieldValue::NodeId(body_id) = fields[body as usize]
            {
                self.ctes.insert(cte_name, body_id);
            }
        }
    }

    fn resolve_select(&mut self, select_id: AnyNodeId) -> Option<QueryLineage> {
        let (tag, fields) = self.stmt.extract_fields(select_id)?;
        let role = self.role_for(tag);

        let SemanticRole::Query {
            from,
            columns: cols_idx,
            ..
        } = role
        else {
            return None;
        };

        // Process FROM first to populate sources.
        if let FieldValue::NodeId(from_id) = fields[from as usize] {
            self.collect_from_sources(from_id);
        }

        // Process result columns.
        let columns = self.resolve_result_columns(select_id, cols_idx)?;

        // Resolve tables for each source.
        let source_names: Vec<String> = self.sources.keys().cloned().collect();
        for source_name in &source_names {
            self.resolve_tables_for_source(source_name);
        }

        // Deduplicate tables.
        self.tables.sort_by(|a, b| a.name.cmp(&b.name));
        self.tables.dedup_by(|a, b| a.name == b.name);

        Some(QueryLineage {
            complete: self.complete,
            columns,
            relations: std::mem::take(&mut self.relations),
            tables: std::mem::take(&mut self.tables),
        })
    }

    fn collect_from_sources(&mut self, from_id: AnyNodeId) {
        if from_id.is_null() {
            return;
        }

        let Some((tag, fields)) = self.stmt.extract_fields(from_id) else {
            return;
        };
        let role = self.role_for(tag);

        match role {
            SemanticRole::SourceRef { name, alias, .. } => {
                let source_name = self.span_text_from_field(&fields, name);
                let alias_name = self.span_text_from_field(&fields, alias);

                if let Some(ref sn) = source_name {
                    let display_name = alias_name.as_ref().unwrap_or(sn).to_ascii_lowercase();
                    let canonical = sn.to_ascii_lowercase();

                    if display_name != canonical {
                        self.alias_to_table
                            .insert(display_name.clone(), canonical.clone());
                    }

                    self.relations.push(RelationAccess {
                        name: canonical.clone(),
                    });

                    let columns = if self.ctes.contains_key(&canonical) {
                        let cte_body = self.ctes[&canonical];
                        super::super::catalog::columns_from_select(self.stmt, cte_body, self.roles)
                    } else {
                        let (cols, _) = self.catalog.table_source_info(&canonical);
                        cols
                    };

                    self.sources.insert(display_name, SourceInfo { columns });
                }
            }
            SemanticRole::ScopedSource { body, alias } => {
                if let Some(alias_text) = self.span_text_from_field(&fields, alias)
                    && let FieldValue::NodeId(body_id) = fields[body as usize]
                {
                    let alias_lower = alias_text.to_ascii_lowercase();
                    let cols =
                        super::super::catalog::columns_from_select(self.stmt, body_id, self.roles);
                    self.subqueries.insert(alias_lower.clone(), body_id);
                    self.sources
                        .insert(alias_lower.clone(), SourceInfo { columns: cols });
                    self.relations.push(RelationAccess { name: alias_lower });
                }
            }
            _ => {
                for child_id in self.stmt.child_node_ids(from_id) {
                    self.collect_from_sources(child_id);
                }
            }
        }
    }

    fn resolve_result_columns(
        &self,
        select_id: AnyNodeId,
        cols_idx: u8,
    ) -> Option<Vec<ColumnLineage>> {
        let (_, fields) = self.stmt.extract_fields(select_id)?;
        let FieldValue::NodeId(list_id) = fields[cols_idx as usize] else {
            return None;
        };
        if list_id.is_null() {
            return None;
        }

        let children = self.stmt.list_children(list_id)?;
        let mut result = Vec::new();
        let mut index: u32 = 0;

        for &child_id in children {
            if child_id.is_null() {
                continue;
            }
            let Some((child_tag, child_fields)) = self.stmt.extract_fields(child_id) else {
                continue;
            };
            let child_role = self.role_for(child_tag);

            let SemanticRole::ResultColumn {
                flags: flags_idx,
                alias: alias_idx,
                expr: expr_idx,
            } = child_role
            else {
                continue;
            };

            // Check for STAR (SELECT *).
            if let FieldValue::Flags(f) = child_fields[flags_idx as usize]
                && f & 1 != 0
            {
                let expanded = self.expand_star();
                for col in expanded {
                    result.push(ColumnLineage {
                        name: col.name.clone(),
                        index,
                        origin: col.origin,
                    });
                    index += 1;
                }
                continue;
            }

            let col_name = self.infer_column_name(&child_fields, alias_idx, expr_idx);
            let origin = self.trace_expr_origin(&child_fields, expr_idx);

            result.push(ColumnLineage {
                name: col_name.unwrap_or_default(),
                index,
                origin,
            });
            index += 1;
        }

        Some(result)
    }

    fn expand_star(&self) -> Vec<ColumnLineage> {
        let mut result = Vec::new();
        for (source_name, info) in &self.sources {
            if let Some(cols) = &info.columns {
                for col in cols {
                    let origin = self.trace_column(source_name, col);
                    result.push(ColumnLineage {
                        name: col.to_ascii_lowercase(),
                        index: 0, // will be set by caller
                        origin,
                    });
                }
            }
        }
        result
    }

    fn infer_column_name(
        &self,
        child_fields: &syntaqlite_syntax::any::NodeFields<'_>,
        alias_idx: u8,
        expr_idx: u8,
    ) -> Option<String> {
        // Try alias first.
        if let FieldValue::NodeId(alias_id) = child_fields[alias_idx as usize]
            && !alias_id.is_null()
            && let Some((_, alias_fields)) = self.stmt.extract_fields(alias_id)
        {
            for j in 0..alias_fields.len() {
                if let FieldValue::Span(s) = alias_fields[j]
                    && !s.is_empty()
                {
                    return Some(s.to_ascii_lowercase());
                }
            }
        }

        // Try bare column ref.
        if let FieldValue::NodeId(expr_id) = child_fields[expr_idx as usize]
            && !expr_id.is_null()
            && let Some((expr_tag, expr_fields)) = self.stmt.extract_fields(expr_id)
        {
            let expr_role = self.role_for(expr_tag);

            if let SemanticRole::ColumnRef {
                column: col_idx, ..
            } = expr_role
                && let FieldValue::Span(col_span) = expr_fields[col_idx as usize]
                && !col_span.is_empty()
            {
                return Some(col_span.to_ascii_lowercase());
            }
        }

        // Fallback: use expression source text.
        if let FieldValue::NodeId(expr_id) = child_fields[expr_idx as usize]
            && !expr_id.is_null()
        {
            return super::super::catalog::expr_source_text(self.stmt, expr_id)
                .map(str::to_ascii_lowercase);
        }

        None
    }

    fn trace_expr_origin(
        &self,
        child_fields: &syntaqlite_syntax::any::NodeFields<'_>,
        expr_idx: u8,
    ) -> Option<ColumnOrigin> {
        let FieldValue::NodeId(expr_id) = child_fields[expr_idx as usize] else {
            return None;
        };
        if expr_id.is_null() {
            return None;
        }

        let (expr_tag, expr_fields) = self.stmt.extract_fields(expr_id)?;
        let expr_role = self.role_for(expr_tag);

        let SemanticRole::ColumnRef {
            column: col_idx,
            table: tbl_idx,
        } = expr_role
        else {
            return None;
        };

        let col_name = if let FieldValue::Span(s) = expr_fields[col_idx as usize] {
            if s.is_empty() {
                return None;
            }
            s.to_ascii_lowercase()
        } else {
            return None;
        };

        let source_name = if let FieldValue::Span(s) = expr_fields[tbl_idx as usize]
            && !s.is_empty()
        {
            s.to_ascii_lowercase()
        } else {
            self.find_source_for_column(&col_name)?
        };

        self.trace_column(&source_name, &col_name)
    }

    fn find_source_for_column(&self, col_name: &str) -> Option<String> {
        let col_lower = col_name.to_ascii_lowercase();
        for (source_name, info) in &self.sources {
            if let Some(cols) = &info.columns
                && cols.iter().any(|c| c.eq_ignore_ascii_case(&col_lower))
            {
                return Some(source_name.clone());
            }
        }
        self.sources.keys().next().cloned()
    }

    /// Trace a column reference through CTEs/subqueries to a physical table.
    fn trace_column(&self, source_name: &str, col_name: &str) -> Option<ColumnOrigin> {
        let source_lower = self
            .alias_to_table
            .get(&source_name.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| source_name.to_ascii_lowercase());

        if let Some(&cte_body) = self.ctes.get(&source_lower) {
            return self.trace_through_select(cte_body, col_name);
        }

        if let Some(&sub_body) = self.subqueries.get(&source_lower) {
            return self.trace_through_select(sub_body, col_name);
        }

        if self.catalog.resolve_relation(&source_lower) {
            if self.catalog.is_view(&source_lower) {
                return None;
            }
            return Some(ColumnOrigin {
                table: source_lower,
                column: col_name.to_ascii_lowercase(),
            });
        }

        None
    }

    fn trace_through_select(&self, select_id: AnyNodeId, col_name: &str) -> Option<ColumnOrigin> {
        if select_id.is_null() {
            return None;
        }

        let actual_select = self.find_select_node(select_id)?;
        let (tag, fields) = self.stmt.extract_fields(actual_select)?;
        let role = self.role_for(tag);

        let SemanticRole::Query {
            from,
            columns: cols_idx,
            ..
        } = role
        else {
            return None;
        };

        let mut inner_sources: HashMap<String, SourceInfo> = HashMap::new();
        if let FieldValue::NodeId(from_id) = fields[from as usize] {
            self.collect_inner_from_sources(from_id, &mut inner_sources);
        } else {
            return None;
        }

        let FieldValue::NodeId(list_id) = fields[cols_idx as usize] else {
            return None;
        };
        if list_id.is_null() {
            return None;
        }

        let children = self.stmt.list_children(list_id)?;
        for &child_id in children {
            if child_id.is_null() {
                continue;
            }
            let (child_tag, child_fields) = self.stmt.extract_fields(child_id)?;
            let child_role = self.role_for(child_tag);

            let SemanticRole::ResultColumn {
                alias: alias_idx,
                expr: expr_idx,
                ..
            } = child_role
            else {
                continue;
            };

            let name = self.infer_column_name(&child_fields, alias_idx, expr_idx)?;
            if !name.eq_ignore_ascii_case(col_name) {
                continue;
            }

            let FieldValue::NodeId(expr_id) = child_fields[expr_idx as usize] else {
                return None;
            };
            if expr_id.is_null() {
                return None;
            }

            let (expr_tag, expr_fields) = self.stmt.extract_fields(expr_id)?;
            let expr_role = self.role_for(expr_tag);

            if let SemanticRole::ColumnRef {
                column: col_idx,
                table: tbl_idx,
            } = expr_role
            {
                let inner_col = if let FieldValue::Span(s) = expr_fields[col_idx as usize] {
                    if s.is_empty() {
                        return None;
                    }
                    s.to_ascii_lowercase()
                } else {
                    return None;
                };

                let inner_source = if let FieldValue::Span(s) = expr_fields[tbl_idx as usize]
                    && !s.is_empty()
                {
                    s.to_ascii_lowercase()
                } else {
                    find_source_in_map(&inner_sources, &inner_col)?
                };

                return self.trace_column(&inner_source, &inner_col);
            }

            return None;
        }

        None
    }

    fn find_select_node(&self, node_id: AnyNodeId) -> Option<AnyNodeId> {
        if node_id.is_null() {
            return None;
        }
        let (tag, fields) = self.stmt.extract_fields(node_id)?;
        let role = self.role_for(tag);

        match role {
            SemanticRole::Query { .. } => Some(node_id),
            SemanticRole::CteScope { body, .. } => {
                if let FieldValue::NodeId(body_id) = fields[body as usize] {
                    self.find_select_node(body_id)
                } else {
                    None
                }
            }
            SemanticRole::Transparent => {
                for child_id in self.stmt.child_node_ids(node_id) {
                    if let Some(result) = self.find_select_node(child_id) {
                        return Some(result);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn collect_inner_from_sources(
        &self,
        from_id: AnyNodeId,
        sources: &mut HashMap<String, SourceInfo>,
    ) {
        if from_id.is_null() {
            return;
        }

        let Some((tag, fields)) = self.stmt.extract_fields(from_id) else {
            return;
        };
        let role = self.role_for(tag);

        match role {
            SemanticRole::SourceRef { name, alias, .. } => {
                if let Some(ref sn) = self.span_text_from_field(&fields, name) {
                    let alias_name = self.span_text_from_field(&fields, alias);
                    let display = alias_name.as_ref().unwrap_or(sn).to_ascii_lowercase();
                    let canonical = sn.to_ascii_lowercase();
                    let (cols, _) = self.catalog.table_source_info(&canonical);
                    sources.insert(display, SourceInfo { columns: cols });
                }
            }
            _ => {
                for child_id in self.stmt.child_node_ids(from_id) {
                    self.collect_inner_from_sources(child_id, sources);
                }
            }
        }
    }

    fn resolve_tables_for_source(&mut self, source_name: &str) {
        let source_lower = self
            .alias_to_table
            .get(&source_name.to_ascii_lowercase())
            .cloned()
            .unwrap_or_else(|| source_name.to_ascii_lowercase());

        if let Some(&cte_body) = self.ctes.get(&source_lower) {
            self.collect_tables_from_select(cte_body);
        } else if let Some(&sub_body) = self.subqueries.get(&source_lower) {
            self.collect_tables_from_select(sub_body);
        } else if self.catalog.resolve_relation(&source_lower) {
            if self.catalog.is_view(&source_lower) {
                self.complete = false;
            }
            self.tables.push(TableAccess { name: source_lower });
        }
    }

    fn collect_tables_from_select(&mut self, node_id: AnyNodeId) {
        if node_id.is_null() {
            return;
        }
        let Some(actual_select) = self.find_select_node(node_id) else {
            return;
        };
        let Some((tag, fields)) = self.stmt.extract_fields(actual_select) else {
            return;
        };
        let role = self.role_for(tag);

        let SemanticRole::Query { from, .. } = role else {
            return;
        };

        if let FieldValue::NodeId(from_id) = fields[from as usize] {
            self.collect_tables_from_from(from_id);
        }
    }

    fn collect_tables_from_from(&mut self, from_id: AnyNodeId) {
        if from_id.is_null() {
            return;
        }

        let Some((tag, fields)) = self.stmt.extract_fields(from_id) else {
            return;
        };
        let role = self.role_for(tag);

        match role {
            SemanticRole::SourceRef { name, .. } => {
                if let Some(sn) = self.span_text_from_field(&fields, name) {
                    let canonical = sn.to_ascii_lowercase();
                    if self.ctes.contains_key(&canonical) {
                        self.collect_tables_from_select(self.ctes[&canonical]);
                    } else if self.catalog.resolve_relation(&canonical) {
                        if self.catalog.is_view(&canonical) {
                            self.complete = false;
                        }
                        self.tables.push(TableAccess { name: canonical });
                    }
                }
            }
            SemanticRole::ScopedSource { body, .. } => {
                if let FieldValue::NodeId(body_id) = fields[body as usize] {
                    self.collect_tables_from_select(body_id);
                }
            }
            _ => {
                for child_id in self.stmt.child_node_ids(from_id) {
                    self.collect_tables_from_from(child_id);
                }
            }
        }
    }

    // -- Helpers --

    fn role_for(&self, tag: syntaqlite_syntax::any::AnyNodeTag) -> SemanticRole {
        self.roles
            .get(u32::from(tag) as usize)
            .copied()
            .unwrap_or(SemanticRole::Transparent)
    }

    fn span_text(&self, node_id: AnyNodeId) -> Option<String> {
        if node_id.is_null() {
            return None;
        }
        let (_, fields) = self.stmt.extract_fields(node_id)?;
        for i in 0..fields.len() {
            if let FieldValue::Span(s) = fields[i]
                && !s.is_empty()
            {
                return Some(s.to_owned());
            }
        }
        None
    }

    fn span_text_from_field(
        &self,
        fields: &syntaqlite_syntax::any::NodeFields<'_>,
        field_idx: u8,
    ) -> Option<String> {
        if field_idx == crate::dialect::FIELD_ABSENT {
            return None;
        }
        match fields[field_idx as usize] {
            FieldValue::Span(s) if !s.is_empty() => Some(s.to_owned()),
            FieldValue::NodeId(id) if !id.is_null() => self.span_text(id),
            _ => None,
        }
    }
}

fn find_source_in_map(sources: &HashMap<String, SourceInfo>, col_name: &str) -> Option<String> {
    let col_lower = col_name.to_ascii_lowercase();
    for (source_name, info) in sources {
        if let Some(cols) = &info.columns
            && cols.iter().any(|c| c.eq_ignore_ascii_case(&col_lower))
        {
            return Some(source_name.clone());
        }
    }
    sources.keys().next().cloned()
}
