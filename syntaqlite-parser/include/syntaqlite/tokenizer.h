// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Streaming tokenizer for SQLite SQL.
//
// This is the lowest-level entry point — it splits SQL text into tokens
// without any parsing or structure. Most users don't need this directly;
// the parser (parser.h) and formatter (formatter.h) handle tokenization
// internally. Use this when you need raw token access (syntax highlighting,
// custom analysis, etc.).
//
// Usage:
//   SyntaqliteConfig config = SYNTAQLITE_CONFIG_DEFAULT;
//   SyntaqliteTokenizer *tok = syntaqlite_tokenizer_create(&config);
//   syntaqlite_tokenizer_reset(tok, sql, len);
//   SyntaqliteToken token;
//   while (syntaqlite_tokenizer_next(tok, &token)) {
//     // process token
//   }
//   syntaqlite_tokenizer_destroy(tok);

#ifndef SYNTAQLITE_TOKENIZER_H
#define SYNTAQLITE_TOKENIZER_H

#include <stdint.h>

#include "syntaqlite/config.h"

#ifdef __cplusplus
extern "C" {
#endif

// Token returned by the tokenizer. The text pointer points into the source
// buffer passed to reset(), so it is only valid while that buffer is alive.
typedef struct SyntaqliteToken {
  const char* text;  // Pointer into source text (not null-terminated)
  uint32_t length;   // Token length in bytes
  uint16_t type;     // SYNTAQLITE_TK_* token type
} SyntaqliteToken;

// Opaque tokenizer handle.
typedef struct SyntaqliteTokenizer SyntaqliteTokenizer;

// --- Lifecycle ---

// 1. Allocate a tokenizer. The tokenizer is inert until reset() is called.
//    The config is copied — the caller's SyntaqliteConfig does not need to
//    outlive the tokenizer. Pass NULL for all defaults.
SyntaqliteTokenizer* syntaqlite_tokenizer_create(const SyntaqliteConfig* config);

// 2. Bind a source buffer. The cursor starts at the beginning. The source
//    must remain valid until the next reset() or destroy(). Can be called
//    again to tokenize a new input without reallocating.
void syntaqlite_tokenizer_reset(SyntaqliteTokenizer* tok,
                                const char* source,
                                uint32_t len);

// 3. Advance to the next token. Returns 1 if a token was written to *out,
//    0 at end-of-input. Every token is returned, including whitespace
//    (SYNTAQLITE_TOKEN_SPACE) and comments (SYNTAQLITE_TOKEN_COMMENT).
int syntaqlite_tokenizer_next(SyntaqliteTokenizer* tok, SyntaqliteToken* out);

// 4. Free the tokenizer. No-op if tok is NULL.
void syntaqlite_tokenizer_destroy(SyntaqliteTokenizer* tok);

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_TOKENIZER_H
