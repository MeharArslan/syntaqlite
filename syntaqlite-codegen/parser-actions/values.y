// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// AST building actions for syntaqlite grammar.
// These rules get merged with SQLite's parse.y during code generation.
//
// Rule signatures MUST match upstream parse.y exactly.
// Python tooling validates coverage and consistency.
//
// Conventions:
// - pCtx: Parse context (SynqParseContext*)
// - pCtx->zSql: Original SQL text (for computing offsets)
// - pCtx->root: Set to root node ID at input rule
// - Terminals are SynqParseToken with .z (pointer) and .n (length)
// - Non-terminals are u32 node IDs

// ============ VALUES clause ============

// Single-row VALUES: produces a ValuesRowList with one row (nexprlist).
values(A) ::= VALUES LP nexprlist(X) RP. {
    A = synq_parse_values_row_list(pCtx, SYNTAQLITE_NULL_NODE, X);
}

// Multi-row VALUES: append a row to existing ValuesRowList.
mvalues(A) ::= values(A) COMMA LP nexprlist(Y) RP. {
    A = synq_parse_values_row_list(pCtx, A, Y);
}

mvalues(A) ::= mvalues(A) COMMA LP nexprlist(Y) RP. {
    A = synq_parse_values_row_list(pCtx, A, Y);
}

// Wrap ValuesRowList into a ValuesClause at the oneselect level.
oneselect(A) ::= values(B). {
    A = synq_parse_values_clause(pCtx, B);
}

oneselect(A) ::= mvalues(B). {
    A = synq_parse_values_clause(pCtx, B);
}