// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Single-pass semantic analysis engine.

use std::collections::HashSet;

use syntaqlite_syntax::any::{AnyNodeId, AnyParsedStatement, FieldValue, NodeFields};
use syntaqlite_syntax::typed::TypedParser;
use syntaqlite_syntax::{ParseOutcome, ParserConfig, TokenType};

use crate::dialect::Dialect;
use crate::dialect::SemanticRole;

use super::ValidationConfig;
use super::catalog::{
    Catalog, CatalogLayer, ColumnResolution, FunctionCheckResult, columns_from_select,
};
use super::diagnostics::{Diagnostic, DiagnosticMessage, Help, Severity};
use super::fuzzy::best_suggestion;
use super::model::{
    CompletionContext, CompletionInfo, SemanticModel, SemanticToken, StoredComment, StoredToken,
};

/// Long-lived semantic analysis engine.
///
/// Create once for a dialect and reuse across inputs. The dialect layer is
/// built at construction and never changes. The database and document layers
/// are reset on each [`analyze`](Self::analyze) call.
pub struct SemanticAnalyzer {
    dialect: Dialect,
    catalog: Catalog,
}

#[expect(dead_code)]
impl SemanticAnalyzer {
    /// Create an analyzer for the built-in `SQLite` dialect.
    #[cfg(feature = "sqlite")]
    pub fn new() -> Self {
        Self::with_dialect(crate::sqlite::dialect::dialect())
    }

    /// Create an analyzer bound to a specific dialect.
    pub fn with_dialect(dialect: impl Into<Dialect>) -> Self {
        let dialect = dialect.into();
        SemanticAnalyzer {
            catalog: Catalog::new(dialect.clone()),
            dialect,
        }
    }

    /// Return the dialect this analyzer was constructed for.
    pub(crate) fn dialect(&self) -> Dialect {
        self.dialect.clone()
    }

    /// Run a complete single-pass analysis: parse, collect tokens, walk AST.
    ///
    /// `user_catalog` supplies the database layer (user-provided schema). Its
    /// database layer is merged into the analyzer's catalog for this pass only.
    /// The document layer is cleared and rebuilt statement-by-statement so that
    /// DDL seen earlier in the file is visible to queries that follow it.
    pub fn analyze(
        &mut self,
        source: &str,
        user_catalog: &Catalog,
        config: &ValidationConfig,
    ) -> SemanticModel {
        self.catalog.new_document();
        self.catalog.copy_schema_layers_from(user_catalog);
        self.analyze_inner(source, config)
    }

    /// Semantic tokens for syntax highlighting, derived from a prior
    /// [`analyze`](Self::analyze) result.
    pub(crate) fn semantic_tokens(&self, model: &SemanticModel) -> Vec<SemanticToken> {
        use syntaqlite_syntax::any::TokenCategory;

        let mut out = Vec::new();
        for t in &model.tokens {
            let cat = self.dialect.classify_token(t.token_type.into(), t.flags);
            if cat != TokenCategory::Other {
                out.push(SemanticToken {
                    offset: t.offset,
                    length: t.length,
                    category: cat,
                });
            }
        }
        for c in &model.comments {
            out.push(SemanticToken {
                offset: c.offset,
                length: c.length,
                category: TokenCategory::Comment,
            });
        }
        out.sort_by_key(|t| t.offset);
        out
    }

    /// Expected tokens and semantic context at `offset` (for completion).
    #[cfg(feature = "sqlite")]
    #[expect(clippy::unused_self)]
    pub(crate) fn completion_info(&self, model: &SemanticModel, offset: usize) -> CompletionInfo {
        let source = model.source();
        let tokens = &model.tokens;
        let cursor = offset.min(source.len());
        let (boundary, backtracked) = completion_boundary(source, tokens, cursor);
        let start = statement_token_start(tokens, boundary);
        let stmt_tokens = &tokens[start..boundary];

        let parser = TypedParser::new(syntaqlite_syntax::typed::grammar());
        let mut cursor_p = parser.incremental_parse(source);
        // Do not call expected_tokens() before feeding any tokens: the C parser
        // returns a garbage `total` count when no tokens have been fed yet,
        // which would trigger a multi-GiB allocation and SIGKILL.
        let mut last_expected: Vec<TokenType> = Vec::new();

        for tok in stmt_tokens {
            let span = tok.offset..(tok.offset + tok.length);
            if cursor_p.feed_token(tok.token_type, span).is_some() {
                return CompletionInfo {
                    tokens: last_expected,
                    context: CompletionContext::from_parser(cursor_p.completion_context()),
                };
            }
            last_expected.clear();
            last_expected.extend(cursor_p.expected_tokens());
        }

        let context = CompletionContext::from_parser(cursor_p.completion_context());

        if backtracked && let Some(extra) = tokens.get(boundary) {
            let span = extra.offset..(extra.offset + extra.length);
            if cursor_p.feed_token(extra.token_type, span).is_none() {
                merge_expected_tokens(
                    &mut last_expected,
                    cursor_p.expected_tokens().collect::<Vec<TokenType>>(),
                );
            }
        }

        CompletionInfo {
            tokens: last_expected,
            context,
        }
    }

    // ── Private ───────────────────────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    fn analyze_inner(&mut self, source: &str, config: &ValidationConfig) -> SemanticModel {
        let parser = syntaqlite_syntax::Parser::with_config(
            &ParserConfig::default().with_collect_tokens(true),
        );
        let mut session = parser.parse(source);

        let mut tokens: Vec<StoredToken> = Vec::new();
        let mut comments: Vec<StoredComment> = Vec::new();
        let mut diagnostics: Vec<Diagnostic> = Vec::new();

        loop {
            let stmt = match session.next() {
                ParseOutcome::Done => break,
                ParseOutcome::Ok(s) => s,
                ParseOutcome::Err(e) => {
                    let (start, end) = parse_error_span(&e, source);
                    diagnostics.push(Diagnostic {
                        start_offset: start,
                        end_offset: end,
                        message: DiagnosticMessage::Other(e.message().to_owned()),
                        severity: Severity::Error,
                        help: None,
                    });
                    // Collect tokens from the partial parse so completion_info
                    // can replay them through the incremental parser.
                    for tok in e.tokens() {
                        tokens.push(StoredToken {
                            offset: tok.offset() as usize,
                            length: tok.length() as usize,
                            token_type: tok.token_type(),
                            flags: tok.flags(),
                        });
                    }
                    for c in e.comments() {
                        comments.push(StoredComment {
                            offset: c.offset() as usize,
                            length: c.length() as usize,
                        });
                    }
                    continue;
                }
            };

            // Collect token and comment positions for semantic highlighting.
            for tok in stmt.tokens() {
                tokens.push(StoredToken {
                    offset: tok.offset() as usize,
                    length: tok.length() as usize,
                    token_type: tok.token_type(),
                    flags: tok.flags(),
                });
            }
            for c in stmt.comments() {
                comments.push(StoredComment {
                    offset: c.offset() as usize,
                    length: c.length() as usize,
                });
            }

            // Semantic walk.
            let root = stmt.root();
            let root_id: AnyNodeId = root.node_id().into();
            let erased = stmt.erase();

            self.catalog.accumulate_ddl(
                CatalogLayer::Document,
                erased,
                root_id,
                self.dialect.clone(),
            );

            ValidationPass::run(
                erased,
                root_id,
                self.dialect.clone(),
                &mut self.catalog,
                config,
                &mut diagnostics,
            );
        }

        SemanticModel {
            source: source.to_owned(),
            tokens,
            comments,
            diagnostics,
        }
    }
}

#[cfg(feature = "sqlite")]
impl Default for SemanticAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

#[cfg(feature = "sqlite")]
fn parse_error_span(err: &syntaqlite_syntax::ParseError<'_>, source: &str) -> (usize, usize) {
    match (err.offset(), err.length()) {
        (Some(off), Some(len)) if len > 0 => (off, off + len),
        (Some(off), _) => {
            if off >= source.len() && !source.is_empty() {
                (source.len() - 1, source.len())
            } else {
                (off, (off + 1).min(source.len()))
            }
        }
        _ => {
            let end = source.len();
            let start = if end > 0 { end - 1 } else { 0 };
            (start, end)
        }
    }
}

#[cfg(feature = "sqlite")]
fn completion_boundary(
    source: &str,
    tokens: &[StoredToken],
    cursor_offset: usize,
) -> (usize, bool) {
    let mut boundary = tokens.partition_point(|t| t.offset + t.length <= cursor_offset);

    while boundary > 0 {
        let tok = &tokens[boundary - 1];
        if tok.length == 0 && tok.offset == cursor_offset {
            boundary -= 1;
        } else {
            break;
        }
    }

    let mut backtracked = false;
    if boundary > 0
        && tokens[boundary - 1].offset + tokens[boundary - 1].length == cursor_offset
        && cursor_offset > 0
    {
        let prev = source.as_bytes()[cursor_offset - 1];
        if prev.is_ascii_alphanumeric() || prev == b'_' {
            boundary -= 1;
            backtracked = true;
        }
    }
    (boundary, backtracked)
}

#[cfg(feature = "sqlite")]
fn statement_token_start(tokens: &[StoredToken], boundary: usize) -> usize {
    tokens[..boundary]
        .iter()
        .rposition(|t| t.token_type == TokenType::Semi)
        .map_or(0, |idx| idx + 1)
}

#[cfg(feature = "sqlite")]
fn merge_expected_tokens(into: &mut Vec<TokenType>, extra: Vec<TokenType>) {
    let mut seen: HashSet<TokenType> = into.iter().copied().collect();
    for token in extra {
        if seen.insert(token) {
            into.push(token);
        }
    }
}

// ── ValidationPass ────────────────────────────────────────────────────────────

/// Per-statement validation pass.  Reads the dialect's [`SemanticRole`] table
/// and dispatches node visits to role-specific handlers.
struct ValidationPass<'a> {
    roles: &'static [SemanticRole],
    source_start: usize,
    catalog: &'a mut Catalog,
    config: &'a ValidationConfig,
    diagnostics: &'a mut Vec<Diagnostic>,
}

impl<'a> ValidationPass<'a> {
    fn run(
        stmt: AnyParsedStatement<'a>,
        root: AnyNodeId,
        dialect: Dialect,
        catalog: &'a mut Catalog,
        config: &'a ValidationConfig,
        diagnostics: &'a mut Vec<Diagnostic>,
    ) {
        let roles = dialect.roles();
        let source_start = stmt.source().as_ptr() as usize;
        let mut pass = ValidationPass {
            roles,
            source_start,
            catalog,
            config,
            diagnostics,
        };
        pass.visit(&stmt, root);
    }

    // ── Core visitor ─────────────────────────────────────────────────────────

    fn visit(&mut self, stmt: &AnyParsedStatement<'a>, node_id: AnyNodeId) {
        if node_id.is_null() {
            return;
        }
        // List nodes have no role — visit their elements directly.
        if let Some(children) = stmt.list_children(node_id) {
            for child in children.iter().copied() {
                if !child.is_null() {
                    self.visit(stmt, child);
                }
            }
            return;
        }
        let Some((tag, fields)) = stmt.extract_fields(node_id) else {
            return;
        };
        let idx = u32::from(tag) as usize;
        let role = self
            .roles
            .get(idx)
            .copied()
            .unwrap_or(SemanticRole::Transparent);

        match role {
            // Catalog roles are handled in the accumulation pass; skip here.
            SemanticRole::DefineTable { .. }
            | SemanticRole::DefineView { .. }
            | SemanticRole::DefineFunction { .. }
            | SemanticRole::ReturnSpec { .. }
            | SemanticRole::Import { .. } => {}

            // Transparent: recurse into children without special handling.
            // ColumnDef and ResultColumn have no validation logic yet — child
            // expressions are reached via transparent traversal.
            SemanticRole::Transparent
            | SemanticRole::ColumnDef { .. }
            | SemanticRole::ResultColumn { .. }
            | SemanticRole::CteBinding { .. } => self.visit_children(stmt, node_id),

            SemanticRole::Call { name, args } => {
                self.visit_call(stmt, node_id, &fields, name, args);
            }
            SemanticRole::ColumnRef { column, table } => {
                self.visit_column_ref(&fields, column, table);
            }
            SemanticRole::SourceRef { name, alias, .. } => {
                self.visit_source_ref(stmt, &fields, name, alias);
            }
            SemanticRole::ScopedSource { body, alias } => {
                self.visit_scoped_source(stmt, &fields, body, alias);
            }
            SemanticRole::Query {
                from,
                columns,
                where_clause,
                groupby,
                having,
                orderby,
                limit_clause,
            } => self.visit_query(
                stmt,
                &fields,
                from,
                columns,
                where_clause,
                groupby,
                having,
                orderby,
                limit_clause,
            ),
            SemanticRole::CteScope {
                recursive,
                bindings,
                body,
            } => self.visit_cte_scope(stmt, &fields, recursive, bindings, body),
            SemanticRole::TriggerScope {
                target: _,
                when,
                body,
            } => {
                self.visit_trigger_scope(stmt, &fields, when, body);
            }
        }
    }

    fn visit_children(&mut self, stmt: &AnyParsedStatement<'a>, node_id: AnyNodeId) {
        for child in stmt.child_node_ids(node_id) {
            if !child.is_null() {
                self.visit(stmt, child);
            }
        }
    }

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn span_offset(&self, s: &str) -> usize {
        s.as_ptr() as usize - self.source_start
    }

    fn field_node_id(fields: &NodeFields<'_>, idx: u8) -> Option<AnyNodeId> {
        match fields[idx as usize] {
            FieldValue::NodeId(id) if !id.is_null() => Some(id),
            _ => None,
        }
    }

    fn visit_opt(&mut self, stmt: &AnyParsedStatement<'a>, id: Option<AnyNodeId>) {
        if let Some(id) = id {
            self.visit(stmt, id);
        }
    }

    /// Extract source text from a `Name` node (`IdentName` or `Error`).
    /// Both node kinds store their span at field 0.
    #[expect(clippy::unused_self)]
    fn name_text(&self, stmt: &AnyParsedStatement<'a>, node_id: Option<AnyNodeId>) -> &'a str {
        let Some(node_id) = node_id else {
            return "";
        };
        let Some((_, fields)) = stmt.extract_fields(node_id) else {
            return "";
        };
        if fields.is_empty() {
            return "";
        }
        match fields[0] {
            FieldValue::Span(s) => s,
            _ => "",
        }
    }

    // ── Role handlers ─────────────────────────────────────────────────────────

    fn visit_source_ref(
        &mut self,
        stmt: &AnyParsedStatement<'a>,
        fields: &NodeFields<'a>,
        name_idx: u8,
        alias_idx: u8,
    ) {
        let FieldValue::Span(name) = fields[name_idx as usize] else {
            return;
        };
        if name.is_empty() {
            return;
        }
        let offset = self.span_offset(name);

        let is_known =
            self.catalog.resolve_relation(name) || self.catalog.resolve_table_function(name);
        if !is_known {
            let mut candidates = self.catalog.all_relation_names();
            candidates.extend(self.catalog.all_table_function_names());
            let suggestion = best_suggestion(name, &candidates, self.config.suggestion_threshold);
            self.diagnostics.push(Diagnostic {
                start_offset: offset,
                end_offset: offset + name.len(),
                message: DiagnosticMessage::UnknownTable {
                    name: name.to_string(),
                },
                severity: self.config.severity(),
                help: suggestion.map(Help::Suggestion),
            });
        }

        let alias = self.name_text(stmt, Self::field_node_id(fields, alias_idx));
        let scope_name = if alias.is_empty() { name } else { alias };
        let columns = self.catalog.columns_for_table_source(name);
        self.catalog.add_query_table(scope_name, columns);
    }

    fn visit_call(
        &mut self,
        stmt: &AnyParsedStatement<'a>,
        node_id: AnyNodeId,
        fields: &NodeFields<'a>,
        name_idx: u8,
        args_idx: u8,
    ) {
        if let FieldValue::Span(name) = fields[name_idx as usize]
            && !name.is_empty()
        {
            let offset = self.span_offset(name);
            let args_id = Self::field_node_id(fields, args_idx);
            let arg_count = args_id
                .and_then(|id| stmt.list_children(id))
                .map_or(0, <[_]>::len);
            match self.catalog.check_function(name, arg_count) {
                FunctionCheckResult::Ok => {}
                FunctionCheckResult::Unknown => {
                    let candidates = self.catalog.all_function_names();
                    let suggestion =
                        best_suggestion(name, &candidates, self.config.suggestion_threshold);
                    self.diagnostics.push(Diagnostic {
                        start_offset: offset,
                        end_offset: offset + name.len(),
                        message: DiagnosticMessage::UnknownFunction {
                            name: name.to_string(),
                        },
                        severity: self.config.severity(),
                        help: suggestion.map(Help::Suggestion),
                    });
                }
                FunctionCheckResult::WrongArity { expected } => {
                    self.diagnostics.push(Diagnostic {
                        start_offset: offset,
                        end_offset: offset + name.len(),
                        message: DiagnosticMessage::FunctionArity {
                            name: name.to_string(),
                            expected,
                            got: arg_count,
                        },
                        severity: self.config.severity(),
                        help: None,
                    });
                }
            }
        }
        self.visit_children(stmt, node_id);
    }

    fn visit_column_ref(&mut self, fields: &NodeFields<'a>, column_idx: u8, table_idx: u8) {
        let FieldValue::Span(column) = fields[column_idx as usize] else {
            return;
        };
        if column.is_empty() {
            return;
        }
        let table = match fields[table_idx as usize] {
            FieldValue::Span(s) if !s.is_empty() => Some(s),
            _ => None,
        };
        let offset = self.span_offset(column);

        match self.catalog.resolve_column(table, column) {
            ColumnResolution::Found | ColumnResolution::TableNotFound => {}
            ColumnResolution::TableFoundColumnMissing => {
                let tbl = table.expect("qualifier present when TableFoundColumnMissing");
                let candidates = self.catalog.all_column_names(Some(tbl));
                let suggestion =
                    best_suggestion(column, &candidates, self.config.suggestion_threshold);
                self.diagnostics.push(Diagnostic {
                    start_offset: offset,
                    end_offset: offset + column.len(),
                    message: DiagnosticMessage::UnknownColumn {
                        column: column.to_string(),
                        table: Some(tbl.to_string()),
                    },
                    severity: self.config.severity(),
                    help: suggestion.map(Help::Suggestion),
                });
            }
            ColumnResolution::NotFound => {
                let candidates = self.catalog.all_column_names(None);
                let suggestion =
                    best_suggestion(column, &candidates, self.config.suggestion_threshold);
                self.diagnostics.push(Diagnostic {
                    start_offset: offset,
                    end_offset: offset + column.len(),
                    message: DiagnosticMessage::UnknownColumn {
                        column: column.to_string(),
                        table: None,
                    },
                    severity: self.config.severity(),
                    help: suggestion.map(Help::Suggestion),
                });
            }
        }
    }

    fn visit_scoped_source(
        &mut self,
        stmt: &AnyParsedStatement<'a>,
        fields: &NodeFields<'a>,
        body_idx: u8,
        alias_idx: u8,
    ) {
        self.catalog.push_query_scope();
        self.visit_opt(stmt, Self::field_node_id(fields, body_idx));
        self.catalog.pop_query_scope();

        let alias = self.name_text(stmt, Self::field_node_id(fields, alias_idx));
        if !alias.is_empty() {
            self.catalog.add_query_table(alias, None);
        }
    }

    #[expect(clippy::too_many_arguments)]
    fn visit_query(
        &mut self,
        stmt: &AnyParsedStatement<'a>,
        fields: &NodeFields<'a>,
        from: u8,
        columns: u8,
        where_clause: u8,
        groupby: u8,
        having: u8,
        orderby: u8,
        limit_clause: u8,
    ) {
        self.visit_opt(stmt, Self::field_node_id(fields, from));
        for idx in [
            columns,
            where_clause,
            groupby,
            having,
            orderby,
            limit_clause,
        ] {
            self.visit_opt(stmt, Self::field_node_id(fields, idx));
        }
    }

    fn visit_cte_scope(
        &mut self,
        stmt: &AnyParsedStatement<'a>,
        fields: &NodeFields<'a>,
        recursive_idx: u8,
        bindings_idx: u8,
        body_idx: u8,
    ) {
        let is_recursive = matches!(fields[recursive_idx as usize], FieldValue::Bool(true));
        let cte_ids = Self::field_node_id(fields, bindings_idx)
            .and_then(|id| stmt.list_children(id))
            .unwrap_or(&[]);

        // Push a scope for the whole WITH clause so CTE names registered via
        // add_query_table have somewhere to live (the stack may be empty at the
        // top level).
        self.catalog.push_query_scope();

        for cte_id in cte_ids.iter().copied() {
            if cte_id.is_null() {
                continue;
            }
            let Some((cte_tag, cte_fields)) = stmt.extract_fields(cte_id) else {
                continue;
            };
            let cte_tag_idx = u32::from(cte_tag) as usize;
            let cte_role = self
                .roles
                .get(cte_tag_idx)
                .copied()
                .unwrap_or(SemanticRole::Transparent);

            let SemanticRole::CteBinding {
                name: cte_name_idx,
                columns: columns_field_idx,
                body: cte_body_idx,
            } = cte_role
            else {
                continue;
            };

            let cte_name = match cte_fields[cte_name_idx as usize] {
                FieldValue::Span(s) => s,
                _ => "",
            };
            let cte_body_id = Self::field_node_id(&cte_fields, cte_body_idx);

            // Extract declared column names (if a column list is present).
            let declared_cols: Option<Vec<&'a str>> = columns_field_idx.and_then(|cidx| {
                let list_id = Self::field_node_id(&cte_fields, cidx)?;
                let children = stmt.list_children(list_id)?;
                let names: Vec<&'a str> = children
                    .iter()
                    .copied()
                    .filter(|id| !id.is_null())
                    .map(|id| self.name_text(stmt, Some(id)))
                    .filter(|s| !s.is_empty())
                    .collect();
                if names.is_empty() { None } else { Some(names) }
            });

            // For recursive CTEs, register the name before visiting the body.
            if is_recursive && !cte_name.is_empty() {
                let cols = declared_cols
                    .as_ref()
                    .map(|v| v.iter().map(ToString::to_string).collect());
                self.catalog.add_query_table(cte_name, cols);
            }

            self.catalog.push_query_scope();
            self.visit_opt(stmt, cte_body_id);
            self.catalog.pop_query_scope();

            if !cte_name.is_empty() {
                if let Some(ref declared) = declared_cols {
                    // Count result columns; emit diagnostic on mismatch.
                    if let Some(actual) = self.count_result_columns(stmt, cte_body_id)
                        && actual != declared.len()
                    {
                        let offset = self.span_offset(cte_name);
                        self.diagnostics.push(Diagnostic {
                            start_offset: offset,
                            end_offset: offset + cte_name.len(),
                            message: DiagnosticMessage::CteColumnCountMismatch {
                                name: cte_name.to_string(),
                                declared: declared.len(),
                                actual,
                            },
                            severity: Severity::Error,
                            help: None,
                        });
                    }
                    // Register with declared column names regardless of count.
                    let cols = declared.iter().map(ToString::to_string).collect();
                    self.catalog.add_query_table(cte_name, Some(cols));
                } else {
                    // No declared column list: infer names from SELECT result columns.
                    let inferred =
                        cte_body_id.and_then(|id| columns_from_select(*stmt, id, self.roles));
                    self.catalog.add_query_table(cte_name, inferred);
                }
            }
        }

        self.visit_opt(stmt, Self::field_node_id(fields, body_idx));
        self.catalog.pop_query_scope();
    }

    /// Count the result columns of a SELECT body node.
    ///
    /// Returns `None` if the body is not a plain `SelectStmt` or if any result
    /// column uses `*` (wildcard), which would require catalog expansion to count.
    fn count_result_columns(
        &self,
        stmt: &AnyParsedStatement<'a>,
        body_id: Option<AnyNodeId>,
    ) -> Option<usize> {
        let body_id = body_id?;
        let (body_tag, body_fields) = stmt.extract_fields(body_id)?;
        let body_role = self
            .roles
            .get(u32::from(body_tag) as usize)
            .copied()
            .unwrap_or(SemanticRole::Transparent);

        // Only handle direct SelectStmt (Query role); skip compound, VALUES, etc.
        let SemanticRole::Query {
            columns: cols_idx, ..
        } = body_role
        else {
            return None;
        };

        let list_id = Self::field_node_id(&body_fields, cols_idx)?;
        let children = stmt.list_children(list_id)?;

        let mut count = 0usize;
        for child_id in children.iter().copied() {
            if child_id.is_null() {
                continue;
            }
            let Some((child_tag, child_fields)) = stmt.extract_fields(child_id) else {
                continue;
            };
            let child_role = self
                .roles
                .get(u32::from(child_tag) as usize)
                .copied()
                .unwrap_or(SemanticRole::Transparent);
            let SemanticRole::ResultColumn {
                flags: flags_idx, ..
            } = child_role
            else {
                continue;
            };
            // STAR flag (bit 0) means wildcard — skip count check entirely.
            if let FieldValue::Flags(f) = child_fields[flags_idx as usize]
                && f & 1 != 0
            {
                return None;
            }
            count += 1;
        }
        Some(count)
    }

    fn visit_trigger_scope(
        &mut self,
        stmt: &AnyParsedStatement<'a>,
        fields: &NodeFields<'a>,
        when_idx: u8,
        body_idx: u8,
    ) {
        self.catalog.push_query_scope();
        self.catalog.add_query_table("OLD", None);
        self.catalog.add_query_table("NEW", None);
        self.visit_opt(stmt, Self::field_node_id(fields, when_idx));
        self.visit_opt(stmt, Self::field_node_id(fields, body_idx));
        self.catalog.pop_query_scope();
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::super::catalog::{AritySpec, CatalogLayer, FunctionCategory, FunctionCheckResult};
    use super::super::diagnostics::{DiagnosticMessage, Help};
    use super::super::render::DiagnosticRenderer;
    use super::*;

    fn sqlite_analyzer() -> SemanticAnalyzer {
        SemanticAnalyzer::new()
    }

    fn sqlite_catalog() -> Catalog {
        Catalog::new(crate::sqlite::dialect::dialect())
    }

    fn strict() -> ValidationConfig {
        ValidationConfig {
            strict_schema: true,
            suggestion_threshold: 2,
        }
    }

    fn lenient() -> ValidationConfig {
        ValidationConfig::default()
    }

    // ── Catalog ────────────────────────────────────────────────────────────────

    #[test]
    fn catalog_add_table_and_resolve() {
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("users", Some(vec!["id".to_string(), "name".to_string()]));
        assert!(cat.resolve_relation("users"));
        assert!(cat.resolve_relation("USERS")); // case-insensitive
        assert!(!cat.resolve_relation("orders"));
    }

    #[test]
    fn catalog_add_view_and_resolve() {
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("active_users", Some(vec!["id".to_string()]));
        assert!(cat.resolve_relation("active_users"));
    }

    #[test]
    fn catalog_add_function_and_check() {
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_function_overload("my_func", FunctionCategory::Scalar, AritySpec::Exact(2));
        assert!(matches!(
            cat.check_function("my_func", 2),
            FunctionCheckResult::Ok
        ));
        assert!(matches!(
            cat.check_function("my_func", 1),
            FunctionCheckResult::WrongArity { .. }
        ));
    }

    #[test]
    fn catalog_add_variadic_function() {
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_function_overload("variadic_fn", FunctionCategory::Scalar, AritySpec::Any);
        assert!(matches!(
            cat.check_function("variadic_fn", 0),
            FunctionCheckResult::Ok
        ));
        assert!(matches!(
            cat.check_function("variadic_fn", 100),
            FunctionCheckResult::Ok
        ));
    }

    #[test]
    fn catalog_builtin_functions_resolved() {
        let cat = sqlite_catalog();
        // SQLite has built-in functions like abs(), coalesce(), etc.
        assert!(!matches!(
            cat.check_function("abs", 1),
            FunctionCheckResult::Unknown
        ));
        assert!(!matches!(
            cat.check_function("coalesce", 2),
            FunctionCheckResult::Unknown
        ));
    }

    #[test]
    fn catalog_from_ddl_populates_tables() {
        let dialect = crate::sqlite::dialect::dialect();
        let cat = Catalog::from_ddl(dialect, "CREATE TABLE users (id INTEGER, name TEXT);");
        assert!(cat.resolve_relation("users"));
    }

    #[test]
    fn catalog_from_ddl_populates_virtual_tables() {
        let dialect = crate::sqlite::dialect::dialect();
        let cat = Catalog::from_ddl(dialect, "CREATE VIRTUAL TABLE fts USING fts5(content);");
        assert!(cat.resolve_relation("fts"));
    }

    #[test]
    fn virtual_table_in_from_clause_no_error() {
        // Virtual tables are known to exist; column refs are conservatively
        // accepted because the column list is not statically known.
        let source = "CREATE VIRTUAL TABLE fts USING fts5(content);\nSELECT * FROM fts;";
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(source, &cat, &strict());
        let unknown_tables: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert!(
            unknown_tables.is_empty(),
            "virtual table should be known: {unknown_tables:?}"
        );
    }

    #[test]
    fn catalog_clear_database() {
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("tmp", Some(vec!["id".to_string()]));
        assert!(cat.resolve_relation("tmp"));
        cat.new_database();
        assert!(!cat.resolve_relation("tmp"));
    }

    // ── Analyzer: no-error cases ───────────────────────────────────────────────

    #[test]
    fn analyze_select_from_known_table_no_errors() {
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("users", Some(vec!["id".to_string(), "name".to_string()]));

        let model = az.analyze("SELECT id FROM users", &cat, &strict());
        let diags: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert!(diags.is_empty(), "unexpected table error: {diags:?}");
    }

    #[test]
    fn analyze_empty_source_no_errors() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze("", &cat, &strict());
        assert!(model.diagnostics().is_empty());
    }

    #[test]
    fn analyze_pragma_no_errors() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze("PRAGMA journal_mode;", &cat, &strict());
        let sem_errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| !d.message.is_parse_error())
            .collect();
        assert!(sem_errs.is_empty());
    }

    // ── Analyzer: unknown table / column ──────────────────────────────────────

    #[test]
    fn analyze_unknown_table_strict_is_error() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze("SELECT * FROM missing_table", &cat, &strict());
        let errs: Vec<_> = model.diagnostics().iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { name } if name == "missing_table"))
            .collect();
        assert_eq!(errs.len(), 1);
        assert_eq!(errs[0].severity, Severity::Error);
    }

    #[test]
    fn analyze_unknown_table_lenient_is_warning() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze("SELECT * FROM missing_table", &cat, &lenient());
        let warns: Vec<_> = model.diagnostics().iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { name } if name == "missing_table"))
            .collect();
        assert_eq!(warns.len(), 1);
        assert_eq!(warns[0].severity, Severity::Warning);
    }

    #[test]
    fn analyze_fuzzy_suggestion_for_unknown_table() {
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("users", Some(vec!["id".to_string()]));
        let model = az.analyze("SELECT * FROM usres", &cat, &strict()); // typo
        let diag = model.diagnostics().iter().find(
            |d| matches!(&d.message, DiagnosticMessage::UnknownTable { name } if name == "usres"),
        );
        assert!(diag.is_some(), "expected unknown-table diagnostic");
        let diag = diag.unwrap();
        assert!(
            matches!(&diag.help, Some(Help::Suggestion(s)) if s == "users"),
            "expected 'users' suggestion, got {:?}",
            diag.help
        );
    }

    // ── Analyzer: DDL accumulation ─────────────────────────────────────────────

    #[test]
    fn analyze_create_table_then_select_no_error() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let src = "CREATE TABLE t (id INTEGER); SELECT id FROM t;";
        let model = az.analyze(src, &cat, &strict());
        let unknown: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert!(
            unknown.is_empty(),
            "DDL-defined table not visible: {unknown:?}"
        );
    }

    #[test]
    fn analyze_create_view_then_select_no_error() {
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("users", Some(vec!["id".to_string()]));
        let src = "CREATE VIEW vw AS SELECT id FROM users; SELECT id FROM vw;";
        let model = az.analyze(src, &cat, &strict());
        let unknown: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert!(unknown.is_empty(), "VIEW not visible: {unknown:?}");
    }

    // ── Analyzer: DDL column extraction (role-based) ───────────────────────────

    #[test]
    fn ddl_columns_visible_to_column_ref() {
        // Verifies columns_from_column_list uses the ColumnDef role to extract
        // names precisely, and those names are visible to column-ref validation.
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let src = "CREATE TABLE t (id INTEGER, name TEXT); SELECT id FROM t;";
        let model = az.analyze(src, &cat, &strict());
        assert!(
            model.diagnostics().is_empty(),
            "id is a known column of t: {:?}",
            model.diagnostics()
        );
    }

    #[test]
    fn ddl_unknown_column_on_ddl_table_flagged() {
        // Verifies that column names extracted from DDL are used for validation:
        // a column not in the DDL definition is correctly flagged.
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let src = "CREATE TABLE t (id INTEGER); SELECT missing FROM t;";
        let model = az.analyze(src, &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { .. }))
            .collect();
        assert_eq!(
            errs.len(),
            1,
            "unknown column should be flagged: {:?}",
            model.diagnostics()
        );
    }

    // ── Analyzer: function validation ──────────────────────────────────────────

    #[test]
    fn analyze_unknown_function_flagged() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze("SELECT totally_unknown_fn(1)", &cat, &strict());
        let errs: Vec<_> = model.diagnostics().iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownFunction { name } if name == "totally_unknown_fn"))
            .collect();
        assert_eq!(errs.len(), 1);
    }

    #[test]
    fn analyze_builtin_abs_no_error() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze("SELECT abs(-1)", &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownFunction { .. }))
            .collect();
        assert!(errs.is_empty(), "abs() should be a known builtin: {errs:?}");
    }

    // ── Analyzer: multiple statements ─────────────────────────────────────────

    #[test]
    fn analyze_multiple_selects_independent() {
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("users", Some(vec!["id".to_string()]));
        let src = "SELECT id FROM users; SELECT id FROM users;";
        let model = az.analyze(src, &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert!(errs.is_empty());
    }

    #[test]
    fn analyze_reuse_clears_document_layer() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();

        // First pass: CREATE TABLE makes 't' visible.
        az.analyze(
            "CREATE TABLE t (id INTEGER); SELECT id FROM t;",
            &cat,
            &strict(),
        );

        // Second pass: 't' should NOT be visible — document layer was cleared.
        let model = az.analyze("SELECT id FROM t;", &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(
                |d| matches!(&d.message, DiagnosticMessage::UnknownTable { name } if name == "t"),
            )
            .collect();
        assert_eq!(
            errs.len(),
            1,
            "document layer should be cleared between passes"
        );
    }

    // ── Connection layer ───────────────────────────────────────────────────────

    #[test]
    fn catalog_connection_table_resolves() {
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Connection)
            .insert_relation("conn_tbl", Some(vec!["id".to_string()]));
        let model = sqlite_analyzer().analyze("SELECT id FROM conn_tbl", &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { name } if name == "conn_tbl"))
            .collect();
        assert!(
            errs.is_empty(),
            "connection table should be visible: {errs:?}"
        );
    }

    #[test]
    fn catalog_connection_shadows_database() {
        // "t" in database has only column "a"; in connection has only "b".
        // Querying "b" should succeed (connection takes priority, shadowing db entry).
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("t", Some(vec!["a".to_string()]));
        cat.layer_mut(CatalogLayer::Connection)
            .insert_relation("t", Some(vec!["b".to_string()]));
        let model = sqlite_analyzer().analyze("SELECT b FROM t", &cat, &strict());
        let col_errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { .. }))
            .collect();
        assert!(
            col_errs.is_empty(),
            "connection layer should shadow database layer: {col_errs:?}"
        );
    }

    #[test]
    fn catalog_document_shadows_connection() {
        // "t" in connection has only "a"; document DDL creates "t" with only "b".
        // Querying "b" should succeed.
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Connection)
            .insert_relation("t", Some(vec!["a".to_string()]));
        let model = sqlite_analyzer().analyze(
            "CREATE TABLE t (b INTEGER); SELECT b FROM t",
            &cat,
            &strict(),
        );
        let col_errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { .. }))
            .collect();
        assert!(
            col_errs.is_empty(),
            "document layer should shadow connection layer: {col_errs:?}"
        );
    }

    #[test]
    fn catalog_clear_connection() {
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Connection)
            .insert_relation("conn_tbl", Some(vec!["id".to_string()]));
        cat.new_connection();
        let model = sqlite_analyzer().analyze("SELECT id FROM conn_tbl", &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { name } if name == "conn_tbl"))
            .collect();
        assert_eq!(errs.len(), 1, "connection table should be gone after clear");
    }

    // ── DiagnosticRenderer ─────────────────────────────────────────────────────

    #[test]
    fn renderer_produces_output_for_error() {
        let source = "SELECT * FROM missing";
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(source, &cat, &strict());
        assert!(!model.diagnostics().is_empty());

        let renderer = DiagnosticRenderer::new(source, "test.sql");
        let mut out = Vec::new();
        let has_errors = renderer
            .render_diagnostics(model.diagnostics(), &mut out)
            .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(has_errors);
        assert!(
            text.contains("error:"),
            "expected 'error:' in output:\n{text}"
        );
        assert!(
            text.contains("missing"),
            "expected table name in output:\n{text}"
        );
    }

    #[test]
    fn renderer_includes_suggestion() {
        let source = "SELECT * FROM usres";
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("users", Some(vec!["id".to_string()]));
        let model = az.analyze(source, &cat, &strict());

        let renderer = DiagnosticRenderer::new(source, "test.sql");
        let mut out = Vec::new();
        renderer
            .render_diagnostics(model.diagnostics(), &mut out)
            .unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(
            text.contains("users"),
            "expected suggestion in output:\n{text}"
        );
    }

    // ── Fuzzy matching ─────────────────────────────────────────────────────────

    // ── CTE column list validation ────────────────────────────────────────────

    #[test]
    fn cte_without_column_list_no_error() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(
            "WITH cte AS (SELECT 1 AS x) SELECT x FROM cte",
            &cat,
            &strict(),
        );
        let sem_errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| !d.message.is_parse_error())
            .collect();
        assert!(sem_errs.is_empty(), "unexpected diagnostics: {sem_errs:?}");
    }

    #[test]
    fn cte_column_list_count_matches_no_error() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(
            "WITH cte(a, b) AS (SELECT 1, 2) SELECT a, b FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::CteColumnCountMismatch { .. }))
            .collect();
        assert!(errs.is_empty(), "unexpected mismatch diagnostic: {errs:?}");
    }

    #[test]
    fn cte_column_list_count_mismatch_is_error() {
        // 1 declared column, 2 result columns
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(
            "WITH cte(a) AS (SELECT 1, 2) SELECT a FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::CteColumnCountMismatch { .. }))
            .collect();
        assert_eq!(errs.len(), 1, "expected CteColumnCountMismatch: {errs:?}");
        assert_eq!(errs[0].severity, Severity::Error);
        if let DiagnosticMessage::CteColumnCountMismatch {
            declared, actual, ..
        } = &errs[0].message
        {
            assert_eq!(*declared, 1);
            assert_eq!(*actual, 2);
        }
    }

    #[test]
    fn cte_column_list_too_many_declared_is_error() {
        // 3 declared columns, 2 result columns
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(
            "WITH cte(a, b, c) AS (SELECT 1, 2) SELECT a FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::CteColumnCountMismatch { .. }))
            .collect();
        assert_eq!(errs.len(), 1, "expected CteColumnCountMismatch: {errs:?}");
        if let DiagnosticMessage::CteColumnCountMismatch {
            declared, actual, ..
        } = &errs[0].message
        {
            assert_eq!(*declared, 3);
            assert_eq!(*actual, 2);
        }
    }

    #[test]
    fn cte_declared_column_names_visible_in_outer_query() {
        // WITH cte(x, y): x and y should be accessible; unknown z should fail
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(
            "WITH cte(x, y) AS (SELECT 1, 2) SELECT z FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { column, .. } if column == "z"))
            .collect();
        assert_eq!(errs.len(), 1, "expected UnknownColumn for 'z': {errs:?}");
    }

    #[test]
    fn cte_declared_columns_no_false_positive_for_valid_ref() {
        // WITH cte(x, y): selecting x should be fine
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(
            "WITH cte(x, y) AS (SELECT 1, 2) SELECT x FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { .. }))
            .collect();
        assert!(errs.is_empty(), "unexpected UnknownColumn: {errs:?}");
    }

    #[test]
    fn cte_star_body_skips_count_check() {
        // SELECT * in CTE body — count check should be skipped (no false error)
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("t", Some(vec!["a".to_string(), "b".to_string()]));
        let model = az.analyze(
            "WITH cte(x, y) AS (SELECT * FROM t) SELECT x FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::CteColumnCountMismatch { .. }))
            .collect();
        assert!(
            errs.is_empty(),
            "should skip count check for SELECT *: {errs:?}"
        );
    }

    // ── CTE column inference (no declared list) ───────────────────────────────

    #[test]
    fn cte_inferred_column_invalid_ref_is_error() {
        // WITH cte AS (SELECT 1 AS x) — x is inferred, z is not; z should error
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(
            "WITH cte AS (SELECT 1 AS x) SELECT z FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { column, .. } if column == "z"))
            .collect();
        assert_eq!(errs.len(), 1, "expected UnknownColumn for 'z': {errs:?}");
    }

    #[test]
    fn cte_inferred_column_valid_ref_no_error() {
        // WITH cte AS (SELECT 1 AS x) — selecting x (the inferred column) is fine
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let model = az.analyze(
            "WITH cte AS (SELECT 1 AS x) SELECT x FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { .. }))
            .collect();
        assert!(errs.is_empty(), "unexpected UnknownColumn: {errs:?}");
    }

    #[test]
    fn cte_star_body_with_no_column_list_accepts_all() {
        // CTE body is SELECT * — inference skipped, all column refs accepted
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.layer_mut(CatalogLayer::Database)
            .insert_relation("t", Some(vec!["a".to_string(), "b".to_string()]));
        let model = az.analyze(
            "WITH cte AS (SELECT * FROM t) SELECT anything FROM cte",
            &cat,
            &strict(),
        );
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { .. }))
            .collect();
        assert!(
            errs.is_empty(),
            "star body should accept any col ref: {errs:?}"
        );
    }

    // ── CREATE TABLE/VIEW AS SELECT column inference ──────────────────────────

    #[test]
    fn create_table_as_select_invalid_column_is_error() {
        // CREATE TABLE t AS SELECT 1 AS x → x inferred; z should error
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let src = "CREATE TABLE t AS SELECT 1 AS x; SELECT z FROM t;";
        let model = az.analyze(src, &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { column, .. } if column == "z"))
            .collect();
        assert_eq!(errs.len(), 1, "expected UnknownColumn for 'z': {errs:?}");
    }

    #[test]
    fn create_table_as_select_valid_column_no_error() {
        // CREATE TABLE t AS SELECT 1 AS x → selecting x is fine
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let src = "CREATE TABLE t AS SELECT 1 AS x; SELECT x FROM t;";
        let model = az.analyze(src, &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { .. }))
            .collect();
        assert!(errs.is_empty(), "unexpected UnknownColumn: {errs:?}");
    }

    #[test]
    fn create_view_as_select_invalid_column_is_error() {
        // CREATE VIEW v AS SELECT 1 AS x → x inferred; z should error
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let src = "CREATE VIEW v AS SELECT 1 AS x; SELECT z FROM v;";
        let model = az.analyze(src, &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { column, .. } if column == "z"))
            .collect();
        assert_eq!(errs.len(), 1, "expected UnknownColumn for 'z': {errs:?}");
    }

    #[test]
    fn create_view_as_select_valid_column_no_error() {
        // CREATE VIEW v AS SELECT 1 AS x → selecting x is fine
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let src = "CREATE VIEW v AS SELECT 1 AS x; SELECT x FROM v;";
        let model = az.analyze(src, &cat, &strict());
        let errs: Vec<_> = model
            .diagnostics()
            .iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownColumn { .. }))
            .collect();
        assert!(errs.is_empty(), "unexpected UnknownColumn: {errs:?}");
    }

    #[test]
    fn levenshtein_same_string_is_zero() {
        use super::super::fuzzy::levenshtein_distance;
        assert_eq!(levenshtein_distance("abc", "abc"), 0);
    }

    #[test]
    fn levenshtein_one_edit() {
        use super::super::fuzzy::levenshtein_distance;
        assert_eq!(levenshtein_distance("abc", "axc"), 1);
        assert_eq!(levenshtein_distance("abc", "abcd"), 1);
        assert_eq!(levenshtein_distance("abcd", "abc"), 1);
    }

    #[test]
    fn best_suggestion_finds_closest() {
        use super::super::fuzzy::best_suggestion;
        let candidates = vec![
            "users".to_string(),
            "orders".to_string(),
            "products".to_string(),
        ];
        let s = best_suggestion("usres", &candidates, 2);
        assert_eq!(s.as_deref(), Some("users"));
    }

    #[test]
    fn best_suggestion_none_when_too_far() {
        use super::super::fuzzy::best_suggestion;
        let candidates = vec!["users".to_string()];
        let s = best_suggestion("xyzzy", &candidates, 2);
        assert!(s.is_none());
    }
}
