// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// SQLite dialect: parser vtable, AST metadata, and formatter bytecode,
// all wired into a single SyntaqliteDialect struct.

#include "syntaqlite/parser.h"
#include "syntaqlite/tokens.h"
#include "syntaqlite/dialect.h"
#include "csrc/ast_builder.h"
#include "csrc/sqlite_parser.h"
#include "csrc/sqlite_dialect_data.h"
#include "csrc/fmt_data.h"

// ---------------------------------------------------------------------------
// SQLite dialect descriptor
// ---------------------------------------------------------------------------

static const SyntaqliteDialect SQLITE_DIALECT = {
    .name = "sqlite",

    // Parser vtable (Lemon lifecycle)
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
    .tk_space = SYNTAQLITE_TK_SPACE,
    .tk_semi = SYNTAQLITE_TK_SEMI,
    .tk_comment = SYNTAQLITE_TK_COMMENT,

    // AST metadata
    .node_count = sizeof(ast_meta_node_names) / sizeof(ast_meta_node_names[0]),
    .node_names = ast_meta_node_names,
    .field_meta = ast_meta_field_meta,
    .field_meta_counts = ast_meta_field_meta_counts,
    .list_tags = ast_meta_list_tags,

    // Formatter bytecode
    .fmt_data = fmt_bytecode_data,
    .fmt_data_len = fmt_bytecode_len,
};

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

const SyntaqliteDialect* syntaqlite_sqlite_dialect(void) {
    return &SQLITE_DIALECT;
}

// Convenience: create a parser pre-configured for the SQLite dialect.
SyntaqliteParser* syntaqlite_create_parser(const SyntaqliteMemMethods* mem) {
    return syntaqlite_create_parser_with_dialect(mem, &SQLITE_DIALECT);
}
