// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Streaming tokenizer for SQLite SQL.
//
// The lowest-level entry point — splits SQL text into a flat sequence of
// tokens without any parsing or tree structure. Most users don't need this
// directly; the parser (parser.h) and formatter (formatter.h) handle
// tokenization internally. Use this when you need raw token access
// (syntax highlighting, custom analysis, etc.).
//
// Lifecycle: create → reset → next (loop) → destroy.
//
// Usage:
//   SyntaqliteTokenizer* tok = syntaqlite_tokenizer_create(NULL);
//   syntaqlite_tokenizer_reset(tok, sql, len);
//   SyntaqliteToken token;
//   while (syntaqlite_tokenizer_next(tok, &token)) {
//     // process token.type, token.text, token.length
//   }
//   syntaqlite_tokenizer_destroy(tok);

#ifndef SYNTAQLITE_TOKENIZER_H
#define SYNTAQLITE_TOKENIZER_H

#include <stdint.h>

#include "syntaqlite/config.h"
#include "syntaqlite/dialect.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// Opaque tokenizer handle (heap-allocated, reusable across inputs).
typedef struct SyntaqliteTokenizer SyntaqliteTokenizer;

// A single token produced by the tokenizer. The text pointer points into
// the source buffer bound by the last reset() call, so it is only valid
// while that buffer is alive. The text is NOT null-terminated.
typedef struct SyntaqliteToken {
  const char* text;  // Pointer into source text.
  uint32_t length;   // Token length in bytes.
  uint32_t type;     // Token type (SYNTAQLITE_TK_* from tokens.h).
} SyntaqliteToken;

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

// Forward declaration.
typedef struct SyntaqliteDialect SyntaqliteDialect;

// Allocate a tokenizer bound to a dialect. The tokenizer is inert until
// reset() is called. The dialect must outlive the tokenizer.
// The mem methods are copied — pass NULL for all defaults (malloc/free).
SyntaqliteTokenizer* syntaqlite_tokenizer_create(
    const SyntaqliteMemMethods* mem,
    const SyntaqliteDialect* dialect);

// Bind a source buffer and start tokenizing from the beginning. The source
// must remain valid until the next reset() or destroy(). Can be called
// again to tokenize a new input without reallocating.
void syntaqlite_tokenizer_reset(SyntaqliteTokenizer* tok,
                                const char* source,
                                uint32_t len);

// Advance to the next token. Returns 1 if a token was written to *out,
// 0 at end-of-input. Every token is returned, including whitespace and
// comments.
int syntaqlite_tokenizer_next(SyntaqliteTokenizer* tok, SyntaqliteToken* out);

// Free the tokenizer and all its memory. No-op if tok is NULL.
void syntaqlite_tokenizer_destroy(SyntaqliteTokenizer* tok);

// Set the dialect config for version/cflag-gated tokenization.
// The config is copied — the caller's struct does not need to outlive the
// tokenizer. Default: latest version (INT32_MAX), no cflags.
// Returns 0 on success.
int syntaqlite_tokenizer_set_dialect_config(
    SyntaqliteTokenizer* tok,
    const SyntaqliteDialectConfig* config);

// ---------------------------------------------------------------------------
// SQLite dialect convenience (opt-out: -DSYNTAQLITE_OMIT_SQLITE_API)
// ---------------------------------------------------------------------------

#ifndef SYNTAQLITE_OMIT_SQLITE_API
const SyntaqliteDialect* syntaqlite_sqlite_dialect(void);
static inline SyntaqliteTokenizer* syntaqlite_create_sqlite_tokenizer(
    const SyntaqliteMemMethods* mem) {
  return syntaqlite_tokenizer_create(mem, syntaqlite_sqlite_dialect());
}
#endif

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_TOKENIZER_H
