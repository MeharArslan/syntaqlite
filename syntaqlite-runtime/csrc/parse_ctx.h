// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Parse context lifecycle functions (runtime-internal).

#ifndef SYNTAQLITE_INTERNAL_PARSE_CTX_H
#define SYNTAQLITE_INTERNAL_PARSE_CTX_H

#include "syntaqlite_ext/ast_builder.h"

#ifdef __cplusplus
extern "C" {
#endif

void synq_parse_ctx_init(SynqParseCtx* ctx, SyntaqliteMemMethods mem);
void synq_parse_ctx_free(SynqParseCtx* ctx);

// Reset to empty state, keeping allocated memory for reuse.
void synq_parse_ctx_clear(SynqParseCtx* ctx);

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_INTERNAL_PARSE_CTX_H
