// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! LSP protocol server — stdio JSON-RPC message loop.

// `LspServer` is intentionally `pub` so it can be re-exported by `lsp/mod.rs`.
// The `server` submodule is private; items here are only reachable via that re-export.
#![allow(unreachable_pub)]

use std::error::Error;
use std::path::PathBuf;

use lsp_server::{Connection, Message, Notification, Request, Response};
use lsp_types::notification::{
    DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument, Notification as _,
    PublishDiagnostics,
};
use lsp_types::request::{
    Completion, Formatting, HoverRequest, Request as _, SemanticTokensFullRequest,
    SignatureHelpRequest,
};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionOptions, CompletionResponse, DiagnosticSeverity,
    Hover, HoverContents, HoverProviderCapability, InitializeParams, MarkupContent, MarkupKind,
    ParameterInformation, ParameterLabel, Position, PositionEncodingKind, Range, SemanticTokenType,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensResult,
    SemanticTokensServerCapabilities, ServerCapabilities, SignatureHelp, SignatureHelpOptions,
    SignatureInformation, TextDocumentSyncCapability, TextDocumentSyncKind, TextEdit, Uri,
};

use crate::dialect::AnyDialect;
use crate::fmt::FormatConfig;
use crate::lsp::{CompletionKind, LspHost, SEMANTIC_TOKEN_LEGEND};
use crate::semantic::diagnostics::Severity;

// ── LspServer ─────────────────────────────────────────────────────────────

/// Stdio LSP server for a syntaqlite dialect.
///
/// Runs a JSON-RPC message loop on stdin/stdout, driving an [`LspHost`]
/// for all analysis requests. Exits cleanly when the client sends a
/// `shutdown` request.
///
/// Use this when you want a turnkey LSP binary that editors can launch as a
/// child process. For programmatic access (e.g., in a web worker or test
/// harness), use [`LspHost`] directly instead.
///
/// # Supported capabilities
///
/// - `textDocument/didOpen`, `didChange`, `didClose`
/// - `textDocument/completion` (keywords and functions)
/// - `textDocument/hover` (table, column, and function info)
/// - `textDocument/signatureHelp` (function arities)
/// - `textDocument/semanticTokens/full`
/// - `textDocument/formatting`
/// - `textDocument/publishDiagnostics` (parse + semantic errors)
///
/// # Example
///
/// ```no_run
/// use syntaqlite::lsp::LspServer;
///
/// // Blocks on stdin/stdout — typically launched by an editor.
/// LspServer::run(syntaqlite::sqlite_dialect()).expect("LSP server failed");
/// ```
pub struct LspServer;

impl LspServer {
    /// Start the LSP server bound to `dialect` and block until shutdown.
    ///
    /// # Errors
    /// Returns `Err` if the LSP connection fails or an unrecoverable I/O error occurs.
    pub fn run(dialect: impl Into<AnyDialect>) -> Result<(), Box<dyn Error + Sync + Send>> {
        let dialect = dialect.into();
        let (connection, io_threads) = Connection::stdio();

        // VSCode only supports UTF-16 and UTF-32 position encodings.
        // Default to UTF-16 which is the LSP baseline.
        let server_capabilities = serde_json::to_value(ServerCapabilities {
            position_encoding: Some(PositionEncodingKind::UTF16),
            text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
            hover_provider: Some(HoverProviderCapability::Simple(true)),
            document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
            completion_provider: Some(CompletionOptions {
                trigger_characters: Some(vec![" ".into(), ".".into(), "\n".into(), "\t".into(), ";".into()]),
                ..Default::default()
            }),
            signature_help_provider: Some(SignatureHelpOptions {
                trigger_characters: Some(vec!["(".into(), ",".into()]),
                retrigger_characters: Some(vec![",".into()]),
                ..Default::default()
            }),
            semantic_tokens_provider: Some(
                SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    legend: SemanticTokensLegend {
                        token_types: SEMANTIC_TOKEN_LEGEND
                            .iter()
                            .map(|&name| SemanticTokenType::new(name))
                            .collect(),
                        token_modifiers: vec![],
                    },
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                    ..Default::default()
                }),
            ),
            ..Default::default()
        })?;

        let init_params_raw = connection.initialize(server_capabilities)?;
        let init_params: InitializeParams = serde_json::from_value(init_params_raw)?;

        if let Some(root) = Self::workspace_root(&init_params) {
            eprintln!("syntaqlite-lsp: workspace root: {}", root.display());
        }

        let mut host = LspHost::with_dialect(dialect);

        for msg in &connection.receiver {
            match msg {
                Message::Request(req) => {
                    if connection.handle_shutdown(&req)? {
                        return Ok(());
                    }
                    Self::handle_request(&connection, &mut host, req)?;
                }
                Message::Notification(notif) => {
                    Self::handle_notification(&connection, &mut host, notif)?;
                }
                Message::Response(_) => {}
            }
        }

        io_threads.join()?;
        Ok(())
    }

    // ── Request dispatch ──────────────────────────────────────────────────

    fn handle_request(
        connection: &Connection,
        host: &mut LspHost,
        req: Request,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        let response = match req.method.as_str() {
            Completion::METHOD => Self::handle_completion(req, host),
            HoverRequest::METHOD => Self::handle_hover(req, host),
            SignatureHelpRequest::METHOD => Self::handle_signature_help(req, host),
            Formatting::METHOD => Self::handle_formatting(req, host),
            SemanticTokensFullRequest::METHOD => Self::handle_semantic_tokens(req, host),
            _ => Response::new_err(
                req.id,
                lsp_server::ErrorCode::MethodNotFound as i32,
                format!("unknown request method: {}", req.method),
            ),
        };
        connection.sender.send(Message::Response(response))?;
        Ok(())
    }

    fn handle_completion(req: Request, host: &mut LspHost) -> Response {
        let params: lsp_types::CompletionParams = match serde_json::from_value(req.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    e.to_string(),
                );
            }
        };
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let uri_str = uri.as_str();

        match host.document_source(uri_str) {
            Some(source) => {
                let offset = SourcePositionMap::new(source).position_to_offset(position);
                let items = host
                    .completion_items(uri_str, offset)
                    .into_iter()
                    .map(|entry| CompletionItem {
                        label: entry.label().to_string(),
                        sort_text: Some(format!(
                            "{}_{}",
                            entry.kind().sort_priority(),
                            entry.label()
                        )),
                        kind: Some(match entry.kind() {
                            CompletionKind::Keyword => CompletionItemKind::KEYWORD,
                            CompletionKind::Function => CompletionItemKind::FUNCTION,
                            CompletionKind::Table => CompletionItemKind::STRUCT,
                            CompletionKind::Column => CompletionItemKind::FIELD,
                        }),
                        detail: Some(entry.kind().as_str().into()),
                        ..Default::default()
                    })
                    .collect();
                Response::new_ok(req.id, CompletionResponse::Array(items))
            }
            None => Response::new_ok(req.id, Option::<CompletionResponse>::None),
        }
    }

    fn handle_hover(req: Request, host: &mut LspHost) -> Response {
        let params: lsp_types::HoverParams = match serde_json::from_value(req.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    e.to_string(),
                );
            }
        };
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let uri_str = uri.as_str();

        let Some(source) = host.document_source(uri_str) else {
            return Response::new_ok(req.id, Option::<Hover>::None);
        };
        let offset = SourcePositionMap::new(source).position_to_offset(position);

        match host.hover_info(uri_str, offset) {
            Some((text, tok_offset, tok_length)) => {
                let source = host
                    .document_source(uri_str)
                    .expect("document must exist for hover");
                let map = SourcePositionMap::new(source);
                let positions = map.offsets_to_positions(&[tok_offset, tok_offset + tok_length]);
                let range = Range::new(positions[0], positions[1]);
                let hover = Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: text,
                    }),
                    range: Some(range),
                };
                Response::new_ok(req.id, hover)
            }
            None => Response::new_ok(req.id, Option::<Hover>::None),
        }
    }

    fn handle_signature_help(req: Request, host: &mut LspHost) -> Response {
        let params: lsp_types::SignatureHelpParams = match serde_json::from_value(req.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    e.to_string(),
                );
            }
        };
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let uri_str = uri.as_str();

        let Some(source) = host.document_source(uri_str) else {
            return Response::new_ok(req.id, Option::<SignatureHelp>::None);
        };
        let offset = SourcePositionMap::new(source).position_to_offset(position);

        match host.signature_help(uri_str, offset) {
            Some(info) => {
                use crate::semantic::catalog::AritySpec;

                let signatures: Vec<SignatureInformation> = info
                    .arities
                    .iter()
                    .map(|arity| {
                        let (label, params) = match arity {
                            AritySpec::Exact(n) => {
                                let names: Vec<String> =
                                    (0..*n).map(|i| format!("arg{}", i + 1)).collect();
                                let label = format!("{}({})", info.name, names.join(", "));
                                let params: Vec<ParameterInformation> = names
                                    .iter()
                                    .map(|name| ParameterInformation {
                                        label: ParameterLabel::Simple(name.clone()),
                                        documentation: None,
                                    })
                                    .collect();
                                (label, params)
                            }
                            AritySpec::AtLeast(n) => {
                                let mut names: Vec<String> =
                                    (0..*n).map(|i| format!("arg{}", i + 1)).collect();
                                names.push("...".to_string());
                                let label = format!("{}({})", info.name, names.join(", "));
                                let params: Vec<ParameterInformation> = names
                                    .iter()
                                    .map(|name| ParameterInformation {
                                        label: ParameterLabel::Simple(name.clone()),
                                        documentation: None,
                                    })
                                    .collect();
                                (label, params)
                            }
                            AritySpec::Any => {
                                let label = format!("{}(...)", info.name);
                                let params = vec![ParameterInformation {
                                    label: ParameterLabel::Simple("...".to_string()),
                                    documentation: None,
                                }];
                                (label, params)
                            }
                        };
                        SignatureInformation {
                            label,
                            documentation: None,
                            parameters: Some(params),
                            active_parameter: Some(info.active_parameter),
                        }
                    })
                    .collect();

                let help = SignatureHelp {
                    signatures,
                    active_signature: Some(0),
                    active_parameter: Some(info.active_parameter),
                };
                Response::new_ok(req.id, help)
            }
            None => Response::new_ok(req.id, Option::<SignatureHelp>::None),
        }
    }

    fn handle_formatting(req: Request, host: &mut LspHost) -> Response {
        let params: lsp_types::DocumentFormattingParams = match serde_json::from_value(req.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    e.to_string(),
                );
            }
        };
        let uri = params.text_document.uri.as_str();
        match host.format(uri, &FormatConfig::default()) {
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

    fn handle_semantic_tokens(req: Request, host: &mut LspHost) -> Response {
        let params: lsp_types::SemanticTokensParams = match serde_json::from_value(req.params) {
            Ok(p) => p,
            Err(e) => {
                return Response::new_err(
                    req.id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    e.to_string(),
                );
            }
        };
        let uri = params.text_document.uri.as_str();
        let encoded = host.semantic_tokens_encoded(uri, None);
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
        Response::new_ok(
            req.id,
            SemanticTokensResult::Tokens(lsp_types::SemanticTokens {
                result_id: None,
                data,
            }),
        )
    }

    // ── Notification dispatch ─────────────────────────────────────────────

    fn handle_notification(
        connection: &Connection,
        host: &mut LspHost,
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
                DiagnosticPublisher::publish(connection, host, uri)?;
            }
            DidChangeTextDocument::METHOD => {
                let params: lsp_types::DidChangeTextDocumentParams =
                    serde_json::from_value(notif.params)?;
                let uri = &params.text_document.uri;
                if let Some(change) = params.content_changes.into_iter().last() {
                    host.update_document(uri.as_str(), params.text_document.version, change.text);
                }
                DiagnosticPublisher::publish(connection, host, uri)?;
            }
            DidCloseTextDocument::METHOD => {
                let params: lsp_types::DidCloseTextDocumentParams =
                    serde_json::from_value(notif.params)?;
                let uri = params.text_document.uri;
                // Clear diagnostics before removing the document.
                let clear = lsp_types::PublishDiagnosticsParams {
                    uri: uri.clone(),
                    diagnostics: vec![],
                    version: None,
                };
                connection
                    .sender
                    .send(Message::Notification(Notification::new(
                        PublishDiagnostics::METHOD.to_string(),
                        clear,
                    )))?;
                host.close_document(uri.as_str());
            }
            _ => {}
        }
        Ok(())
    }

    // ── Helpers ───────────────────────────────────────────────────────────

    fn workspace_root(params: &InitializeParams) -> Option<PathBuf> {
        #[expect(deprecated)]
        if let Some(uri) = &params.root_uri {
            let s = uri.as_str();
            if let Some(path) = s.strip_prefix("file://") {
                return Some(PathBuf::from(path));
            }
        }
        #[expect(deprecated)]
        params.root_path.as_ref().map(PathBuf::from)
    }
}

// ── DiagnosticPublisher ───────────────────────────────────────────────────

/// Converts host diagnostics to LSP format and pushes them to the client.
struct DiagnosticPublisher;

impl DiagnosticPublisher {
    fn publish(
        connection: &Connection,
        host: &mut LspHost,
        uri: &Uri,
    ) -> Result<(), Box<dyn Error + Sync + Send>> {
        let uri_str = uri.as_str();
        let Some((version, source, diags)) = host.document_all_diagnostics(uri_str) else {
            return Ok(());
        };

        // Collect all offsets and convert in a single O(n) pass.
        let mut offsets: Vec<usize> = Vec::with_capacity(diags.len() * 2);
        for d in &diags {
            offsets.push(d.start_offset());
            offsets.push(d.end_offset());
        }
        let map = SourcePositionMap::new(&source);
        let positions = map.offsets_to_positions(&offsets);

        let lsp_diags: Vec<lsp_types::Diagnostic> = diags
            .iter()
            .enumerate()
            .map(|(i, d)| lsp_types::Diagnostic {
                range: Range::new(positions[i * 2], positions[i * 2 + 1]),
                severity: Some(match d.severity() {
                    Severity::Error => DiagnosticSeverity::ERROR,
                    Severity::Warning => DiagnosticSeverity::WARNING,
                    Severity::Info => DiagnosticSeverity::INFORMATION,
                    Severity::Hint => DiagnosticSeverity::HINT,
                }),
                message: d.message().to_string(),
                source: Some("syntaqlite".to_string()),
                ..Default::default()
            })
            .collect();

        let params = lsp_types::PublishDiagnosticsParams {
            uri: uri.clone(),
            diagnostics: lsp_diags,
            version: Some(version),
        };
        connection
            .sender
            .send(Message::Notification(Notification::new(
                PublishDiagnostics::METHOD.to_string(),
                params,
            )))?;
        Ok(())
    }
}

// ── SourcePositionMap ─────────────────────────────────────────────────────

/// Converts between byte offsets and LSP `Position` (line/character) values
/// for a fixed source string.
///
/// Both directions use a single O(n) walk over the source bytes.
pub(super) struct SourcePositionMap<'a> {
    src: &'a [u8],
}

impl<'a> SourcePositionMap<'a> {
    pub(crate) fn new(source: &'a str) -> Self {
        SourcePositionMap {
            src: source.as_bytes(),
        }
    }

    /// Convert multiple byte offsets to LSP `Position`s in one O(n) pass.
    ///
    /// Internally sorts the offsets, walks the source once, then returns
    /// results in the original order.
    pub(crate) fn offsets_to_positions(&self, offsets: &[usize]) -> Vec<Position> {
        if offsets.is_empty() {
            return Vec::new();
        }

        let mut indexed: Vec<(usize, usize)> = offsets
            .iter()
            .copied()
            .enumerate()
            .map(|(i, o)| (o, i))
            .collect();
        indexed.sort_unstable_by_key(|&(o, _)| o);

        let mut result = vec![Position::new(0, 0); offsets.len()];
        let len = self.src.len();
        let mut line = 0u32;
        let mut col = 0u32;
        let mut pos = 0usize;

        for (offset, orig_idx) in indexed {
            let offset = offset.min(len);
            while pos < offset {
                if self.src[pos] == b'\n' {
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

    /// Convert an LSP `Position` to a byte offset.
    pub(crate) fn position_to_offset(&self, pos: Position) -> usize {
        let len = self.src.len();
        let mut line = 0usize;
        let mut line_start = 0usize;

        while line < pos.line as usize && line_start < len {
            match self.src[line_start..].iter().position(|&b| b == b'\n') {
                Some(nl) => {
                    line_start += nl + 1;
                    line += 1;
                }
                None => return len,
            }
        }

        let line_end = self.src[line_start..]
            .iter()
            .position(|&b| b == b'\n')
            .map_or(len, |rel| line_start + rel);

        line_start + (pos.character as usize).min(line_end - line_start)
    }
}
