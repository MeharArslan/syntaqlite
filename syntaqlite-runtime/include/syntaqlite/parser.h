// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Streaming parser for SQL — the main entry point for AST access.
//
// Produces a typed AST from SQL text. Each call to syntaqlite_parser_next()
// parses one statement and returns the root node ID. All nodes live in an
// internal arena and remain valid until the next reset() or destroy().
//
// Lifecycle: create → [configure] → reset → next (loop) → read nodes → destroy.
// A single parser can be reused across inputs by calling reset() again.
//
// Usage:
//   SyntaqliteParser* p = syntaqlite_parser_create(NULL);
//   syntaqlite_parser_reset(p, sql, len);
//   SyntaqliteParseResult r;
//   while ((r = syntaqlite_parser_next(p)).root != SYNTAQLITE_NULL_NODE) {
//     const void* node = syntaqlite_parser_node(p, r.root);
//     // cast to dialect-specific node type and switch on tag ...
//   }
//   if (r.error) { /* handle r.error_msg */ }
//   syntaqlite_parser_destroy(p);
//
// With token collection (required for formatting):
//   SyntaqliteParser* p = syntaqlite_parser_create(NULL);
//   syntaqlite_parser_set_collect_tokens(p, 1);
//   // ... parse as above, then pass to formatter ...

#ifndef SYNTAQLITE_PARSER_H
#define SYNTAQLITE_PARSER_H

#include <stdint.h>
#include <stdio.h>

#include "syntaqlite/config.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// Opaque parser handle (heap-allocated, reusable across inputs).
typedef struct SyntaqliteParser SyntaqliteParser;

// Opaque dialect handle — produced by dialect crates (e.g. syntaqlite_sqlite_dialect()).
typedef struct SyntaqliteDialect SyntaqliteDialect;

// A trivia item (comment) captured during parsing.
typedef struct SyntaqliteTrivia {
  uint32_t offset;   // Byte offset in source.
  uint32_t length;   // Byte length.
  uint8_t kind;      // 0 = line comment (--), 1 = block comment (/* */).
} SyntaqliteTrivia;

// Result of parsing one statement via syntaqlite_parser_next().
//
// Check root first: if it is SYNTAQLITE_NULL_NODE, parsing is done — then
// check error to see whether it ended cleanly or with a parse error.
typedef struct SyntaqliteParseResult {
  uint32_t root;          // Root node ID, or SYNTAQLITE_NULL_NODE.
  int32_t error;          // Nonzero if a parse error occurred.
  const char* error_msg;  // Human-readable message (owned by parser), or NULL.
} SyntaqliteParseResult;

// A recorded macro invocation region, populated via the low-level API
// (begin_macro / end_macro). The formatter uses these to reconstruct macro
// calls from the expanded AST.
typedef struct SyntaqliteMacroRegion {
  uint32_t call_offset;    // Byte offset of macro call in original source.
  uint32_t call_length;    // Byte length of entire macro call.
} SyntaqliteMacroRegion;

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

// Allocate a parser for a specific dialect. The parser is inert until reset()
// is called. The mem methods are copied — the caller's struct does not need
// to outlive the parser. Pass NULL for mem defaults (malloc/free). The dialect
// pointer must remain valid for the lifetime of the parser.
SyntaqliteParser* syntaqlite_create_parser_with_dialect(
    const SyntaqliteMemMethods* mem, const SyntaqliteDialect* dialect);

// Bind a source buffer and reset all internal state. The source must remain
// valid until the next reset() or destroy(). Can be called again to parse a
// new input without reallocating — all previous nodes are invalidated.
void syntaqlite_parser_reset(SyntaqliteParser* p,
                             const char* source,
                             uint32_t len);

// Parse the next SQL statement. Call in a loop until root is
// SYNTAQLITE_NULL_NODE. Bare semicolons between statements are skipped
// automatically. Each call appends nodes to the arena; nodes from all
// statements remain valid until the next reset() or destroy().
SyntaqliteParseResult syntaqlite_parser_next(SyntaqliteParser* p);

// Free the parser, its arena, and all its nodes. No-op if p is NULL.
void syntaqlite_parser_destroy(SyntaqliteParser* p);

// ---------------------------------------------------------------------------
// Reading results
// ---------------------------------------------------------------------------

// Look up a node by its arena ID. The returned pointer is valid until the
// next reset() or destroy(). Cast to the dialect-specific node union type
// and use the tag field to determine which member to read.
const void* syntaqlite_parser_node(SyntaqliteParser* p,
                                   uint32_t node_id);

// Return a pointer to the source text bound by the last reset() call.
// Useful for extracting token text via SyntaqliteSourceSpan offsets:
//   syntaqlite_parser_source(p) + span.offset
const char* syntaqlite_parser_source(SyntaqliteParser* p);

// Return the byte length of the source text bound by the last reset() call.
uint32_t syntaqlite_parser_source_length(SyntaqliteParser* p);

// Return the trivia (comments) captured during parsing. The returned pointer
// is valid until the next reset() or destroy(). Requires collect_tokens to be
// enabled. Sets *count to the number of trivia items.
const SyntaqliteTrivia* syntaqlite_parser_trivia(SyntaqliteParser* p,
                                                  uint32_t* count);

// Return the macro regions recorded via begin_macro/end_macro. The returned
// pointer is valid until the next reset() or destroy(). Sets *count to the
// number of regions.
const SyntaqliteMacroRegion* syntaqlite_parser_macro_regions(
    SyntaqliteParser* p, uint32_t* count);

// ---------------------------------------------------------------------------
// AST dump
// ---------------------------------------------------------------------------

// Dump an AST node tree as indented text. Returns a malloc'd NUL-terminated
// string. The caller must free() the result. Returns NULL on allocation failure.
char* syntaqlite_dump_node(SyntaqliteParser* p, uint32_t node_id,
                           uint32_t indent);

// ---------------------------------------------------------------------------
// Configuration (call after create, before first reset)
// ---------------------------------------------------------------------------

// Enable parser trace output (debug builds only). When enabled, the parser
// prints shift/reduce actions to stderr. Useful for diagnosing grammar
// conflicts or unexpected parses. Default: off (0).
// Returns 0 on success, -1 if the parser has already been used.
int syntaqlite_parser_set_trace(SyntaqliteParser* p, int enable);

// Enable token collection. When enabled, the parser records every token
// (including whitespace and comments) so the formatter can reproduce the
// original layout. Required before passing the parser to the formatter.
// Default: off (0).
// Returns 0 on success, -1 if the parser has already been used.
int syntaqlite_parser_set_collect_tokens(SyntaqliteParser* p, int enable);


// ---------------------------------------------------------------------------
// Low-level token-feeding API
// ---------------------------------------------------------------------------
//
// Alternative to syntaqlite_parser_next() for embedders that perform their
// own tokenization (e.g. macro expansion). Call reset() first to bind a
// source buffer, then feed tokens one at a time.
//
// Usage:
//   syntaqlite_parser_reset(p, source, len);
//   while (has_more_tokens) {
//     int rc = syntaqlite_parser_feed_token(p, type, text, tlen);
//     if (rc == 1) { /* statement complete, read result */ }
//     if (rc < 0) { /* error */ }
//   }
//   int rc = syntaqlite_parser_finish(p);
//   if (rc == 1) { /* final statement complete */ }

// Feed a single token. TK_SPACE is silently skipped. TK_COMMENT is recorded
// as trivia (when collect_tokens is enabled) but not fed to the parser.
// Returns: 0 = keep going, 1 = statement completed, -1 = error.
int syntaqlite_parser_feed_token(SyntaqliteParser* p,
                                  int token_type,
                                  const char* text,
                                  int len);

// Retrieve the parse result after feed_token returns 1 or after finish().
SyntaqliteParseResult syntaqlite_parser_result(SyntaqliteParser* p);

// Signal end-of-input. Synthesizes a SEMI if needed and sends EOF to the
// parser. Returns: 0 = done (no pending statement), 1 = final statement
// completed, -1 = error.
int syntaqlite_parser_finish(SyntaqliteParser* p);

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

#endif  // SYNTAQLITE_PARSER_H
