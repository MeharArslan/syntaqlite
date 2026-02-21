// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::error::Error;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{Formatting, Request as _, SemanticTokensFullRequest};
use lsp_types::{
    DiagnosticSeverity, Position, Range, SemanticTokenType, SemanticTokensFullOptions,
    SemanticTokensLegend, SemanticTokensOptions, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, TextEdit,
};
use syntaqlite_lsp::{AnalysisHost, Severity, TokenCategory};
use syntaqlite_runtime::Dialect;
use syntaqlite_runtime::fmt::FormatConfig;

pub(crate) fn cmd_lsp(dialect: &Dialect) -> Result<(), String> {
    run_lsp(dialect).map_err(|e| format!("LSP error: {e}"))
}

fn run_lsp(dialect: &Dialect) -> Result<(), Box<dyn Error + Sync + Send>> {
    let (connection, io_threads) = Connection::stdio();

    let server_capabilities = serde_json::to_value(ServerCapabilities {
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                legend: SemanticTokensLegend {
                    token_types: vec![
                        SemanticTokenType::KEYWORD,            // 0
                        SemanticTokenType::VARIABLE,           // 1
                        SemanticTokenType::STRING,             // 2
                        SemanticTokenType::NUMBER,             // 3
                        SemanticTokenType::OPERATOR,           // 4
                        SemanticTokenType::COMMENT,            // 5
                        SemanticTokenType::new("punctuation"), // 6
                        SemanticTokenType::TYPE,               // 7 (identifier)
                        SemanticTokenType::FUNCTION,           // 8
                    ],
                    token_modifiers: vec![],
                },
                full: Some(SemanticTokensFullOptions::Bool(true)),
                ..Default::default()
            },
        )),
        ..Default::default()
    })?;

    let _init_params = connection.initialize(server_capabilities)?;

    let mut host = AnalysisHost::new(*dialect);

    for msg in &connection.receiver {
        match msg {
            Message::Request(req) => {
                if connection.handle_shutdown(&req)? {
                    return Ok(());
                }
                handle_request(&connection, &host, req)?;
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
    host: &AnalysisHost,
    req: Request,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    if req.method == Formatting::METHOD {
        let params: lsp_types::DocumentFormattingParams = serde_json::from_value(req.params)?;
        let uri = params.text_document.uri.as_str();
        let config = FormatConfig::default();

        let response = match host.format(uri, &config) {
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
        };
        connection.sender.send(Message::Response(response))?;
    } else if req.method == SemanticTokensFullRequest::METHOD {
        let params: lsp_types::SemanticTokensParams = serde_json::from_value(req.params)?;
        let uri = params.text_document.uri.as_str();
        let source = host.document_source(uri).unwrap_or("").to_string();
        let tokens = host.semantic_tokens(uri);

        let mut data = Vec::new();
        let mut prev_line = 0u32;
        let mut prev_start = 0u32;

        for tok in &tokens {
            let pos = offset_to_position(&source, tok.offset);
            let delta_line = pos.line - prev_line;
            let delta_start = if delta_line == 0 {
                pos.character - prev_start
            } else {
                pos.character
            };
            let token_type = category_to_legend_index(tok.category);
            data.push(lsp_types::SemanticToken {
                delta_line,
                delta_start,
                length: tok.length as u32,
                token_type,
                token_modifiers_bitset: 0,
            });
            prev_line = pos.line;
            prev_start = pos.character;
        }

        let result = SemanticTokensResult::Tokens(lsp_types::SemanticTokens {
            result_id: None,
            data,
        });
        let response = Response::new_ok(req.id, result);
        connection.sender.send(Message::Response(response))?;
    }
    Ok(())
}

/// Map a `TokenCategory` to an index in the semantic tokens legend.
fn category_to_legend_index(cat: TokenCategory) -> u32 {
    match cat {
        TokenCategory::Keyword => 0,
        TokenCategory::Variable => 1,
        TokenCategory::String => 2,
        TokenCategory::Number => 3,
        TokenCategory::Operator => 4,
        TokenCategory::Comment => 5,
        TokenCategory::Punctuation => 6,
        TokenCategory::Identifier => 7,
        TokenCategory::Function => 8,
        TokenCategory::Other => 0,
    }
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
            let uri = params.text_document.uri.as_str();
            host.open_document(uri, params.text_document.version, params.text_document.text);
            publish_diagnostics(connection, host, uri)?;
        }
        DidChangeTextDocument::METHOD => {
            let params: lsp_types::DidChangeTextDocumentParams =
                serde_json::from_value(notif.params)?;
            let uri = params.text_document.uri.as_str();
            // Full sync — take the last content change.
            if let Some(change) = params.content_changes.into_iter().last() {
                host.update_document(uri, params.text_document.version, change.text);
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
    uri: &str,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let source = host.document_source(uri).unwrap_or("").to_string();
    let diags = host.diagnostics(uri);

    let lsp_diags: Vec<lsp_types::Diagnostic> = diags
        .iter()
        .map(|d| {
            let start = offset_to_position(&source, d.start_offset);
            let end = offset_to_position(&source, d.end_offset);
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
        uri: uri
            .parse()
            .unwrap_or_else(|_| format!("file:///{uri}").parse().unwrap()),
        diagnostics: lsp_diags,
        version: None,
    };

    let notif = Notification::new(PublishDiagnostics::METHOD.to_string(), params);
    connection.sender.send(Message::Notification(notif))?;
    Ok(())
}

/// Convert a byte offset to an LSP Position (line, character).
fn offset_to_position(source: &str, offset: usize) -> Position {
    let offset = offset.min(source.len());
    let mut line = 0u32;
    let mut col = 0u32;
    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += ch.len_utf8() as u32;
        }
    }
    Position::new(line, col)
}
