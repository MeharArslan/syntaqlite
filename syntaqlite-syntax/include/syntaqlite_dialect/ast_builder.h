// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Parse context and AST builder interface.
// Provides:
//   - SynqParseCtx: parse/AST state threaded via %extra_argument
//   - SynqParseToken: terminal token type (used as %token_type in lemon
//   grammar)
//   - synq_span(): converts SynqParseToken to SyntaqliteSourceSpan
//   - AST builder functions: synq_parse_build, synq_parse_list_append, etc.
//   - AST_NODE macro for in-place AST node mutation
//
// Grammar actions receive pCtx via lemon's %extra_argument mechanism.

#ifndef SYNTAQLITE_EXT_AST_BUILDER_H
#define SYNTAQLITE_EXT_AST_BUILDER_H

#include <stdint.h>
#include <string.h>

#include "syntaqlite/grammar.h"
#include "syntaqlite/parser.h"
#include "syntaqlite/types.h"
#include "syntaqlite_dialect/arena.h"
#include "syntaqlite_dialect/vec.h"

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
  const char* source;  // Source text base pointer (for offset computation).
  const SyntaqliteGrammar*
      env;                // Dialect env (for cflag checks in actions).
  uint32_t root;          // Root node ID of the current statement.
  int stmt_completed;     // Set by grammar actions when ecmd reduces.
  int error;              // Set when a syntax error occurs.
  uint32_t error_offset;  // Byte offset of the error token in source.
  uint32_t error_length;  // Byte length of the error token.
  int saw_subquery;       // Set by grammar actions when a subquery is reduced.
  int saw_update_delete_limit;  // Set when ORDER BY / LIMIT used on DELETE or
                                // UPDATE.

  // Token marking — points to the parser's token list (NULL if not collecting).
  // Typed as void* because SYNQ_VEC produces anonymous struct types; the
  // synq_mark_as_id() helper casts it to the right layout.
  void* tokens;
} SynqParseCtx;

// Common header for all list nodes in the arena.
typedef struct SynqListHeader {
  uint32_t tag;
  uint32_t count;
} SynqListHeader;

// ---------------------------------------------------------------------------
// AST node access macro (for in-place mutation in grammar actions)
// ---------------------------------------------------------------------------

// Cast the arena pointer for a node ID to a void pointer.
// Dialect code should further cast to the dialect-specific node union.
#define AST_NODE(arena_ptr, id) ((void*)synq_arena_ptr((arena_ptr), (id)))

// ---------------------------------------------------------------------------
// AST builder functions
// ---------------------------------------------------------------------------

// Flush the topmost list from the stack into the arena.
static inline void synq_parse_list_flush_top(SynqParseCtx* ctx) {
  SynqListDesc* desc = &syntaqlite_vec_at(
      &ctx->list_stack, syntaqlite_vec_len(&ctx->list_stack) - 1);
  uint32_t count = syntaqlite_vec_len(&ctx->child_buf) - desc->offset;
  uint32_t children_size = count * (uint32_t)sizeof(uint32_t);

  SynqListHeader hdr = {.tag = desc->tag, .count = count};
  synq_arena_commit(&ctx->ast, desc->node_id, &hdr, (uint32_t)sizeof(hdr),
                    ctx->mem);
  synq_arena_append(&ctx->ast,
                    &syntaqlite_vec_at(&ctx->child_buf, desc->offset),
                    children_size, ctx->mem);

  syntaqlite_vec_truncate(&ctx->child_buf, desc->offset);
  syntaqlite_vec_pop(&ctx->list_stack);
}

static inline void synq_parse_ctx_init(SynqParseCtx* ctx,
                                       SyntaqliteMemMethods mem) {
  ctx->mem = mem;
  synq_arena_init(&ctx->ast);
  syntaqlite_vec_init(&ctx->child_buf);
  syntaqlite_vec_init(&ctx->list_stack);
}

static inline void synq_parse_ctx_free(SynqParseCtx* ctx) {
  syntaqlite_vec_free(&ctx->child_buf, ctx->mem);
  syntaqlite_vec_free(&ctx->list_stack, ctx->mem);
  synq_arena_free(&ctx->ast, ctx->mem);
}

// Reset to empty state, keeping allocated memory for reuse.
static inline void synq_parse_ctx_clear(SynqParseCtx* ctx) {
  syntaqlite_vec_clear(&ctx->child_buf);
  syntaqlite_vec_clear(&ctx->list_stack);
  synq_arena_clear(&ctx->ast);
}

// Generic node builder: copy node data into the arena.
static inline uint32_t synq_parse_build(SynqParseCtx* ctx,
                                        const void* node_data,
                                        uint32_t node_size) {
  return synq_arena_alloc(&ctx->ast, node_data, node_size, ctx->mem);
}

// Build an error placeholder node in the arena. The returned node ID can be
// stored in any node field that would normally hold a valid node ID, allowing
// the arena to record where an error occurred for later inspection.
static inline uint32_t synq_parse_error_node(SynqParseCtx* ctx,
                                             uint32_t offset,
                                             uint32_t length) {
  SyntaqliteErrorNode node = {
      .tag = SYNTAQLITE_ERROR_NODE_TAG,
      .offset = offset,
      .length = length,
  };
  return synq_parse_build(ctx, &node, (uint32_t)sizeof(node));
}

static inline uint32_t synq_parse_list_append(SynqParseCtx* ctx,
                                              uint32_t tag,
                                              uint32_t list_id,
                                              uint32_t child) {
  if (list_id == SYNTAQLITE_NULL_NODE) {
    SynqListDesc desc;
    desc.node_id = synq_arena_reserve_id(&ctx->ast, ctx->mem);
    desc.offset = syntaqlite_vec_len(&ctx->child_buf);
    desc.tag = tag;
    syntaqlite_vec_push(&ctx->list_stack, desc, ctx->mem);
    syntaqlite_vec_push(&ctx->child_buf, child, ctx->mem);
    return desc.node_id;
  }

  // Auto-flush completed inner lists above the target.
  while (syntaqlite_vec_at(&ctx->list_stack,
                           syntaqlite_vec_len(&ctx->list_stack) - 1)
             .node_id != list_id) {
    synq_parse_list_flush_top(ctx);
  }
  syntaqlite_vec_push(&ctx->child_buf, child, ctx->mem);
  return list_id;
}

static inline void synq_parse_list_flush(SynqParseCtx* ctx) {
  while (syntaqlite_vec_len(&ctx->list_stack) > 0) {
    synq_parse_list_flush_top(ctx);
  }
}

// ---------------------------------------------------------------------------
// Token → span conversion
// ---------------------------------------------------------------------------

static inline SyntaqliteSourceSpan synq_span(SynqParseCtx* ctx,
                                             SynqParseToken tok) {
  if (tok.z == NULL)
    return (SyntaqliteSourceSpan){0, 0};
  uint32_t offset = (uint32_t)(tok.z - ctx->source);
  return (SyntaqliteSourceSpan){
      .offset = offset,
      .length = (uint16_t)tok.n,
  };
}

#define SYNQ_NO_SPAN ((SyntaqliteSourceSpan){0, 0})

// Mark a token as "used as identifier" (fallback from keyword).
// O(1) — uses the token_idx stored in SynqParseToken at collection time.
static inline void synq_mark_as_id(SynqParseCtx* ctx, SynqParseToken tok) {
  if (!ctx->tokens || tok.token_idx == 0xFFFFFFFF)
    return;
  // ctx->tokens is a void* pointing to SYNQ_VEC(SyntaqliteTokenPos).
  // The vec layout is: { SyntaqliteTokenPos* data; uint32_t count; uint32_t
  // capacity; }
  typedef struct {
    SyntaqliteTokenPos* data;
    uint32_t count;
    uint32_t capacity;
  } TokenVec;
  TokenVec* tv = (TokenVec*)ctx->tokens;
  tv->data[tok.token_idx].flags |= SYNQ_TOKEN_FLAG_AS_ID;
}

// Mark a token as "used as function name" in a function-call expression.
// O(1) — uses the token_idx stored in SynqParseToken at collection time.
static inline void synq_mark_as_function(SynqParseCtx* ctx,
                                         SynqParseToken tok) {
  if (!ctx->tokens || tok.token_idx == 0xFFFFFFFF)
    return;
  // ctx->tokens is a void* pointing to SYNQ_VEC(SyntaqliteTokenPos).
  // The vec layout is: { SyntaqliteTokenPos* data; uint32_t count; uint32_t
  // capacity; }
  typedef struct {
    SyntaqliteTokenPos* data;
    uint32_t count;
    uint32_t capacity;
  } TokenVec;
  TokenVec* tv = (TokenVec*)ctx->tokens;
  tv->data[tok.token_idx].flags |= SYNQ_TOKEN_FLAG_AS_FUNCTION;
}

// Mark a token as "used as type name" in type contexts.
// O(1) — uses the token_idx stored in SynqParseToken at collection time.
static inline void synq_mark_as_type(SynqParseCtx* ctx, SynqParseToken tok) {
  if (!ctx->tokens || tok.token_idx == 0xFFFFFFFF)
    return;
  // ctx->tokens is a void* pointing to SYNQ_VEC(SyntaqliteTokenPos).
  // The vec layout is: { SyntaqliteTokenPos* data; uint32_t count; uint32_t
  // capacity; }
  typedef struct {
    SyntaqliteTokenPos* data;
    uint32_t count;
    uint32_t capacity;
  } TokenVec;
  TokenVec* tv = (TokenVec*)ctx->tokens;
  tv->data[tok.token_idx].flags |= SYNQ_TOKEN_FLAG_AS_TYPE;
}

// Range field metadata types (SyntaqliteFieldRangeMeta,
// SyntaqliteRangeMetaEntry) are defined in syntaqlite/grammar.h.

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_EXT_AST_BUILDER_H
