// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use syntaqlite::dialect::Dialect;
use syntaqlite::lsp::LspServer;

pub(crate) fn cmd_lsp(dialect: Dialect<'_>) -> Result<(), String> {
    LspServer::run(dialect).map_err(|e| format!("LSP error: {e}"))
}
