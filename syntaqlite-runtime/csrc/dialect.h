// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Dialect vtable: function pointers and config that a concrete grammar
// (e.g. SQLite) must provide to the generic parser engine.

#ifndef SYNQ_DIALECT_H
#define SYNQ_DIALECT_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "csrc/parser.h"
#include "syntaqlite/parser.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SynqDialect {
    // Lemon parser lifecycle.
    void* (*lemon_alloc)(void* (*mallocProc)(size_t));
    void (*lemon_init)(void* parser);
    void (*lemon_finalize)(void* parser);
    void (*lemon_free)(void* parser, void (*freeProc)(void*));
    void (*lemon_parse)(void* parser, int token_type, SynqToken minor,
                        SynqParseCtx* ctx);
    // NULL in release builds or if tracing is unsupported.
    void (*lemon_trace)(FILE* trace_file, char* prompt);

    // Range metadata for the macro straddle check.
    // Indexed by node tag; table has `node_count` entries.
    const SynqRangeMetaEntry* range_meta;
    uint32_t node_count;

    // Well-known token IDs.
    int tk_space;
    int tk_semi;
    int tk_comment;
} SynqDialect;

#ifdef __cplusplus
}
#endif

#endif  // SYNQ_DIALECT_H
