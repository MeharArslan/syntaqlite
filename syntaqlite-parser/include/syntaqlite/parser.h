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
#include "syntaqlite/dialect.h"
#include "syntaqlite/types.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// Opaque parser handle (heap-allocated, reusable across inputs).
typedef struct SyntaqliteParser SyntaqliteParser;

// Opaque dialect handle — produced by dialect crates (e.g.
// syntaqlite_sqlite_dialect()).
typedef struct SyntaqliteDialect SyntaqliteDialect;

// A comment captured during parsing.
typedef struct SyntaqliteComment {
  uint32_t offset;  // Byte offset in source.
  uint32_t length;  // Byte length.
  uint8_t kind;     // 0 = line comment (--), 1 = block comment (/* */).
} SyntaqliteComment;

// Token flags bitfield.
#define SYNQ_TOKEN_FLAG_AS_ID \
  1  // Token was consumed as identifier (fallback from keyword).
#define SYNQ_TOKEN_FLAG_AS_FUNCTION 2  // Token was consumed as function name.
#define SYNQ_TOKEN_FLAG_AS_TYPE 4      // Token was consumed as a type name.

// A non-whitespace, non-comment token position captured during parsing.
typedef struct SyntaqliteTokenPos {
  uint32_t offset;  // Byte offset in source.
  uint32_t length;  // Byte length.
  uint32_t type;    // Original token type from tokenizer (pre-fallback).
  uint32_t flags;   // Bitfield: SYNQ_TOKEN_FLAG_AS_ID / AS_FUNCTION / AS_TYPE.
} SyntaqliteTokenPos;

// Result of parsing one statement via syntaqlite_parser_next().
//
// Check root first: if it is SYNTAQLITE_NULL_NODE, parsing is done — then
// check error to see whether it ended cleanly or with a parse error.
typedef struct SyntaqliteParseResult {
  uint32_t root;          // Root node ID, or SYNTAQLITE_NULL_NODE.
  int32_t error;          // Nonzero if a parse error occurred.
  const char* error_msg;  // Human-readable message (owned by parser), or NULL.
  uint32_t
      error_offset;  // Byte offset of the error token (0xFFFFFFFF = unknown).
  uint32_t error_length;  // Byte length of the error token (0 = unknown).
  int32_t saw_subquery;   // Nonzero if the statement contains a subquery.
  int32_t saw_update_delete_limit;  // Nonzero if DELETE/UPDATE uses ORDER BY or
                                    // LIMIT.
} SyntaqliteParseResult;

// A recorded macro invocation region, populated via the low-level API
// (begin_macro / end_macro). The formatter uses these to reconstruct macro
// calls from the expanded AST.
typedef struct SyntaqliteMacroRegion {
  uint32_t call_offset;  // Byte offset of macro call in original source.
  uint32_t call_length;  // Byte length of entire macro call.
} SyntaqliteMacroRegion;

// Error node tag — stored as the first uint32_t of a SyntaqliteErrorNode.
// Tag 0 is the sentinel for error nodes; it is never used as a real node tag
// in generated code (NodeTag::Null = 0 is a codegen sentinel, not stored in
// the arena under normal operation).
#define SYNTAQLITE_ERROR_NODE_TAG 0u

// An error placeholder node stored in the arena when a parse error occurs.
// Written by grammar actions via synq_parse_error_node() and recognised by
// NodeReader::required_node / optional_node before dispatching on the tag.
typedef struct SyntaqliteErrorNode {
  uint32_t tag;     // Always SYNTAQLITE_ERROR_NODE_TAG (0).
  uint32_t offset;  // Byte offset of the error in source.
  uint32_t length;  // Byte length of the error token (0 = unknown).
} SyntaqliteErrorNode;

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

// Allocate a parser for a specific dialect. The parser is inert until reset()
// is called. The mem methods are copied — the caller's struct does not need
// to outlive the parser. Pass NULL for mem defaults (malloc/free). The dialect
// pointer must remain valid for the lifetime of the parser.
SyntaqliteParser* syntaqlite_create_parser_with_dialect(
    const SyntaqliteMemMethods* mem,
    const SyntaqliteDialect* dialect);

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
const void* syntaqlite_parser_node(SyntaqliteParser* p, uint32_t node_id);

// Return the number of nodes currently in the arena. Flushes any pending
// list nodes first, so the returned count and all node data are consistent.
uint32_t syntaqlite_parser_node_count(SyntaqliteParser* p);

// Return a pointer to the source text bound by the last reset() call.
// Useful for extracting token text via SyntaqliteSourceSpan offsets:
//   syntaqlite_parser_source(p) + span.offset
const char* syntaqlite_parser_source(SyntaqliteParser* p);

// Return the byte length of the source text bound by the last reset() call.
uint32_t syntaqlite_parser_source_length(SyntaqliteParser* p);

// Return the comments captured during parsing. The returned pointer
// is valid until the next reset() or destroy(). Requires collect_tokens to be
// enabled. Sets *count to the number of comments.
const SyntaqliteComment* syntaqlite_parser_comments(SyntaqliteParser* p,
                                                    uint32_t* count);

// Return the non-whitespace, non-comment token positions captured during
// parsing. Requires collect_tokens to be enabled. Sets *count to the number
// of tokens. The returned pointer is valid until the next reset() or destroy().
const SyntaqliteTokenPos* syntaqlite_parser_tokens(SyntaqliteParser* p,
                                                   uint32_t* count);

// Return the macro regions recorded via begin_macro/end_macro. The returned
// pointer is valid until the next reset() or destroy(). Sets *count to the
// number of regions.
const SyntaqliteMacroRegion* syntaqlite_parser_macro_regions(
    SyntaqliteParser* p,
    uint32_t* count);

// ---------------------------------------------------------------------------
// Source span helpers
// ---------------------------------------------------------------------------

// Extract text for a source span. Returns a pointer into the parser's source
// buffer (NOT null-terminated). Sets *out_len to the byte length. Returns
// NULL if the span is empty (length == 0).
//
// Usage:
//   SyntaqliteColumnRef* col = ...;
//   uint32_t len;
//   const char* name = syntaqlite_span_text(p, col->column, &len);
//   if (name) printf("column: %.*s\n", (int)len, name);
static inline const char* syntaqlite_span_text(SyntaqliteParser* p,
                                               SyntaqliteSourceSpan span,
                                               uint32_t* out_len) {
  if (span.length == 0) {
    *out_len = 0;
    return NULL;
  }
  *out_len = span.length;
  return syntaqlite_parser_source(p) + span.offset;
}

// Test whether a source span is present (non-empty).
static inline int syntaqlite_span_is_present(SyntaqliteSourceSpan span) {
  return span.length != 0;
}

// ---------------------------------------------------------------------------
// List node helpers
// ---------------------------------------------------------------------------

// Return the number of children in a list node. The node pointer must point
// to a list node (one whose struct has tag + count + children[] layout).
static inline uint32_t syntaqlite_list_count(const void* list_node) {
  const uint32_t* raw = (const uint32_t*)list_node;
  return raw[1];  // count is the second uint32_t after tag
}

// Return the node ID of the i-th child in a list node. Does NOT bounds-check.
static inline uint32_t syntaqlite_list_child_id(const void* list_node,
                                                uint32_t index) {
  const uint32_t* raw = (const uint32_t*)list_node;
  return raw[2 + index];  // children start at offset 8 (after tag + count)
}

// Return a pointer to the i-th child node in a list. Combines list_child_id
// with syntaqlite_parser_node for convenience. Returns NULL if the child ID
// is SYNTAQLITE_NULL_NODE.
static inline const void* syntaqlite_list_child(SyntaqliteParser* p,
                                                const void* list_node,
                                                uint32_t index) {
  uint32_t child_id = syntaqlite_list_child_id(list_node, index);
  if (child_id == SYNTAQLITE_NULL_NODE)
    return NULL;
  return syntaqlite_parser_node(p, child_id);
}

// ---------------------------------------------------------------------------
// Optional node field helpers
// ---------------------------------------------------------------------------

// Test whether a node ID field is present (not SYNTAQLITE_NULL_NODE).
static inline int syntaqlite_node_is_present(uint32_t node_id) {
  return node_id != SYNTAQLITE_NULL_NODE;
}

// ---------------------------------------------------------------------------
// Typed access macros
// ---------------------------------------------------------------------------

// Look up a node by ID and cast to a concrete type. Returns NULL if the
// node ID is SYNTAQLITE_NULL_NODE.
//
//   const SyntaqliteStmt* stmt = SYNTAQLITE_NODE(p, SyntaqliteStmt, root_id);
#define SYNTAQLITE_NODE(p, Type, id) \
  ((id) == SYNTAQLITE_NULL_NODE      \
       ? (const Type*)0              \
       : (const Type*)syntaqlite_parser_node((p), (id)))

// Cast the i-th child of a list node to a concrete type.
//
//   const SyntaqliteColumnDef* cd =
//       SYNTAQLITE_LIST_ITEM(p, SyntaqliteColumnDef, list, i);
#define SYNTAQLITE_LIST_ITEM(p, Type, list, i) \
  ((const Type*)syntaqlite_list_child((p), (list), (i)))

// Iterate over every child in a list node, binding each as `const Type* var`.
// Handles null list IDs (zero iterations). The variable is scoped to the loop.
//
//   SYNTAQLITE_LIST_FOREACH(p, SyntaqliteColumnDef, col, ct->columns) {
//     printf("%.*s\n", col->column_name.length,
//            syntaqlite_parser_source(p) + col->column_name.offset);
//   }
#define SYNTAQLITE_LIST_FOREACH(p, Type, var, list_id)                    \
  for (const void *                                                       \
           _sqlist_##var = syntaqlite_node_is_present(list_id)            \
                               ? syntaqlite_parser_node((p), (list_id))   \
                               : 0,                                       \
          *_sqonce_##var = 0;                                             \
       !_sqonce_##var; _sqonce_##var = (const void*)1)                    \
    for (uint32_t _sqi_##var = 0,                                         \
                  _sqn_##var = _sqlist_##var                              \
                                   ? syntaqlite_list_count(_sqlist_##var) \
                                   : 0;                                   \
         _sqi_##var < _sqn_##var; _sqi_##var++)                           \
      for (const Type* var =                                              \
               SYNTAQLITE_LIST_ITEM(p, Type, _sqlist_##var, _sqi_##var);  \
           var; var = 0)

// ---------------------------------------------------------------------------
// AST dump
// ---------------------------------------------------------------------------

// Dump an AST node tree as indented text. Returns a malloc'd NUL-terminated
// string. The caller must free() the result. Returns NULL on allocation
// failure.
char* syntaqlite_dump_node(SyntaqliteParser* p,
                           uint32_t node_id,
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

// Set the dialect config for version/cflag-gated tokenization.
// The config is copied — the caller's struct does not need to outlive the
// parser. Default: latest version (INT32_MAX), no cflags.
// Returns 0 on success, -1 if the parser has already been used.
int syntaqlite_parser_set_dialect_config(SyntaqliteParser* p,
                                         const SyntaqliteDialectConfig* config);

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
// as a comment (when collect_tokens is enabled) but not fed to the parser.
// Returns: 0 = keep going, 1 = statement completed, -1 = error.
int syntaqlite_parser_feed_token(SyntaqliteParser* p,
                                 int token_type,
                                 const char* text,
                                 int len);

// Retrieve the parse result after feed_token returns 1 or after finish().
SyntaqliteParseResult syntaqlite_parser_result(SyntaqliteParser* p);

// Enumerate terminal tokens that are valid next lookaheads at the parser's
// current state. Returns the total number of expected tokens.
//
// If out_tokens is non-NULL, up to out_cap token IDs are written.
// This API is intended for grammar-aware completion engines.
int syntaqlite_parser_expected_tokens(SyntaqliteParser* p,
                                      int* out_tokens,
                                      int out_cap);

// Return the semantic completion context at the parser's current state.
// 0 = Unknown, 1 = Expression, 2 = TableRef.
uint32_t syntaqlite_parser_completion_context(SyntaqliteParser* p);

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

// ---------------------------------------------------------------------------
// SQLite dialect convenience (opt-out: -DSYNTAQLITE_OMIT_SQLITE_API)
// ---------------------------------------------------------------------------

#ifndef SYNTAQLITE_OMIT_SQLITE_API
const SyntaqliteDialect* syntaqlite_sqlite_dialect(void);
static inline SyntaqliteParser* syntaqlite_create_sqlite_parser(
    const SyntaqliteMemMethods* mem) {
  return syntaqlite_create_parser_with_dialect(mem,
                                               syntaqlite_sqlite_dialect());
}
#endif

#ifdef __cplusplus
}
#endif

// ---------------------------------------------------------------------------
// C++ convenience wrappers (requires C++17)
// ---------------------------------------------------------------------------

#if defined(__cplusplus) && __cplusplus >= 201703L
#include <string_view>

namespace syntaqlite {

// Extracts a string_view from a source span.  Returns empty if absent.
inline std::string_view SpanText(SyntaqliteParser* p,
                                 SyntaqliteSourceSpan span) {
  if (span.length == 0)
    return {};
  return {syntaqlite_parser_source(p) + span.offset, span.length};
}

// Returns true if a source span is present (non-empty).
inline bool IsPresent(SyntaqliteSourceSpan span) {
  return span.length != 0;
}

// Returns true if a node ID is present (not null).
inline bool IsPresent(uint32_t node_id) {
  return node_id != SYNTAQLITE_NULL_NODE;
}

// Maps a concrete node type to its tag constant. Specialize in dialect
// headers (e.g. sqlite_node.h) to enable tag-checked NodeCast.
template <typename T>
struct NodeTag {
  // No default value — unspecialized types get an unchecked cast.
  static constexpr bool kHasTag = false;
  static constexpr uint32_t kValue = 0;
};

// Returns a typed pointer to a node.  Returns nullptr if `node_id` is null.
// When a NodeTag<T> specialization exists, also checks the tag and returns
// nullptr on mismatch.
template <typename T>
const T* NodeCast(SyntaqliteParser* p, uint32_t node_id) {
  if (node_id == SYNTAQLITE_NULL_NODE)
    return nullptr;
  const T* node = static_cast<const T*>(syntaqlite_parser_node(p, node_id));
  if constexpr (NodeTag<T>::kHasTag) {
    if (node->tag != NodeTag<T>::kValue)
      return nullptr;
  }
  return node;
}

// Iterable view over a list node's children with typed element access.
//
// Usage:
//   for (const auto* col :
//        syntaqlite::MakeListView<SyntaqliteColumnDef>(p, table.columns)) {
//     printf("%.*s\n", col->column_name.length,
//            syntaqlite_parser_source(p) + col->column_name.offset);
//   }
template <typename T>
class ListView {
 public:
  ListView(SyntaqliteParser* parser, const void* list)
      : parser_(parser), list_(list) {}

  uint32_t size() const { return list_ ? syntaqlite_list_count(list_) : 0; }

  const T* operator[](uint32_t i) const {
    return static_cast<const T*>(syntaqlite_list_child(parser_, list_, i));
  }

  class Iterator {
   public:
    Iterator(SyntaqliteParser* parser, const void* list, uint32_t index)
        : parser_(parser), list_(list), index_(index) {}

    const T* operator*() const {
      return static_cast<const T*>(
          syntaqlite_list_child(parser_, list_, index_));
    }
    Iterator& operator++() {
      ++index_;
      return *this;
    }
    bool operator!=(const Iterator& other) const {
      return index_ != other.index_;
    }

   private:
    SyntaqliteParser* parser_;
    const void* list_;
    uint32_t index_;
  };

  Iterator begin() const { return {parser_, list_, 0}; }
  Iterator end() const { return {parser_, list_, size()}; }

 private:
  SyntaqliteParser* parser_;
  const void* list_;
};

// Creates a ListView from a list node ID.  Returns an empty view if null.
template <typename T>
ListView<T> MakeListView(SyntaqliteParser* p, uint32_t list_id) {
  if (list_id == SYNTAQLITE_NULL_NODE)
    return {p, nullptr};
  return {p, syntaqlite_parser_node(p, list_id)};
}

// RAII wrapper for SyntaqliteParser.  Non-copyable, movable.
//
// Usage:
//   auto parser = syntaqlite::SqliteParser();  // see below
//   parser.Reset("SELECT 1; SELECT 2;");
//   SyntaqliteParseResult result;
//   while (result = parser.Next(), syntaqlite::IsPresent(result.root)) {
//     const auto* stmt = parser.Node<SyntaqliteStmt>(result.root);
//     ...
//   }
class Parser {
 public:
  // Takes ownership of a raw parser handle.
  explicit Parser(SyntaqliteParser* raw) : raw_(raw) {}
  ~Parser() { syntaqlite_parser_destroy(raw_); }

  Parser(const Parser&) = delete;
  Parser& operator=(const Parser&) = delete;
  Parser(Parser&& other) noexcept : raw_(other.raw_) { other.raw_ = nullptr; }
  Parser& operator=(Parser&& other) noexcept {
    if (this != &other) {
      syntaqlite_parser_destroy(raw_);
      raw_ = other.raw_;
      other.raw_ = nullptr;
    }
    return *this;
  }

  // Returns the underlying C handle.
  SyntaqliteParser* raw() const { return raw_; }

  // Binds source text and resets all parser state.
  void Reset(const char* sql, uint32_t len) {
    syntaqlite_parser_reset(raw_, sql, len);
  }
  void Reset(std::string_view sql) {
    syntaqlite_parser_reset(raw_, sql.data(),
                            static_cast<uint32_t>(sql.size()));
  }

  // Parses the next statement.
  SyntaqliteParseResult Next() { return syntaqlite_parser_next(raw_); }

  // Returns a typed pointer to a node by ID.
  template <typename T>
  const T* Node(uint32_t node_id) const {
    return NodeCast<T>(raw_, node_id);
  }

  // Returns an iterable view over a list node's children.
  template <typename T>
  ListView<T> List(uint32_t list_id) const {
    return MakeListView<T>(raw_, list_id);
  }

  // Extracts text from a source span.
  std::string_view Text(SyntaqliteSourceSpan span) const {
    return SpanText(raw_, span);
  }

  // Dumps an AST node as indented text.  Caller must free() the result.
  char* DumpNode(uint32_t node_id, uint32_t indent = 0) const {
    return syntaqlite_dump_node(raw_, node_id, indent);
  }

 private:
  SyntaqliteParser* raw_;
};

// SQLite dialect convenience (opt-out: -DSYNTAQLITE_OMIT_SQLITE_API).
#ifndef SYNTAQLITE_OMIT_SQLITE_API
inline Parser SqliteParser() {
  return Parser(syntaqlite_create_sqlite_parser(nullptr));
}
#endif

}  // namespace syntaqlite
#endif

#endif  // SYNTAQLITE_PARSER_H
