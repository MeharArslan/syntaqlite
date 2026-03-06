// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Single-pass semantic analysis engine.

use std::collections::HashSet;

use syntaqlite_syntax::any::AnyNodeId;
use syntaqlite_syntax::ast_traits::AstTypes;
use syntaqlite_syntax::typed::{GrammarNodeType, TypedParser};
use syntaqlite_syntax::{ParseOutcome, ParserConfig, TokenType};

use crate::dialect::Dialect;

use super::ValidationConfig;
use super::catalog::Catalog;
use super::diagnostics::{Diagnostic, DiagnosticMessage, Severity};
use super::model::{
    CompletionContext, CompletionInfo, SemanticModel, SemanticToken, StoredComment, StoredToken,
};
use super::walker::{WalkContext, Walker};

/// Long-lived semantic analysis engine.
///
/// Create once for a dialect and reuse across inputs. The dialect layer is
/// built at construction and never changes. The database and document layers
/// are reset on each [`analyze`](Self::analyze) call.
pub struct SemanticAnalyzer {
    dialect: Dialect,
    catalog: Catalog,
}

impl SemanticAnalyzer {
    /// Create an analyzer for the built-in `SQLite` dialect.
    #[cfg(feature = "sqlite")]
    pub(crate) fn new() -> Self {
        Self::with_dialect(crate::sqlite::dialect::dialect())
    }

    /// Create an analyzer bound to a specific dialect.
    pub(crate) fn with_dialect(dialect: impl Into<Dialect>) -> Self {
        let dialect = dialect.into();
        SemanticAnalyzer {
            catalog: Catalog::new(dialect),
            dialect,
        }
    }

    /// Return the dialect this analyzer was constructed for.
    pub(crate) fn dialect(&self) -> Dialect {
        self.dialect
    }

    /// Run a complete single-pass analysis: parse, collect tokens, walk AST.
    ///
    /// `user_catalog` supplies the database layer (user-provided schema). Its
    /// database layer is merged into the analyzer's catalog for this pass only.
    /// The document layer is cleared and rebuilt statement-by-statement so that
    /// DDL seen earlier in the file is visible to queries that follow it.
    pub(crate) fn analyze(
        &mut self,
        source: &str,
        user_catalog: &Catalog,
        config: &ValidationConfig,
    ) -> SemanticModel {
        self.catalog.clear_document();
        self.catalog.database = user_catalog.database.clone();
        self.analyze_inner::<syntaqlite_syntax::nodes::SqliteAstMarker>(source, config)
    }

    /// Semantic tokens for syntax highlighting, derived from a prior
    /// [`analyze`](Self::analyze) result.
    pub(crate) fn semantic_tokens(&self, model: &SemanticModel) -> Vec<SemanticToken> {
        use syntaqlite_syntax::any::TokenCategory;

        let mut out = Vec::new();
        for t in &model.tokens {
            let cat = self.dialect.classify_token(t.token_type.into(), t.flags);
            if cat != TokenCategory::Other {
                out.push(SemanticToken { offset: t.offset, length: t.length, category: cat });
            }
        }
        for c in &model.comments {
            out.push(SemanticToken {
                offset:   c.offset,
                length:   c.length,
                category: TokenCategory::Comment,
            });
        }
        out.sort_by_key(|t| t.offset);
        out
    }

    /// Expected tokens and semantic context at `offset` (for completion).
    pub(crate) fn completion_info(&self, model: &SemanticModel, offset: usize) -> CompletionInfo {
        let source       = model.source();
        let tokens       = &model.tokens;
        let cursor       = offset.min(source.len());
        let (boundary, backtracked) = completion_boundary(source, tokens, cursor);
        let start        = statement_token_start(tokens, boundary);
        let stmt_tokens  = &tokens[start..boundary];

        let parser       = TypedParser::new(syntaqlite_syntax::typed::grammar());
        let mut cursor_p = parser.incremental_parse(source);
        let mut last_expected: Vec<TokenType> = cursor_p.expected_tokens().collect();

        for tok in stmt_tokens {
            let span = tok.offset..(tok.offset + tok.length);
            if cursor_p.feed_token(tok.token_type, span).is_some() {
                return CompletionInfo {
                    tokens:  last_expected,
                    context: CompletionContext::from_parser(cursor_p.completion_context()),
                };
            }
            last_expected = cursor_p.expected_tokens().collect();
        }

        let context = CompletionContext::from_parser(cursor_p.completion_context());

        if backtracked {
            if let Some(extra) = tokens.get(boundary) {
                let span = extra.offset..(extra.offset + extra.length);
                if cursor_p.feed_token(extra.token_type, span).is_none() {
                    merge_expected_tokens(&mut last_expected, cursor_p.expected_tokens().collect());
                }
            }
        }

        CompletionInfo { tokens: last_expected, context }
    }

    // ── Private ───────────────────────────────────────────────────────────────

    #[cfg(feature = "sqlite")]
    fn analyze_inner<A: for<'a> AstTypes<'a>>(
        &mut self,
        source: &str,
        config: &ValidationConfig,
    ) -> SemanticModel {
        let parser = syntaqlite_syntax::Parser::with_config(
            &ParserConfig::default().with_collect_tokens(true),
        );
        let mut session = parser.parse(source);

        let mut tokens:      Vec<StoredToken>  = Vec::new();
        let mut comments:    Vec<StoredComment> = Vec::new();
        let mut diagnostics: Vec<Diagnostic>   = Vec::new();

        loop {
            let stmt = match session.next() {
                ParseOutcome::Done    => break,
                ParseOutcome::Ok(s)   => s,
                ParseOutcome::Err(e)  => {
                    let (start, end) = parse_error_span(&e, source);
                    diagnostics.push(Diagnostic {
                        start_offset: start,
                        end_offset:   end,
                        message:      DiagnosticMessage::Other(e.message().to_owned()),
                        severity:     Severity::Error,
                        help:         None,
                    });
                    continue;
                }
            };

            // Collect token and comment positions for semantic highlighting.
            for tok in stmt.tokens() {
                tokens.push(StoredToken {
                    offset:     str_offset(source, tok.text()),
                    length:     tok.text().len(),
                    token_type: tok.token_type(),
                    flags:      tok.flags(),
                });
            }
            for c in stmt.comments() {
                comments.push(StoredComment {
                    offset: str_offset(source, c.text),
                    length: c.text.len(),
                });
            }

            // Semantic walk.
            let root    = stmt.root();
            let root_id: AnyNodeId = root.node_id().into();
            let erased  = stmt.erase();

            self.catalog.accumulate_ddl::<A>(erased, root_id, self.dialect);

            if let Some(root_stmt) = A::Stmt::from_result(erased, root_id) {
                let ctx = WalkContext { catalog: &mut self.catalog, config };
                diagnostics.extend(Walker::<A>::run(erased, root_stmt, ctx));
            }
        }

        SemanticModel { source: source.to_owned(), tokens, comments, diagnostics }
    }
}

#[cfg(feature = "sqlite")]
impl Default for SemanticAnalyzer {
    fn default() -> Self { Self::new() }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn str_offset(source: &str, part: &str) -> usize {
    part.as_ptr() as usize - source.as_ptr() as usize
}

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
            let end   = source.len();
            let start = if end > 0 { end - 1 } else { 0 };
            (start, end)
        }
    }
}

fn completion_boundary(
    source: &str,
    tokens: &[StoredToken],
    cursor_offset: usize,
) -> (usize, bool) {
    let mut boundary =
        tokens.partition_point(|t| t.offset + t.length <= cursor_offset);

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
            boundary   -= 1;
            backtracked = true;
        }
    }
    (boundary, backtracked)
}

fn statement_token_start(tokens: &[StoredToken], boundary: usize) -> usize {
    tokens[..boundary]
        .iter()
        .rposition(|t| t.token_type == TokenType::Semi)
        .map_or(0, |idx| idx + 1)
}

fn merge_expected_tokens(into: &mut Vec<TokenType>, extra: Vec<TokenType>) {
    let mut seen: HashSet<TokenType> = into.iter().copied().collect();
    for token in extra {
        if seen.insert(token) {
            into.push(token);
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::catalog::FunctionCheckResult;
    use super::super::diagnostics::{DiagnosticMessage, Help};
    use super::super::render::DiagnosticRenderer;

    fn sqlite_analyzer() -> SemanticAnalyzer {
        SemanticAnalyzer::new()
    }

    fn sqlite_catalog() -> Catalog {
        Catalog::new(crate::sqlite::dialect::dialect())
    }

    fn strict() -> ValidationConfig {
        ValidationConfig { strict_schema: true, suggestion_threshold: 2 }
    }

    fn lenient() -> ValidationConfig {
        ValidationConfig::default()
    }

    // ── Catalog ────────────────────────────────────────────────────────────────

    #[test]
    fn catalog_add_table_and_resolve() {
        let mut cat = sqlite_catalog();
        cat.add_table("users", &["id", "name"]);
        assert!(cat.resolve_relation("users"));
        assert!(cat.resolve_relation("USERS")); // case-insensitive
        assert!(!cat.resolve_relation("orders"));
    }

    #[test]
    fn catalog_add_view_and_resolve() {
        let mut cat = sqlite_catalog();
        cat.add_view("active_users", &["id"]);
        assert!(cat.resolve_relation("active_users"));
    }

    #[test]
    fn catalog_add_function_and_check() {
        let mut cat = sqlite_catalog();
        cat.add_function("my_func", Some(2));
        assert!(matches!(cat.check_function("my_func", 2), FunctionCheckResult::Ok));
        assert!(matches!(
            cat.check_function("my_func", 1),
            FunctionCheckResult::WrongArity { .. }
        ));
    }

    #[test]
    fn catalog_add_variadic_function() {
        let mut cat = sqlite_catalog();
        cat.add_function("variadic_fn", None);
        assert!(matches!(cat.check_function("variadic_fn", 0), FunctionCheckResult::Ok));
        assert!(matches!(cat.check_function("variadic_fn", 100), FunctionCheckResult::Ok));
    }

    #[test]
    fn catalog_builtin_functions_resolved() {
        let cat = sqlite_catalog();
        // SQLite has built-in functions like abs(), coalesce(), etc.
        assert!(!matches!(cat.check_function("abs", 1), FunctionCheckResult::Unknown));
        assert!(!matches!(cat.check_function("coalesce", 2), FunctionCheckResult::Unknown));
    }

    #[test]
    #[ignore = "requires SQLITE_SEMANTIC_ROLES to be populated by run-codegen (step 3)"]
    fn catalog_from_ddl_populates_tables() {
        let dialect = crate::sqlite::dialect::dialect();
        let cat = Catalog::from_ddl(dialect, "CREATE TABLE users (id INTEGER, name TEXT);");
        assert!(cat.resolve_relation("users"));
    }

    #[test]
    fn catalog_clear_database() {
        let mut cat = sqlite_catalog();
        cat.add_table("tmp", &["id"]);
        assert!(cat.resolve_relation("tmp"));
        cat.clear_database();
        assert!(!cat.resolve_relation("tmp"));
    }

    // ── Analyzer: no-error cases ───────────────────────────────────────────────

    #[test]
    fn analyze_select_from_known_table_no_errors() {
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.add_table("users", &["id", "name"]);

        let model = az.analyze("SELECT id FROM users", &cat, &strict());
        let diags: Vec<_> = model.diagnostics().iter()
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
        let sem_errs: Vec<_> = model.diagnostics().iter()
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
        cat.add_table("users", &["id"]);
        let model = az.analyze("SELECT * FROM usres", &cat, &strict()); // typo
        let diag = model.diagnostics().iter()
            .find(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { name } if name == "usres"));
        assert!(diag.is_some(), "expected unknown-table diagnostic");
        let diag = diag.unwrap();
        assert!(
            matches!(&diag.help, Some(Help::Suggestion(s)) if s == "users"),
            "expected 'users' suggestion, got {:?}", diag.help
        );
    }

    // ── Analyzer: DDL accumulation ─────────────────────────────────────────────

    #[test]
    #[ignore = "requires SQLITE_SEMANTIC_ROLES to be populated by run-codegen (step 3)"]
    fn analyze_create_table_then_select_no_error() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();
        let src = "CREATE TABLE t (id INTEGER); SELECT id FROM t;";
        let model = az.analyze(src, &cat, &strict());
        let unknown: Vec<_> = model.diagnostics().iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert!(unknown.is_empty(), "DDL-defined table not visible: {unknown:?}");
    }

    #[test]
    #[ignore = "requires SQLITE_SEMANTIC_ROLES to be populated by run-codegen (step 3)"]
    fn analyze_create_view_then_select_no_error() {
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.add_table("users", &["id"]);
        let src = "CREATE VIEW vw AS SELECT id FROM users; SELECT id FROM vw;";
        let model = az.analyze(src, &cat, &strict());
        let unknown: Vec<_> = model.diagnostics().iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert!(unknown.is_empty(), "VIEW not visible: {unknown:?}");
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
        let errs: Vec<_> = model.diagnostics().iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownFunction { .. }))
            .collect();
        assert!(errs.is_empty(), "abs() should be a known builtin: {errs:?}");
    }

    // ── Analyzer: multiple statements ─────────────────────────────────────────

    #[test]
    fn analyze_multiple_selects_independent() {
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.add_table("users", &["id"]);
        let src = "SELECT id FROM users; SELECT id FROM users;";
        let model = az.analyze(src, &cat, &strict());
        let errs: Vec<_> = model.diagnostics().iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { .. }))
            .collect();
        assert!(errs.is_empty());
    }

    #[test]
    fn analyze_reuse_clears_document_layer() {
        let mut az = sqlite_analyzer();
        let cat = sqlite_catalog();

        // First pass: CREATE TABLE makes 't' visible.
        az.analyze("CREATE TABLE t (id INTEGER); SELECT id FROM t;", &cat, &strict());

        // Second pass: 't' should NOT be visible — document layer was cleared.
        let model = az.analyze("SELECT id FROM t;", &cat, &strict());
        let errs: Vec<_> = model.diagnostics().iter()
            .filter(|d| matches!(&d.message, DiagnosticMessage::UnknownTable { name } if name == "t"))
            .collect();
        assert_eq!(errs.len(), 1, "document layer should be cleared between passes");
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
        let has_errors = renderer.render_diagnostics(model.diagnostics(), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(has_errors);
        assert!(text.contains("error:"), "expected 'error:' in output:\n{text}");
        assert!(text.contains("missing"), "expected table name in output:\n{text}");
    }

    #[test]
    fn renderer_includes_suggestion() {
        let source = "SELECT * FROM usres";
        let mut az = sqlite_analyzer();
        let mut cat = sqlite_catalog();
        cat.add_table("users", &["id"]);
        let model = az.analyze(source, &cat, &strict());

        let renderer = DiagnosticRenderer::new(source, "test.sql");
        let mut out = Vec::new();
        renderer.render_diagnostics(model.diagnostics(), &mut out).unwrap();
        let text = String::from_utf8(out).unwrap();
        assert!(text.contains("users"), "expected suggestion in output:\n{text}");
    }

    // ── Fuzzy matching ─────────────────────────────────────────────────────────

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
        let candidates = vec!["users".to_string(), "orders".to_string(), "products".to_string()];
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
