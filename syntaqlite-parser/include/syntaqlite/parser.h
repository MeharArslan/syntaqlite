// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Streaming parser for SQLite SQL — the main entry point for AST access.
//
// Lifecycle: create → reset → next (loop) → read nodes → destroy.
// A single parser can be reused across inputs by calling reset() again.
//
// Node ownership: all nodes live in the parser's internal arena and remain
// valid until the next reset() or destroy() call. Access them via
// syntaqlite_parser_node().
//
// If you plan to format afterwards, set config.parser.collect_tokens = 1
// so the formatter can preserve comments and whitespace decisions.
//
// Usage:
//   SyntaqliteConfig config = SYNTAQLITE_CONFIG_DEFAULT;
//   SyntaqliteParser *p = syntaqlite_parser_create(&config);
//   syntaqlite_parser_reset(p, sql, len);
//   SyntaqliteParseResult result;
//   while ((result = syntaqlite_parser_next(p)).root != SYNTAQLITE_NULL_NODE) {
//     // walk the AST via syntaqlite_parser_node(p, result.root)
//   }
//   if (result.error) { /* handle error */ }
//   syntaqlite_parser_destroy(p);

#ifndef SYNTAQLITE_PARSER_H
#define SYNTAQLITE_PARSER_H

#include <stdint.h>

#include "syntaqlite/ast_nodes.h"
#include "syntaqlite/config.h"

#ifdef __cplusplus
extern "C" {
#endif

// Opaque parser handle (heap-allocated).
typedef struct SyntaqliteParser SyntaqliteParser;

// Result from syntaqlite_parser_next(). Check root first: if it is
// SYNTAQLITE_NULL_NODE, parsing is done — then check error to see if it
// ended cleanly or with a parse error.
typedef struct SyntaqliteParseResult {
  uint32_t root;  // Root node ID, or SYNTAQLITE_NULL_NODE at end-of-input.
  int error;      // Nonzero if a parse error occurred.
  const char* error_msg;  // Error message (owned by parser), or NULL.
} SyntaqliteParseResult;

// --- Lifecycle ---

// 1. Allocate a parser. The parser is inert until reset() is called.
//    The config is copied — the caller's SyntaqliteConfig does not need to
//    outlive the parser. Pass NULL for all defaults.
SyntaqliteParser* syntaqlite_parser_create(const SyntaqliteConfig* config);

// 2. Bind a source buffer and reset all internal state. The source must
//    remain valid until the next reset() or destroy(). Can be called again
//    to parse a new input without reallocating — all previous nodes are
//    invalidated when this is called.
void syntaqlite_parser_reset(SyntaqliteParser* p,
                             const char* source,
                             uint32_t len);

// 3. Parse the next statement. Call in a loop until root is
//    SYNTAQLITE_NULL_NODE. Bare semicolons between statements are skipped
//    automatically. Each call appends nodes to the arena; all nodes from
//    all statements remain valid until the next reset() or destroy().
SyntaqliteParseResult syntaqlite_parser_next(SyntaqliteParser* p);

// --- Reading results (between next() calls, or after the loop) ---

// Look up a node by ID. The returned pointer is valid until the next
// reset() or destroy(). Use the node's type tag to determine which union
// member to read (see ast_nodes_gen.h).
const SyntaqliteNode* syntaqlite_parser_node(SyntaqliteParser* p,
                                             uint32_t node_id);

// Access the source text bound by the last reset() call. Useful for
// extracting token text via SyntaqliteSourceSpan offsets.
const char* syntaqlite_parser_source(SyntaqliteParser* p);

// Length of the source text bound by the last reset() call.
uint32_t syntaqlite_parser_source_length(SyntaqliteParser* p);

// --- Cleanup ---

// 4. Free the parser and all its nodes. No-op if p is NULL.
void syntaqlite_parser_destroy(SyntaqliteParser* p);

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_PARSER_H
