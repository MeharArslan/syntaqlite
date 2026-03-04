// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Incremental (token-feeding) parser API.
//
// An alternative to syntaqlite_parser_next() for embedders that perform their
// own tokenization — for example, to support macro expansion before parsing.
// Feed tokens one at a time after calling syntaqlite_parser_reset(); the
// parser signals statement boundaries as tokens arrive.
//
// When feed_token or finish returns SYNTAQLITE_PARSE_OK or _RECOVERED, read
// the result via the syntaqlite_result_*() accessors (defined in parser.h).
// The result is valid until the next feed_token, finish, reset, or destroy.
//
// Usage:
//   SyntaqliteParser* p = syntaqlite_create_sqlite_parser(NULL);
//   syntaqlite_parser_reset(p, source, len);
//   while (has_more_tokens) {
//     int32_t rc = syntaqlite_parser_feed_token(p, type, text, tlen);
//     if (rc == SYNTAQLITE_PARSE_OK || rc == SYNTAQLITE_PARSE_RECOVERED) {
//       uint32_t root = syntaqlite_result_root(p);
//       // read nodes ...
//     }
//     if (rc == SYNTAQLITE_PARSE_ERROR) { /* handle error */ break; }
//   }
//   int32_t rc = syntaqlite_parser_finish(p);
//   if (rc == SYNTAQLITE_PARSE_OK) { /* final statement complete */ }
//   syntaqlite_parser_destroy(p);
//
// For macro region tracking, bracket expanded tokens with
// syntaqlite_parser_begin_macro() / syntaqlite_parser_end_macro(). Read
// accumulated regions via syntaqlite_result_macros() after parsing.

#ifndef SYNTAQLITE_INCREMENTAL_PARSER_H
#define SYNTAQLITE_INCREMENTAL_PARSER_H

#include "syntaqlite/parser.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Token feeding
// ---------------------------------------------------------------------------

// Feed a single token. TK_SPACE is silently skipped. TK_COMMENT is recorded
// as a comment (when collect_tokens is enabled) but not fed to the parser.
//
// Returns a SYNTAQLITE_PARSE_* code:
//   DONE      = keep going (statement not yet complete)
//   OK        = statement completed cleanly
//   RECOVERED = statement completed with error recovery
//   ERROR     = unrecoverable parse error
int32_t syntaqlite_parser_feed_token(SyntaqliteParser* p,
                                     uint32_t token_type,
                                     const char* text,
                                     uint32_t len);

// Signal end-of-input. Synthesizes a SEMI if needed and sends EOF to the
// parser. Returns a SYNTAQLITE_PARSE_* code.
int32_t syntaqlite_parser_finish(SyntaqliteParser* p);

// ---------------------------------------------------------------------------
// Completion / lookahead
// ---------------------------------------------------------------------------

// Enumerate terminal tokens that are valid next lookaheads at the parser's
// current state. Returns the total number of expected tokens.
// If out_tokens is non-NULL, up to out_cap token IDs are written.
uint32_t syntaqlite_parser_expected_tokens(SyntaqliteParser* p,
                                           uint32_t* out_tokens,
                                           uint32_t out_cap);

// Return the semantic completion context at the parser's current state.
// 0 = Unknown, 1 = Expression, 2 = TableRef.
uint32_t syntaqlite_parser_completion_context(SyntaqliteParser* p);

// ---------------------------------------------------------------------------
// Macro region tracking
// ---------------------------------------------------------------------------

// Mark subsequent fed tokens as being inside a macro expansion.
// call_offset/call_length describe the macro call's byte range in the
// original source. Calls may nest (for nested macro expansions).
void syntaqlite_parser_begin_macro(SyntaqliteParser* p,
                                   uint32_t call_offset,
                                   uint32_t call_length);

// End the innermost macro expansion region.
void syntaqlite_parser_end_macro(SyntaqliteParser* p);

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_INCREMENTAL_PARSER_H
