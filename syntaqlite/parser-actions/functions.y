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

// ============ Function Calls ============

// Function call with arguments: func(args) or func(DISTINCT args)
expr(A) ::= idj(B) LP distinct(C) exprlist(D) RP. {
    A = synq_parse_function_call(pCtx,
        synq_span(pCtx, B),
        (SyntaqliteFunctionCallFlags){.raw = (uint8_t)C},
        D,
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE);
}

// Function call with star: COUNT(*)
expr(A) ::= idj(B) LP STAR RP. {
    A = synq_parse_function_call(pCtx,
        synq_span(pCtx, B),
        (SyntaqliteFunctionCallFlags){.bits = {.star = 1}},
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE,
        SYNTAQLITE_NULL_NODE);
}

// Function call with arguments and filter/over: func(args) FILTER/OVER
expr(A) ::= idj(B) LP distinct(C) exprlist(D) RP filter_over(E). {
    SyntaqliteFilterOver *fo = (SyntaqliteFilterOver*)synq_arena_ptr(&pCtx->ast, E);
    A = synq_parse_function_call(pCtx,
        synq_span(pCtx, B),
        (SyntaqliteFunctionCallFlags){.raw = (uint8_t)C},
        D,
        fo->filter_expr,
        fo->over_def);
}

// Function call with star and filter/over: COUNT(*) FILTER/OVER
expr(A) ::= idj(B) LP STAR RP filter_over(C). {
    SyntaqliteFilterOver *fo = (SyntaqliteFilterOver*)synq_arena_ptr(&pCtx->ast, C);
    A = synq_parse_function_call(pCtx,
        synq_span(pCtx, B),
        (SyntaqliteFunctionCallFlags){.bits = {.star = 1}},
        SYNTAQLITE_NULL_NODE,
        fo->filter_expr,
        fo->over_def);
}