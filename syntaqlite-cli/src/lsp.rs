// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::error::Error;
use std::path::PathBuf;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{Completion, Formatting, Request as _, SemanticTokensFullRequest};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionResponse, DiagnosticSeverity,
    InitializeParams, Position, PositionEncodingKind, Range, SemanticTokenType,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensResult, SemanticTokensServerCapabilities, ServerCapabilities,
    TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Uri,
};
use syntaqlite::Dialect;
use syntaqlite::dialect::{SEMANTIC_TOKEN_LEGEND, TokenCategory};
use syntaqlite::fmt::FormatConfig;
use syntaqlite::lsp::{AnalysisHost, Severity};

pub(crate) fn cmd_lsp(dialect: &Dialect) -> Result<(), String> {
    run_lsp(dialect).map_err(|e| format!("LSP error: {e}"))
}

fn run_lsp(dialect: &Dialect) -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        position_encoding: Some(PositionEncodingKind::UTF8),
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec![" ".into(), "\n".into(), "\t".into(), ";".into()]),
            ..Default::default()
        }),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: SEMANTIC_TOKEN_LEGEND
                        .iter()
                        .map(|&name| SemanticTokenType::new(name))
                        .collect(),
                    token_modifiers: vec![],
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                ..Default::default()
            },
        )),
        ..Default::default()
    })?;

    let init_params_raw = connection.initialize(server_capabilities)?;
    let init_params: InitializeParams = serde_json::from_value(init_params_raw)?;

    if let Some(root) = workspace_root(&init_params) {
        eprintln!("syntaqlite-lsp: workspace root: {}", root.display());
    }

    let mut host = AnalysisHost::with_dialect(*dialect);

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(&connection, dialect, &mut host, req)?;
            }
            Message::Notification(notif) => {
                handle_notification(&connection, &mut host, notif)?;
            }
            Message::Response(_) => {}
        }
    }

    io_threads.join()?;
    Ok(())
}

fn handle_request(
    connection: &Connection,
    dialect: &Dialect,
    host: &mut AnalysisHost,
    req: Request,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let response = match req.method.as_str() {
        Completion::METHOD => {
            let params: lsp_types::CompletionParams = serde_json::from_value(req.params)?;
            let uri = params.text_document_position.text_document.uri;
            let position = params.text_document_position.position;
            let uri_str = uri.as_str();
            match host.document_source(uri_str) {
                Some(source) => {
                    let offset = position_to_offset(source, position);
                    let expected = host.expected_tokens_at_offset(uri_str, offset);
                    let items = completion_items_for_expected(dialect, host, &expected);
                    Response::new_ok(req.id, CompletionResponse::Array(items))
                }
                None => Response::new_ok(req.id, Option::<CompletionResponse>::None),
            }
        }
        Formatting::METHOD => {
            let params: lsp_types::DocumentFormattingParams = serde_json::from_value(req.params)?;
            let uri = params.text_document.uri.as_str();
            let config = FormatConfig::default();

            match host.format(uri, &config) {
                Ok(formatted) => {
                    let edit = TextEdit {
                        range: Range::new(Position::new(0, 0), Position::new(u32::MAX, 0)),
                        new_text: formatted,
                    };
                    Response::new_ok(req.id, Some(vec![edit]))
                }
                Err(e) => Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InternalError as i32,
                    e.to_string(),
                ),
            }
        }
        SemanticTokensFullRequest::METHOD => {
            let params: lsp_types::SemanticTokensParams = serde_json::from_value(req.params)?;
            let uri = params.text_document.uri.as_str();
            let encoded = host.semantic_tokens_encoded(uri, None);

            // Convert flat u32 array (5 per token) into lsp_types::SemanticToken structs.
            let data: Vec<lsp_types::SemanticToken> = encoded
                .chunks_exact(5)
                .map(|c| lsp_types::SemanticToken {
                    delta_line: c[0],
                    delta_start: c[1],
                    length: c[2],
                    token_type: c[3],
                    token_modifiers_bitset: c[4],
                })
                .collect();

            let result = SemanticTokensResult::Tokens(lsp_types::SemanticTokens {
                result_id: None,
                data,
            });
            Response::new_ok(req.id, result)
        }
        _ => Response::new_err(
            req.id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            format!("unknown request method: {}", req.method),
        ),
    };
    connection.sender.send(Message::Response(response))?;
    Ok(())
}

fn completion_items_for_expected(
    dialect: &Dialect,
    host: &AnalysisHost,
    expected: &[u32],
) -> Vec<CompletionItem> {
    let expected_set: std::collections::HashSet<u32> = expected.iter().copied().collect();
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    let mut expects_identifier = false;

    for &tok in expected {
        let category = dialect.token_category(tok);
        if category == TokenCategory::Identifier {
            expects_identifier = true;
        }
    }

    for i in 0..dialect.keyword_count() {
        let Some((code, label)) = dialect.keyword_entry(i) else {
            continue;
        };
        if !expected_set.contains(&code) {
            continue;
        }
        if !is_keyword_symbol(label) {
            continue;
        }
        let label = label.to_string();
        if seen.insert(label.clone()) {
            out.push(CompletionItem {
                label,
                kind: Some(CompletionItemKind::KEYWORD),
                detail: Some("keyword".into()),
                ..Default::default()
            });
        }
    }

    if expects_identifier {
        if let Some(ctx) = host.ambient_context() {
            for table in &ctx.tables {
                if seen.insert(table.name.clone()) {
                    out.push(CompletionItem {
                        label: table.name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        detail: Some("table".into()),
                        ..Default::default()
                    });
                }
                for col in &table.columns {
                    if seen.insert(col.name.clone()) {
                        out.push(CompletionItem {
                            label: col.name.clone(),
                            kind: Some(CompletionItemKind::FIELD),
                            detail: Some(format!("column ({})", table.name)),
                            ..Default::default()
                        });
                    }
                }
            }
            for view in &ctx.views {
                if seen.insert(view.name.clone()) {
                    out.push(CompletionItem {
                        label: view.name.clone(),
                        kind: Some(CompletionItemKind::STRUCT),
                        detail: Some("view".into()),
                        ..Default::default()
                    });
                }
                for col in &view.columns {
                    if seen.insert(col.name.clone()) {
                        out.push(CompletionItem {
                            label: col.name.clone(),
                            kind: Some(CompletionItemKind::FIELD),
                            detail: Some(format!("column ({})", view.name)),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        // All available functions: SQLite built-in (filtered by version/cflags)
        // merged with user-provided functions from ambient context.
        for func in host.available_functions() {
            if seen.insert(func.name.clone()) {
                let detail = match func.args {
                    Some(n) => format!("function ({n} args)"),
                    None => "function (variadic)".into(),
                };
                out.push(CompletionItem {
                    label: func.name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(detail),
                    ..Default::default()
                });
            }
        }
    }

    out
}

fn is_keyword_symbol(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit() || b == b'_')
}

fn handle_notification(
    connection: &Connection,
    host: &mut AnalysisHost,
    notif: Notification,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    match notif.method.as_str() {
        DidOpenTextDocument::METHOD => {
            let params: lsp_types::DidOpenTextDocumentParams =
                serde_json::from_value(notif.params)?;
            let uri = &params.text_document.uri;
            host.open_document(
                uri.as_str(),
                params.text_document.version,
                params.text_document.text,
            );
            publish_diagnostics(connection, host, uri)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: lsp_types::DidChangeTextDocumentParams =
                serde_json::from_value(notif.params)?;
            let uri = &params.text_document.uri;
            // Full sync — take the last content change.
            if let Some(change) = params.content_changes.into_iter().last() {
                host.update_document(uri.as_str(), params.text_document.version, change.text);
            }
            publish_diagnostics(connection, host, uri)?;
        }
        DidCloseTextDocument::METHOD => {
            let params: lsp_types::DidCloseTextDocumentParams =
                serde_json::from_value(notif.params)?;
            let uri = params.text_document.uri.as_str();
            // Clear diagnostics before closing.
            let clear = lsp_types::PublishDiagnosticsParams {
                uri: params.text_document.uri.clone(),
                diagnostics: vec![],
                version: None,
            };
            let notif = Notification::new(PublishDiagnostics::METHOD.to_string(), clear);
            connection.sender.send(Message::Notification(notif))?;
            host.close_document(uri);
        }
        _ => {}
    }
    Ok(())
}

fn publish_diagnostics(
    connection: &Connection,
    host: &mut AnalysisHost,
    uri: &Uri,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let uri_str = uri.as_str();
    let Some((version, source, diags)) = host.document_diagnostics(uri_str) else {
        return Ok(());
    };

    // Collect all offsets and convert in a single O(n) pass.
    let mut offsets: Vec<usize> = Vec::with_capacity(diags.len() * 2);
    for d in diags {
        offsets.push(d.start_offset);
        offsets.push(d.end_offset);
    }
    let positions = offsets_to_positions(source, &offsets);

    let lsp_diags: Vec<lsp_types::Diagnostic> = diags
        .iter()
        .enumerate()
        .map(|(i, d)| {
            let start = positions[i * 2];
            let end = positions[i * 2 + 1];
            lsp_types::Diagnostic {
                range: Range::new(start, end),
                severity: Some(match d.severity {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                    Severity::Info => DiagnosticSeverity::INFORMATION,
                    Severity::Hint => DiagnosticSeverity::HINT,
                }),
                message: d.message.clone(),
                source: Some("syntaqlite".to_string()),
                ..Default::default()
            }
        })
        .collect();

    let params = lsp_types::PublishDiagnosticsParams {
        uri: uri.clone(),
        diagnostics: lsp_diags,
        version: Some(version),
    };

    let notif = Notification::new(PublishDiagnostics::METHOD.to_string(), params);
    connection.sender.send(Message::Notification(notif))?;
    Ok(())
}

/// Convert multiple byte offsets to LSP Positions in a single O(n) pass.
///
/// Sorts offsets internally and walks the source once, producing positions
/// in the original order.
fn offsets_to_positions(source: &str, offsets: &[usize]) -> Vec<Position> {
    if offsets.is_empty() {
        return Vec::new();
    }

    // Build (offset, original_index) pairs sorted by offset.
    let mut indexed: Vec<(usize, usize)> = offsets
        .iter()
        .copied()
        .enumerate()
        .map(|(i, o)| (o, i))
        .collect();
    indexed.sort_unstable_by_key(|&(o, _)| o);

    let mut result = vec![Position::new(0, 0); offsets.len()];
    let src = source.as_bytes();
    let len = src.len();
    let mut line = 0u32;
    let mut col = 0u32;
    let mut pos = 0usize;

    for (offset, orig_idx) in indexed {
        let offset = offset.min(len);
        while pos < offset {
            if src[pos] == b'\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
            pos += 1;
        }
        result[orig_idx] = Position::new(line, col);
    }

    result
}

fn position_to_offset(source: &str, pos: Position) -> usize {
    let src = source.as_bytes();
    let len = src.len();
    let mut line = 0usize;
    let mut line_start = 0usize;

    while line < pos.line as usize && line_start < len {
        if let Some(next_nl) = src[line_start..].iter().position(|&b| b == b'\n') {
            line_start += next_nl + 1;
            line += 1;
        } else {
            return len;
        }
    }

    let line_end = src[line_start..]
        .iter()
        .position(|&b| b == b'\n')
        .map(|rel| line_start + rel)
        .unwrap_or(len);

    line_start + (pos.character as usize).min(line_end - line_start)
}

/// Extract the workspace root directory from LSP initialization params.
///
/// Prefers `root_uri` (modern); falls back to deprecated `root_path`.
fn workspace_root(params: &InitializeParams) -> Option<PathBuf> {
    #[allow(deprecated)]
    if let Some(uri) = &params.root_uri {
        let s = uri.as_str();
        if let Some(path) = s.strip_prefix("file://") {
            return Some(PathBuf::from(path));
        }
    }
    #[allow(deprecated)]
    params.root_path.as_ref().map(PathBuf::from)
}

#[cfg(test)]
mod tests {
    use lsp_types::CompletionItem;

    use super::completion_items_for_expected;
    use syntaqlite::lsp::AnalysisHost;

    #[test]
    fn join_kw_uses_dialect_token_keyword_mapping() {
        let dialect = *syntaqlite::sqlite::low_level::dialect();
        let mut host = AnalysisHost::with_dialect(dialect);
        let uri = "file:///test.sql";
        let sql = "SELECT * FROM s AS x J";
        host.open_document(uri, 1, sql.to_string());

        let expected = host.expected_tokens_at_offset(uri, sql.len());
        let items = completion_items_for_expected(&dialect, &host, &expected);

        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        let join_kw = syntaqlite::sqlite::low_level::TokenType::JoinKw as u32;
        let join_kw_labels: Vec<&str> = (0..dialect.keyword_count())
            .filter_map(|i| dialect.keyword_entry(i))
            .filter_map(|(code, kw)| (code == join_kw).then_some(kw))
            .collect();
        let expected_names: Vec<String> = expected.iter().map(|&tok| format!("#{tok}")).collect();
        assert!(
            join_kw_labels.iter().any(|kw| labels.contains(kw)),
            "none of {join_kw_labels:?} found: labels={labels:?}, expected={expected_names:?}"
        );
    }

    #[test]
    fn function_completions_include_builtin_functions() {
        let dialect = *syntaqlite::sqlite::low_level::dialect();
        let mut host = AnalysisHost::with_dialect(dialect);
        let uri = "file:///test.sql";
        // After SELECT, an identifier (including function names) is expected.
        let sql = "SELECT ";
        host.open_document(uri, 1, sql.to_string());

        let expected = host.expected_tokens_at_offset(uri, sql.len());
        let items = completion_items_for_expected(&dialect, &host, &expected);

        let func_items: Vec<&CompletionItem> = items
            .iter()
            .filter(|i| i.kind == Some(lsp_types::CompletionItemKind::FUNCTION))
            .collect();
        assert!(
            !func_items.is_empty(),
            "should have function completion items"
        );

        let labels: Vec<&str> = func_items.iter().map(|i| i.label.as_str()).collect();
        assert!(labels.contains(&"abs"), "abs should be in completions");
        assert!(labels.contains(&"count"), "count should be in completions");
    }

    #[test]
    fn function_completions_respect_dialect_config() {
        let dialect = *syntaqlite::sqlite::low_level::dialect();
        let mut host = AnalysisHost::with_dialect(dialect);
        let mut config = syntaqlite::dialect::ffi::DialectConfig::default();
        // Enable math functions
        config.cflags.set(34);
        host.set_dialect_config(config);

        let uri = "file:///test.sql";
        let sql = "SELECT ";
        host.open_document(uri, 1, sql.to_string());

        let expected = host.expected_tokens_at_offset(uri, sql.len());
        let items = completion_items_for_expected(&dialect, &host, &expected);
        let labels: Vec<&str> = items.iter().map(|i| i.label.as_str()).collect();
        assert!(
            labels.contains(&"acos"),
            "acos should appear with ENABLE_MATH_FUNCTIONS"
        );
    }

    #[test]
    fn function_completions_not_offered_where_identifiers_not_expected() {
        let dialect = *syntaqlite::sqlite::low_level::dialect();
        let mut host = AnalysisHost::with_dialect(dialect);
        let uri = "file:///test.sql";
        // After FROM + table + trailing space, only keywords like JOIN/WHERE expected, not identifiers for functions.
        // Actually, identifiers are expected here (table names). Let's use a position
        // where only keywords are expected: right after SELECT keyword with no space.
        // "CREATE " expects TABLE/VIEW/etc. keywords, not identifiers.
        let sql = "CREATE ";
        host.open_document(uri, 1, sql.to_string());

        let expected = host.expected_tokens_at_offset(uri, sql.len());
        let items = completion_items_for_expected(&dialect, &host, &expected);
        let func_items: Vec<&CompletionItem> = items
            .iter()
            .filter(|i| i.kind == Some(lsp_types::CompletionItemKind::FUNCTION))
            .collect();
        assert!(
            func_items.is_empty(),
            "no function completions expected after CREATE, got: {:?}",
            func_items.iter().map(|i| &i.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn keyword_completion_items_use_keyword_kind() {
        let dialect = *syntaqlite::sqlite::low_level::dialect();
        let mut host = AnalysisHost::with_dialect(dialect);
        let uri = "file:///test.sql";
        let sql = "SELECT a FROM t WH";
        host.open_document(uri, 1, sql.to_string());

        let expected = host.expected_tokens_at_offset(uri, sql.len());
        let items = completion_items_for_expected(&dialect, &host, &expected);

        let where_item = items.iter().find(|i| i.label == "WHERE").unwrap();
        assert_eq!(
            where_item.kind,
            Some(lsp_types::CompletionItemKind::KEYWORD)
        );
        assert_eq!(where_item.detail.as_deref(), Some("keyword"));
    }
}
