// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Incremental (token-feeding) parser API.
//
// An alternative to syntaqlite_parser_next() for embedders that perform their
// own tokenization — for example, to support macro expansion before parsing.
// Feed tokens one at a time after calling syntaqlite_parser_reset(); the
// parser signals statement boundaries as tokens arrive.
//
// Usage:
//   SyntaqliteParser* p = syntaqlite_create_sqlite_parser(NULL);
//   syntaqlite_parser_reset(p, source, len);
//   while (has_more_tokens) {
//     int rc = syntaqlite_parser_feed_token(p, type, text, tlen);
//     if (rc == 1) {
//       SyntaqliteParseResult r = syntaqlite_parser_result(p);
//       // read nodes from arena ...
//     }
//     if (rc < 0) { /* parse error */ }
//   }
//   int rc = syntaqlite_parser_finish(p);
//   if (rc == 1) { /* final statement complete */ }
//   syntaqlite_parser_destroy(p);
//
// For macro region tracking, bracket expanded tokens with
// syntaqlite_parser_begin_macro() / syntaqlite_parser_end_macro(). Read
// accumulated regions with syntaqlite_parser_macro_regions() after parsing.

#ifndef SYNTAQLITE_INCREMENTAL_PARSER_H
#define SYNTAQLITE_INCREMENTAL_PARSER_H

#include "syntaqlite/parser.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// A recorded macro invocation region, populated via begin_macro / end_macro.
// The formatter uses these to reconstruct macro calls from the expanded AST.
typedef struct SyntaqliteMacroRegion {
  uint32_t call_offset;  // Byte offset of macro call in original source.
  uint32_t call_length;  // Byte length of entire macro call.
} SyntaqliteMacroRegion;

// ---------------------------------------------------------------------------
// Token feeding
// ---------------------------------------------------------------------------

// Feed a single token. TK_SPACE is silently skipped. TK_COMMENT is recorded
// as a comment (when collect_tokens is enabled) but not fed to the parser.
// Returns: 0 = keep going, 1 = statement completed, -1 = error.
int syntaqlite_parser_feed_token(SyntaqliteParser* p,
                                 int token_type,
                                 const char* text,
                                 int len);

// Signal end-of-input. Synthesizes a SEMI if needed and sends EOF to the
// parser. Returns: 0 = done (no pending statement), 1 = final statement
// completed, -1 = error.
int syntaqlite_parser_finish(SyntaqliteParser* p);

// Retrieve the parse result after feed_token returns 1 or after finish().
SyntaqliteParseResult syntaqlite_parser_result(SyntaqliteParser* p);

// ---------------------------------------------------------------------------
// Completion / lookahead
// ---------------------------------------------------------------------------

// Enumerate terminal tokens that are valid next lookaheads at the parser's
// current state. Returns the total number of expected tokens.
// If out_tokens is non-NULL, up to out_cap token IDs are written.
// Intended for grammar-aware completion engines.
int syntaqlite_parser_expected_tokens(SyntaqliteParser* p,
                                      int* out_tokens,
                                      int out_cap);

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

// Return the macro regions recorded via begin_macro/end_macro. The returned
// pointer is valid until the next reset() or destroy(). Sets *count to the
// number of regions.
const SyntaqliteMacroRegion* syntaqlite_parser_macro_regions(
    SyntaqliteParser* p,
    uint32_t* count);

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_INCREMENTAL_PARSER_H
