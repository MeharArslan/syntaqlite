// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Streaming parser for SQLite SQL — the main entry point for AST access.
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
//     const SyntaqliteNode* node = syntaqlite_parser_node(p, r.root);
//     // switch on node->type ...
//   }
//   if (r.error) { /* handle r.error_msg */ }
//   syntaqlite_parser_destroy(p);
//
// With token collection (required for formatting):
//   SyntaqliteParser* p = syntaqlite_parser_create(NULL);
//   syntaqlite_parser_set_collect_tokens(p, 1);
//   // ... parse as above, then pass to formatter ...
//
// With a dialect extension:
//   const SyntaqliteDialectExtension* ext =
//       syntaqlite_load_extension("libsql_dialect.so", NULL);
//   SyntaqliteParser* p = syntaqlite_parser_create(NULL);
//   syntaqlite_parser_set_extension(p, ext);
//   // ... parse as above ...

#ifndef SYNTAQLITE_PARSER_H
#define SYNTAQLITE_PARSER_H

#include <stdint.h>
#include <stdio.h>

#include "syntaqlite/node.h"
#include "syntaqlite/config.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// Opaque parser handle (heap-allocated, reusable across inputs).
typedef struct SyntaqliteParser SyntaqliteParser;

// Result of parsing one statement via syntaqlite_parser_next().
//
// Check root first: if it is SYNTAQLITE_NULL_NODE, parsing is done — then
// check error to see whether it ended cleanly or with a parse error.
typedef struct SyntaqliteParseResult {
  uint32_t root;          // Root node ID, or SYNTAQLITE_NULL_NODE.
  int error;              // Nonzero if a parse error occurred.
  const char* error_msg;  // Human-readable message (owned by parser), or NULL.
} SyntaqliteParseResult;

// Opaque handle to a dialect extension (extra keywords, grammar rules, AST
// node types) produced by the syntaqlite codegen tooling. Extensions are
// additive — the base SQLite grammar is always included.
typedef struct SyntaqliteDialectExtension SyntaqliteDialectExtension;

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

// Allocate a parser. The parser is inert until reset() is called. The mem
// methods are copied — the caller's struct does not need to outlive the
// parser. Pass NULL for all defaults (malloc/free).
SyntaqliteParser* syntaqlite_parser_create(const SyntaqliteMemMethods* mem);

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
// next reset() or destroy(). Use node->type to determine which union member
// to read (see node.h for the full SyntaqliteNode definition).
const SyntaqliteNode* syntaqlite_parser_node(SyntaqliteParser* p,
                                             uint32_t node_id);

// Return a pointer to the source text bound by the last reset() call.
// Useful for extracting token text via SyntaqliteSourceSpan offsets:
//   syntaqlite_parser_source(p) + span.offset
const char* syntaqlite_parser_source(SyntaqliteParser* p);

// Return the byte length of the source text bound by the last reset() call.
uint32_t syntaqlite_parser_source_length(SyntaqliteParser* p);

// ---------------------------------------------------------------------------
// Configuration (call after create, before first reset)
// ---------------------------------------------------------------------------

// Enable parser trace output (debug builds only). When enabled, the parser
// prints shift/reduce actions to stderr. Useful for diagnosing grammar
// conflicts or unexpected parses. Default: off (0).
void syntaqlite_parser_set_trace(SyntaqliteParser* p, int enable);

// Enable token collection. When enabled, the parser records every token
// (including whitespace and comments) so the formatter can reproduce the
// original layout. Required before passing the parser to the formatter.
// Default: off (0).
void syntaqlite_parser_set_collect_tokens(SyntaqliteParser* p, int enable);

// Set a dialect extension on this parser. The extension pointer must remain
// valid for the lifetime of the parser. Pass NULL to remove a previously
// set extension (reverts to pure SQLite grammar).
void syntaqlite_parser_set_extension(SyntaqliteParser* p,
                                     const SyntaqliteDialectExtension* ext);

// ---------------------------------------------------------------------------
// Dialect extensions
// ---------------------------------------------------------------------------

// Load a dialect extension from a shared library at runtime (dlopen).
//
// lib_path:    Path to the .so / .dylib / .dll containing the extension.
// entry_point: Name of the C entry-point symbol that returns the extension
//              tables. Pass NULL to use the default convention: the
//              library's basename with "_dialect_extension" appended
//              (e.g. "libsql_dialect.so" → "libsql_dialect_extension").
//
// Returns a pointer to the loaded extension, or NULL on failure. The
// returned pointer is valid until the library is unloaded.
const SyntaqliteDialectExtension* syntaqlite_load_extension(
    const char* lib_path,
    const char* entry_point);

// ---------------------------------------------------------------------------
// Debug / inspection
// ---------------------------------------------------------------------------

// Print an AST subtree rooted at node_id to a file stream (e.g. stderr).
// Needs the parser to resolve child node IDs and access source text.
void syntaqlite_ast_print(SyntaqliteParser* p, uint32_t node_id, FILE* out);

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_PARSER_H
