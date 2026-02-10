// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#include "syntaqlite/parser.h"

#include <stdio.h>
#include <string.h>

#include "csrc/parser.h"
#include "csrc/sqlite_parser.h"
#include "csrc/sqlite_tokenize.h"
#include "syntaqlite/tokens.h"

// ---------------------------------------------------------------------------
// AST builder internals
// ---------------------------------------------------------------------------

// Common header for all list nodes in the arena.
typedef struct {
  uint32_t tag;
  uint32_t count;
} SynqListHeader;

// Flush the topmost list from the stack into the arena.
static void list_flush_top(SynqParseCtx* ctx) {
  SynqListDesc* desc =
      &synq_vec_at(&ctx->list_stack, synq_vec_len(&ctx->list_stack) - 1);
  uint32_t count = synq_vec_len(&ctx->child_buf) - desc->offset;
  uint32_t children_size = count * (uint32_t)sizeof(uint32_t);

  SynqListHeader hdr = {.tag = desc->tag, .count = count};
  synq_arena_commit(&ctx->ast, desc->node_id, &hdr, (uint32_t)sizeof(hdr),
                    ctx->mem);
  synq_arena_append(&ctx->ast, &synq_vec_at(&ctx->child_buf, desc->offset),
                    children_size, ctx->mem);

  synq_vec_truncate(&ctx->child_buf, desc->offset);
  synq_vec_pop(&ctx->list_stack);
}

void synq_parse_ctx_init(SynqParseCtx* ctx, SyntaqliteMemMethods mem) {
  ctx->mem = mem;
  synq_arena_init(&ctx->ast);
  synq_vec_init(&ctx->child_buf);
  synq_vec_init(&ctx->list_stack);
}

void synq_parse_ctx_free(SynqParseCtx* ctx) {
  synq_vec_free(&ctx->child_buf, ctx->mem);
  synq_vec_free(&ctx->list_stack, ctx->mem);
  synq_arena_free(&ctx->ast, ctx->mem);
}

void synq_parse_ctx_clear(SynqParseCtx* ctx) {
  synq_vec_clear(&ctx->child_buf);
  synq_vec_clear(&ctx->list_stack);
  synq_arena_clear(&ctx->ast);
}

uint32_t synq_parse_build(SynqParseCtx* ctx,
                          const void* node_data,
                          uint32_t node_size) {
  return synq_arena_alloc(&ctx->ast, node_data, node_size, ctx->mem);
}

uint32_t synq_parse_list_append(SynqParseCtx* ctx,
                                uint32_t tag,
                                uint32_t list_id,
                                uint32_t child) {
  if (list_id == SYNTAQLITE_NULL_NODE) {
    SynqListDesc desc;
    desc.node_id = synq_arena_reserve_id(&ctx->ast, ctx->mem);
    desc.offset = synq_vec_len(&ctx->child_buf);
    desc.tag = tag;
    synq_vec_push(&ctx->list_stack, desc, ctx->mem);
    synq_vec_push(&ctx->child_buf, child, ctx->mem);
    return desc.node_id;
  }
  // Auto-flush completed inner lists above the target
  while (synq_vec_at(&ctx->list_stack, synq_vec_len(&ctx->list_stack) - 1)
             .node_id != list_id) {
    list_flush_top(ctx);
  }
  synq_vec_push(&ctx->child_buf, child, ctx->mem);
  return list_id;
}

void synq_parse_list_flush(SynqParseCtx* ctx) {
  while (synq_vec_len(&ctx->list_stack) > 0) {
    list_flush_top(ctx);
  }
}

// ---------------------------------------------------------------------------
// Parser struct
// ---------------------------------------------------------------------------

struct SyntaqliteParser {
  SyntaqliteMemMethods mem;
  void* lemon;
  SynqParseCtx ctx;
  const char* source;
  uint32_t source_len;
  uint32_t offset;  // Tokenizer cursor into source.
  int trace;
  int collect_tokens;
  const SyntaqliteDialectExtension* extension;
};

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

SyntaqliteParser* syntaqlite_parser_create(const SyntaqliteMemMethods* mem) {
  SyntaqliteMemMethods m = mem ? *mem : SYNTAQLITE_MEM_METHODS_DEFAULT;
  SyntaqliteParser* p = m.xMalloc(sizeof(SyntaqliteParser));
  memset(p, 0, sizeof(*p));
  p->mem = m;
  p->lemon = SyntaqliteParseAlloc(m.xMalloc);
  synq_parse_ctx_init(&p->ctx, m);
  return p;
}

void syntaqlite_parser_reset(SyntaqliteParser* p,
                             const char* source,
                             uint32_t len) {
  // Clear AST arena — keeps allocated memory for reuse.
  synq_parse_ctx_clear(&p->ctx);

  // Re-initialize lemon parser state (reuses allocation).
  SyntaqliteParseFinalize(p->lemon);
  SyntaqliteParseInit(p->lemon);

  p->source = source;
  p->source_len = len;
  p->offset = 0;

  // Reset parse context.
  p->ctx.source = source;
  p->ctx.root = SYNTAQLITE_NULL_NODE;
  p->ctx.stmt_completed = 0;
  p->ctx.error = 0;
  p->ctx.error_offset = 0;
}

SyntaqliteParseResult syntaqlite_parser_next(SyntaqliteParser* p) {
  SyntaqliteParseResult result = {
      .root = SYNTAQLITE_NULL_NODE,
      .error = 0,
      .error_msg = NULL,
  };

  // Reset per-statement state.
  p->ctx.root = SYNTAQLITE_NULL_NODE;
  p->ctx.stmt_completed = 0;
  p->ctx.error = 0;

  int had_tokens = 0;

  while (p->offset < p->source_len) {
    int token_type = 0;
    i64 token_len = synq_sqlite3GetToken(
        (const unsigned char*)p->source + p->offset, &token_type);

    uint32_t offset = p->offset;
    p->offset += (uint32_t)token_len;

    // Skip whitespace and comments.
    if (token_type == SYNTAQLITE_TK_SPACE ||
        token_type == SYNTAQLITE_TK_COMMENT) {
      continue;
    }

    had_tokens = 1;
    SynqToken minor = {
        .z = p->source + offset,
        .n = (int)token_len,
        .type = token_type,
    };
    SyntaqliteParse(p->lemon, token_type, minor, &p->ctx);

    // A statement was completed (ecmd reduced).
    if (p->ctx.stmt_completed) {
      p->ctx.stmt_completed = 0;

      // Bare semicolons produce SYNTAQLITE_NULL_NODE — skip them.
      if (p->ctx.root == SYNTAQLITE_NULL_NODE) {
        had_tokens = 0;
        continue;
      }

      result.root = p->ctx.root;
      return result;
    }
  }

  // End of input — feed EOF to trigger final reductions.
  if (had_tokens) {
    SynqToken eof = {.z = NULL, .n = 0, .type = 0};
    SyntaqliteParse(p->lemon, 0, eof, &p->ctx);

    if (p->ctx.stmt_completed && p->ctx.root != SYNTAQLITE_NULL_NODE) {
      result.root = p->ctx.root;
    } else {
      // Tokens were fed but no statement completed — syntax error.
      result.error = 1;
      result.error_msg = "incomplete SQL statement";
    }
  }

  return result;
}

void syntaqlite_parser_destroy(SyntaqliteParser* p) {
  if (p) {
    SyntaqliteParseFree(p->lemon, p->mem.xFree);
    synq_parse_ctx_free(&p->ctx);
    p->mem.xFree(p);
  }
}

// ---------------------------------------------------------------------------
// Reading results
// ---------------------------------------------------------------------------

const SyntaqliteNode* syntaqlite_parser_node(SyntaqliteParser* p,
                                             uint32_t node_id) {
  return (const SyntaqliteNode*)synq_arena_ptr(&p->ctx.ast, node_id);
}

const char* syntaqlite_parser_source(SyntaqliteParser* p) {
  return p->source;
}

uint32_t syntaqlite_parser_source_length(SyntaqliteParser* p) {
  return p->source_len;
}

// ---------------------------------------------------------------------------
// Configuration
// ---------------------------------------------------------------------------

void syntaqlite_parser_set_trace(SyntaqliteParser* p, int enable) {
  p->trace = enable;
#ifndef NDEBUG
  if (enable) {
    SyntaqliteParseTrace(stderr, "parser> ");
  } else {
    SyntaqliteParseTrace(NULL, NULL);
  }
#else
  (void)p;
  (void)enable;
#endif
}

void syntaqlite_parser_set_collect_tokens(SyntaqliteParser* p, int enable) {
  p->collect_tokens = enable;
}

void syntaqlite_parser_set_extension(
    SyntaqliteParser* p,
    const SyntaqliteDialectExtension* ext) {
  p->extension = ext;
}
