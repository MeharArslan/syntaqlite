// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Streaming parser for SQL — the main entry point for AST access.
//
// Produces a typed AST from SQL text. Each call to syntaqlite_parser_next()
// parses one statement and returns a SYNTAQLITE_PARSE_* status code. Result
// details are accessed via the syntaqlite_result_*() accessors, which are
// valid until the next syntaqlite_parser_next(), reset(), or destroy() call.
// The arena is reset between statements, so only O(statement) memory is used.
//
// Lifecycle: create → [configure] → reset → next (loop) → read nodes → destroy.
// A single parser can be reused across inputs by calling reset() again.
//
// Usage:
//   SyntaqliteParser* p = syntaqlite_parser_create(NULL);
//   syntaqlite_parser_reset(p, sql, len);
//   for (;;) {
//     int32_t rc = syntaqlite_parser_next(p);
//     switch (rc) {
//       case SYNTAQLITE_PARSE_DONE:
//         goto done;
//       case SYNTAQLITE_PARSE_OK: {
//         uint32_t root = syntaqlite_result_root(p);
//         const void* node = syntaqlite_parser_node(p, root);
//         // cast to dialect-specific node type and switch on tag ...
//         break;
//       }
//       case SYNTAQLITE_PARSE_ERROR: {
//         fprintf(stderr, "%s\n", syntaqlite_result_error_msg(p));
//         uint32_t recovery_root = syntaqlite_result_recovery_root(p);
//         if (recovery_root == SYNTAQLITE_NULL_NODE)
//           goto done;
//         break;
//       }
//     }
//   }
// done:
//   syntaqlite_parser_destroy(p);
//
// Token/comment capture is OFF by default. If you need
// syntaqlite_result_tokens() / syntaqlite_result_comments() (for formatting,
// diagnostics, etc.), call syntaqlite_parser_set_collect_tokens(p, 1) before
// the first reset().
// For custom dialects, see the "Advanced" section below.
// For macro-aware or incremental token feeding, see incremental.h.

#ifndef SYNTAQLITE_PARSER_H
#define SYNTAQLITE_PARSER_H

#include <stdint.h>
#include <stdio.h>

#include "syntaqlite/config.h"
#include "syntaqlite/grammar.h"
#include "syntaqlite/types.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Types
// ---------------------------------------------------------------------------

// Opaque parser handle (heap-allocated, reusable across inputs).
typedef struct SyntaqliteParser SyntaqliteParser;

// Return codes from syntaqlite_parser_next() and syntaqlite_parser_finish().
//
//   DONE  = no statement (all input consumed, or only bare semicolons)
//   OK    = statement parsed cleanly; nodes are valid
//   ERROR = statement has syntax/runtime error
//           - use syntaqlite_result_recovery_root() to check whether a
//             partial recovery tree is available
//           - syntaqlite_result_root() is always SYNTAQLITE_NULL_NODE on ERROR
//
// The integer values are stable ABI (DONE=0, OK=1, ERROR=-1).
#define SYNTAQLITE_PARSE_DONE 0
#define SYNTAQLITE_PARSE_OK 1
#define SYNTAQLITE_PARSE_ERROR (-1)

// A comment captured during parsing.
typedef struct SyntaqliteComment {
  uint32_t offset;  // Byte offset in source.
  uint32_t length;  // Byte length.
  uint8_t kind;     // 0 = line comment (--), 1 = block comment (/* */).
} SyntaqliteComment;

// Token-usage flags: set by the parser during disambiguation to record how
// each token was consumed.  Use SYNQ_TOKEN_FLAG_* as bitmasks on the flags
// field of SyntaqliteParserToken.
typedef uint32_t SyntaqliteParserTokenFlags;
#define SYNQ_TOKEN_FLAG_AS_ID \
  ((SyntaqliteParserTokenFlags)1)  // Consumed as identifier (keyword fallback).
#define SYNQ_TOKEN_FLAG_AS_FUNCTION \
  ((SyntaqliteParserTokenFlags)2)  // Consumed as function name.
#define SYNQ_TOKEN_FLAG_AS_TYPE \
  ((SyntaqliteParserTokenFlags)4)  // Consumed as type name.

// A non-whitespace, non-comment token position captured during parsing.
typedef struct SyntaqliteParserToken {
  uint32_t offset;  // Byte offset in source.
  uint32_t length;  // Byte length.
  uint32_t type;    // Original token type from tokenizer (pre-fallback).
  SyntaqliteParserTokenFlags flags;  // Bitmask of SYNQ_TOKEN_FLAG_* values.
} SyntaqliteParserToken;

// A recorded macro invocation region.
// For the input-side begin/end API see incremental.h.
typedef struct SyntaqliteMacroRegion {
  uint32_t call_offset;  // Byte offset of macro call in original source.
  uint32_t call_length;  // Byte length of entire macro call.
} SyntaqliteMacroRegion;

// ---------------------------------------------------------------------------
// Core API
// ---------------------------------------------------------------------------

// Allocate a parser bound to a specific grammar environment.
SyntaqliteParser* syntaqlite_parser_create_with_grammar(
    const SyntaqliteMemMethods* mem,
    SyntaqliteGrammar env);

// Bind a source buffer and reset all internal state. The source must remain
// valid until the next reset() or destroy(). Can be called again to parse a
// new input without reallocating — all previous nodes are invalidated.
void syntaqlite_parser_reset(SyntaqliteParser* p,
                             const char* source,
                             uint32_t len);

// Parse the next SQL statement. Call in a loop until SYNTAQLITE_PARSE_DONE.
// Bare semicolons between statements are skipped automatically.
// The arena is reset at the start of each call — pointers from the previous
// call become invalid.
//
// Returns one of the SYNTAQLITE_PARSE_* codes.
int32_t syntaqlite_parser_next(SyntaqliteParser* p);

// Free the parser, its arena, and all its nodes. No-op if p is NULL.
void syntaqlite_parser_destroy(SyntaqliteParser* p);

// ---------------------------------------------------------------------------
// Result accessors
// Valid until the next syntaqlite_parser_next(), reset(), or destroy() call.
// ---------------------------------------------------------------------------

// Statement root node ID for SYNTAQLITE_PARSE_OK results.
// Returns SYNTAQLITE_NULL_NODE for DONE/ERROR.
uint32_t syntaqlite_result_root(SyntaqliteParser* p);

// Partial recovery root for SYNTAQLITE_PARSE_ERROR results.
// Returns SYNTAQLITE_NULL_NODE when no recovery tree is available.
// Recovery trees may include grammar-level error nodes where parsing resumed.
uint32_t syntaqlite_result_recovery_root(SyntaqliteParser* p);

// Human-readable error message, or NULL.
const char* syntaqlite_result_error_msg(SyntaqliteParser* p);

// Byte offset of error token (0xFFFFFFFF = unknown).
uint32_t syntaqlite_result_error_offset(SyntaqliteParser* p);

// Byte length of error token (0 = unknown).
uint32_t syntaqlite_result_error_length(SyntaqliteParser* p);

// Per-statement token/comment/macro arrays.
// Token/comment arrays are empty unless collect_tokens is enabled via
// syntaqlite_parser_set_collect_tokens(p, 1) before first reset().
const SyntaqliteComment* syntaqlite_result_comments(SyntaqliteParser* p,
                                                    uint32_t* count);
const SyntaqliteParserToken* syntaqlite_result_tokens(SyntaqliteParser* p,
                                                      uint32_t* count);
const SyntaqliteMacroRegion* syntaqlite_result_macros(SyntaqliteParser* p,
                                                      uint32_t* count);

// ---------------------------------------------------------------------------
// Arena accessors
// ---------------------------------------------------------------------------

// Look up a node by its arena ID. The returned pointer is valid until the
// next syntaqlite_parser_next(), reset(), or destroy(). Cast to the
// dialect-specific node union type and use the tag field to determine which
// member to read.
const void* syntaqlite_parser_node(SyntaqliteParser* p, uint32_t node_id);

// Return a pointer to the source text bound by the last reset() call.
const char* syntaqlite_parser_source(SyntaqliteParser* p);

// Return the byte length of the source text bound by the last reset() call.
uint32_t syntaqlite_parser_source_length(SyntaqliteParser* p);

// Return the number of nodes currently in the arena.
uint32_t syntaqlite_parser_node_count(SyntaqliteParser* p);

// ---------------------------------------------------------------------------
// Source span helpers
// ---------------------------------------------------------------------------

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

static inline uint32_t syntaqlite_span_is_present(SyntaqliteSourceSpan span) {
  return span.length != 0;
}

// ---------------------------------------------------------------------------
// Node and list helpers
// ---------------------------------------------------------------------------

static inline uint32_t syntaqlite_node_is_present(uint32_t node_id) {
  return node_id != SYNTAQLITE_NULL_NODE;
}

static inline uint32_t syntaqlite_list_count(const void* list_node) {
  const uint32_t* raw = (const uint32_t*)list_node;
  return raw[1];
}

static inline uint32_t syntaqlite_list_child_id(const void* list_node,
                                                uint32_t index) {
  const uint32_t* raw = (const uint32_t*)list_node;
  return raw[2 + index];
}

static inline const void* syntaqlite_list_child(SyntaqliteParser* p,
                                                const void* list_node,
                                                uint32_t index) {
  uint32_t child_id = syntaqlite_list_child_id(list_node, index);
  if (child_id == SYNTAQLITE_NULL_NODE)
    return NULL;
  return syntaqlite_parser_node(p, child_id);
}

// ---------------------------------------------------------------------------
// Typed access macros
// ---------------------------------------------------------------------------

#define SYNTAQLITE_NODE(p, Type, id) \
  ((id) == SYNTAQLITE_NULL_NODE      \
       ? (const Type*)0              \
       : (const Type*)syntaqlite_parser_node((p), (id)))

#define SYNTAQLITE_LIST_ITEM(p, Type, list, i) \
  ((const Type*)syntaqlite_list_child((p), (list), (i)))

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

// ============================================================================
// Configuration — call after create(), before first reset()
// ============================================================================

// Enable token/comment collection for result_tokens/result_comments.
// Default: off (0), in which case those arrays are empty.
// Returns 0 on success, -1 if the parser has already been used.
int32_t syntaqlite_parser_set_collect_tokens(SyntaqliteParser* p,
                                             uint32_t enable);

// Enable parser trace output (debug builds only). Default: off (0).
// Returns 0 on success, -1 if the parser has already been used.
int32_t syntaqlite_parser_set_trace(SyntaqliteParser* p, uint32_t enable);

// ============================================================================
// Debugging
// ============================================================================

// Dump an AST node tree as indented text. Returns a malloc'd NUL-terminated
// string. The caller must free() the result. Returns NULL on allocation
// failure.
char* syntaqlite_dump_node(SyntaqliteParser* p,
                           uint32_t node_id,
                           uint32_t indent);

// ============================================================================
// Advanced: custom dialects
// ============================================================================

#ifndef SYNTAQLITE_OMIT_SQLITE_API
// Allocate a parser for the built-in SQLite grammar. The parser is inert
// until reset() is called. Pass NULL for mem to use malloc/free.
SyntaqliteParser* syntaqlite_parser_create(const SyntaqliteMemMethods* mem);

SyntaqliteGrammar syntaqlite_sqlite_grammar(void);
#endif

#ifdef __cplusplus
}
#endif

// ============================================================================
// C++ convenience wrappers (requires C++17)
// ============================================================================

#if defined(__cplusplus) && __cplusplus >= 201703L
#include <string_view>

namespace syntaqlite {

inline std::string_view SpanText(SyntaqliteParser* p,
                                 SyntaqliteSourceSpan span) {
  if (span.length == 0)
    return {};
  return {syntaqlite_parser_source(p) + span.offset, span.length};
}

inline bool IsPresent(SyntaqliteSourceSpan span) {
  return span.length != 0;
}
inline bool IsPresent(uint32_t node_id) {
  return node_id != SYNTAQLITE_NULL_NODE;
}

template <typename T>
struct NodeTag {
  static constexpr bool kHasTag = false;
  static constexpr uint32_t kValue = 0;
};

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

template <typename T>
ListView<T> MakeListView(SyntaqliteParser* p, uint32_t list_id) {
  if (list_id == SYNTAQLITE_NULL_NODE)
    return {p, nullptr};
  return {p, syntaqlite_parser_node(p, list_id)};
}

// RAII wrapper for SyntaqliteParser.  Non-copyable, movable.
//
// Usage:
//   auto parser = syntaqlite::SqliteParser();
//   parser.Reset("SELECT 1; SELECT 2;");
//   for (;;) {
//     int rc = parser.Next();
//     switch (rc) {
//       case SYNTAQLITE_PARSE_DONE:
//         return;
//       case SYNTAQLITE_PARSE_OK:
//         (void)parser.Node<SyntaqliteStmt>(parser.ResultRoot());
//         break;
//       case SYNTAQLITE_PARSE_ERROR:
//         if (parser.ResultRecoveryRoot() == SYNTAQLITE_NULL_NODE)
//           return;
//         break;
//     }
//   }
class Parser {
 public:
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

  SyntaqliteParser* raw() const { return raw_; }

  void Reset(const char* sql, uint32_t len) {
    syntaqlite_parser_reset(raw_, sql, len);
  }
  void Reset(std::string_view sql) {
    syntaqlite_parser_reset(raw_, sql.data(),
                            static_cast<uint32_t>(sql.size()));
  }

  // Returns a SYNTAQLITE_PARSE_* code.
  int32_t Next() { return syntaqlite_parser_next(raw_); }

  uint32_t ResultRoot() const { return syntaqlite_result_root(raw_); }
  uint32_t ResultRecoveryRoot() const {
    return syntaqlite_result_recovery_root(raw_);
  }
  const char* ResultErrorMsg() const {
    return syntaqlite_result_error_msg(raw_);
  }

  template <typename T>
  const T* Node(uint32_t node_id) const {
    return NodeCast<T>(raw_, node_id);
  }

  template <typename T>
  ListView<T> List(uint32_t list_id) const {
    return MakeListView<T>(raw_, list_id);
  }

  std::string_view Text(SyntaqliteSourceSpan span) const {
    return SpanText(raw_, span);
  }

  char* DumpNode(uint32_t node_id, uint32_t indent = 0) const {
    return syntaqlite_dump_node(raw_, node_id, indent);
  }

 private:
  SyntaqliteParser* raw_;
};

#ifndef SYNTAQLITE_OMIT_SQLITE_API
inline Parser SqliteParser() {
  return Parser(syntaqlite_parser_create(nullptr));
}
#endif

}  // namespace syntaqlite
#endif

#endif  // SYNTAQLITE_PARSER_H
