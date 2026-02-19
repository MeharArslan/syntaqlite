// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// SQLite dialect: vtable instance + public API wrapper that injects the
// dialect into the generic engine's create function.

#include "syntaqlite/parser.h"
#include "syntaqlite/tokens.h"
#include "csrc/ast_builder.h"
#include "csrc/sqlite_parser.h"
#include "csrc/dialect.h"

// ---------------------------------------------------------------------------
// Dialect vtable
// ---------------------------------------------------------------------------

static const SynqDialect SQLITE_DIALECT = {
    .lemon_alloc = SyntaqliteParseAlloc,
    .lemon_init = SyntaqliteParseInit,
    .lemon_finalize = SyntaqliteParseFinalize,
    .lemon_free = SyntaqliteParseFree,
    .lemon_parse = SyntaqliteParse,
#ifndef NDEBUG
    .lemon_trace = SyntaqliteParseTrace,
#else
    .lemon_trace = NULL,
#endif
    .range_meta = range_meta_table,
    .node_count = SYNTAQLITE_NODE_COUNT,
    .tk_space = SYNTAQLITE_TK_SPACE,
    .tk_semi = SYNTAQLITE_TK_SEMI,
    .tk_comment = SYNTAQLITE_TK_COMMENT,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

// Return the SQLite dialect instance.
const SynqDialect* syntaqlite_sqlite_dialect(void) {
    return &SQLITE_DIALECT;
}

// Convenience: create a parser pre-configured for the SQLite dialect.
SyntaqliteParser* syntaqlite_create_parser(const SyntaqliteMemMethods* mem) {
    return syntaqlite_create_parser_with_dialect(mem, &SQLITE_DIALECT);
}
