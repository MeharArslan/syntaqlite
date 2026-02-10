// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Parse context and AST builder interface.
// Provides:
//   - SynqParseCtx: parse/AST state threaded via %extra_argument
//   - SynqToken: terminal token type (used as %token_type in lemon grammar)
//   - synq_span(): converts SynqToken to SyntaqliteSourceSpan
//   - AST builder functions: synq_parse_build, synq_parse_list_append, etc.
//   - AST_NODE macro for in-place AST node mutation
//   - Custom struct types for multi-valued grammar nonterminals
//
// Grammar actions receive pCtx via lemon's %extra_argument mechanism.

#ifndef SYNQ_PARSER_H
#define SYNQ_PARSER_H

#include <stdint.h>
#include <string.h>

#include "csrc/arena.h"
#include "csrc/vec.h"
#include "syntaqlite/node.h"

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// List descriptor: lightweight metadata for one in-progress list.
// ---------------------------------------------------------------------------

typedef struct SynqListDesc {
  uint32_t node_id;  // reserved arena ID
  uint32_t offset;   // start index into child_buf
  uint32_t tag;
} SynqListDesc;

// ---------------------------------------------------------------------------
// Parse context — threaded through grammar actions via %extra_argument
// ---------------------------------------------------------------------------

typedef struct SynqParseCtx {
  // AST storage
  SyntaqliteMemMethods mem;
  SynqArena ast;
  SYNQ_VEC(uint32_t) child_buf;
  SYNQ_VEC(SynqListDesc) list_stack;

  // Parser state
  const char* source;     // Source text base pointer (for offset computation).
  uint32_t root;          // Root node ID of the current statement.
  int stmt_completed;     // Set by grammar actions when ecmd reduces.
  int error;              // Set when a syntax error occurs.
  uint32_t error_offset;  // Byte offset of the error token in source.
} SynqParseCtx;

void synq_parse_ctx_init(SynqParseCtx* ctx, SyntaqliteMemMethods mem);
void synq_parse_ctx_free(SynqParseCtx* ctx);

// Reset to empty state, keeping allocated memory for reuse.
void synq_parse_ctx_clear(SynqParseCtx* ctx);

// ---------------------------------------------------------------------------
// AST node access macro (for in-place mutation in grammar actions)
// ---------------------------------------------------------------------------

#define AST_NODE(arena_ptr, id) \
  ((SyntaqliteNode*)synq_arena_ptr((arena_ptr), (id)))

// ---------------------------------------------------------------------------
// AST builder functions
// ---------------------------------------------------------------------------

// Generic node builder: copy node data into the arena.
uint32_t synq_parse_build(SynqParseCtx* ctx,
                          const void* node_data,
                          uint32_t node_size);

uint32_t synq_parse_list_append(SynqParseCtx* ctx,
                                uint32_t tag,
                                uint32_t list_id,
                                uint32_t child);

void synq_parse_list_flush(SynqParseCtx* ctx);

// ---------------------------------------------------------------------------
// Token type — used as %token_type in the lemon grammar
// ---------------------------------------------------------------------------
//
// Terminals carry a pointer to the source text, the token length, and the
// token type ID.  Grammar actions access these via .z, .n, and .type.

typedef struct SynqToken {
  const char* z;  // Pointer to start of token in source text.
  int n;          // Length of token in bytes.
  int type;       // Token type ID (SYNTAQLITE_TK_*).
} SynqToken;

// ---------------------------------------------------------------------------
// Token → span conversion
// ---------------------------------------------------------------------------

static inline SyntaqliteSourceSpan synq_span(SynqParseCtx* ctx,
                                             SynqToken tok) {
  if (tok.z == NULL) return (SyntaqliteSourceSpan){0, 0};
  uint32_t offset = (uint32_t)(tok.z - ctx->source);
  return (SyntaqliteSourceSpan){
      .offset = offset,
      .length = (uint16_t)tok.n,
  };
}

#define SYNQ_NO_SPAN ((SyntaqliteSourceSpan){0, 0})

// ---------------------------------------------------------------------------
// Custom struct types for multi-valued grammar nonterminals
// ---------------------------------------------------------------------------

// columnname: passes name span + typetoken span from column definition.
typedef struct SynqColumnNameValue {
  SyntaqliteSourceSpan name;
  SyntaqliteSourceSpan typetoken;
} SynqColumnNameValue;

// ccons / tcons / generated: a constraint node + pending constraint name.
typedef struct SynqConstraintValue {
  uint32_t node;
  SyntaqliteSourceSpan pending_name;
} SynqConstraintValue;

// carglist / conslist: accumulated constraint list + pending name for next.
typedef struct SynqConstraintListValue {
  uint32_t list;
  SyntaqliteSourceSpan pending_name;
} SynqConstraintListValue;

// on_using: ON expr / USING column-list discriminator.
typedef struct SynqOnUsingValue {
  uint32_t on_expr;
  uint32_t using_cols;
} SynqOnUsingValue;

// with: recursive flag + CTE list node ID.
typedef struct SynqWithValue {
  uint32_t cte_list;
  int is_recursive;
} SynqWithValue;

#ifdef __cplusplus
}
#endif

#endif  // SYNQ_PARSER_H
