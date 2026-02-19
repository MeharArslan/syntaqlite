// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// SQLite dialect: parser vtable, AST metadata, and formatter bytecode,
// all wired into a single SyntaqliteDialect struct.

#include "syntaqlite/parser.h"
#include "syntaqlite/sqlite_tokens.h"
#include "syntaqlite/dialect.h"
#include "csrc/dialect_builder.h"
#include "csrc/dialect_parse.h"
#include "csrc/dialect_meta.h"
#include "csrc/dialect_fmt.h"

// ---------------------------------------------------------------------------
// SQLite dialect descriptor
// ---------------------------------------------------------------------------

static const SyntaqliteDialect SQLITE_DIALECT = {
    .name = "sqlite",

    // Parse tables + reduce actions
    .tables = &SQLITE_PARSE_TABLES,
    .reduce_actions = (SynqReduceActionsFn)yy_reduce_actions,

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

    // Formatter data
    .fmt_strings = fmt_strings,
    .fmt_string_count = sizeof(fmt_strings) / sizeof(fmt_strings[0]),
    .fmt_enum_display = fmt_enum_display,
    .fmt_enum_display_count = sizeof(fmt_enum_display) / sizeof(fmt_enum_display[0]),
    .fmt_ops = fmt_ops,
    .fmt_op_count = sizeof(fmt_ops) / 6,
    .fmt_dispatch = fmt_dispatch,
    .fmt_dispatch_count = sizeof(fmt_dispatch) / sizeof(fmt_dispatch[0]),
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
