// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#include "syntaqlite/parser.h"

#include <stdio.h>
#include <string.h>

#include "csrc/arena.h"
#include "csrc/ast_builder.h"
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
  uint32_t offset;           // Tokenizer cursor into source.
  int last_token_type;       // Last non-whitespace token fed to Lemon.
  int finished;              // 1 after EOF has been sent to Lemon.
  int had_error;             // Sticky error flag.
  char error_msg[256];       // Error message buffer.
  int trace;
  int collect_tokens;
  const SyntaqliteDialectExtension* extension;
  SYNQ_VEC(SyntaqliteTrivia) trivia;
  int macro_depth;           // Nesting depth (0 = not in macro).
  SYNQ_VEC(SyntaqliteMacroRegion) macros;
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
  synq_vec_init(&p->trivia);
  synq_vec_init(&p->macros);
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
  p->last_token_type = 0;
  p->finished = 0;
  p->had_error = 0;
  p->error_msg[0] = '\0';
  synq_vec_clear(&p->trivia);
  p->macro_depth = 0;
  synq_vec_clear(&p->macros);

  // Reset parse context.
  p->ctx.source = source;
  p->ctx.root = SYNTAQLITE_NULL_NODE;
  p->ctx.stmt_completed = 0;
  p->ctx.error = 0;
  p->ctx.error_offset = 0;
}

// ---------------------------------------------------------------------------
// Internal: feed one real token to Lemon.
// Returns: 0 = keep going, 1 = statement completed, -1 = error.
// ---------------------------------------------------------------------------

static int feed_one_token(SyntaqliteParser* p, int token_type,
                           const char* text, int len) {
  SynqToken minor = {.z = text, .n = len, .type = token_type};
  SyntaqliteParse(p->lemon, token_type, minor, &p->ctx);
  p->last_token_type = token_type;

  if (p->ctx.error) {
    p->had_error = 1;
    if (p->error_msg[0] == '\0') {
      snprintf(p->error_msg, sizeof(p->error_msg),
               "syntax error near '%.*s'", len, text ? text : "");
    }
    return -1;
  }

  if (p->ctx.stmt_completed) {
    p->ctx.stmt_completed = 0;
    return 1;
  }

  return 0;
}

// ---------------------------------------------------------------------------
// Internal: check macro straddle after statement completion.
// ---------------------------------------------------------------------------

// Check if a macro region boundary crosses through a single AST node.
// A straddle means the node has SourceSpan fields both inside and outside
// a macro region — the macro didn't expand to a complete syntactic unit.
// Returns 0 on success, -1 if straddle detected.
static int check_macro_straddle(SyntaqliteParser* p) {
  uint32_t macro_count = synq_vec_len(&p->macros);
  if (macro_count == 0) return 0;

  uint32_t node_count = synq_vec_len(&p->ctx.ast.offsets);

  for (uint32_t nid = 0; nid < node_count; nid++) {
    const SyntaqliteNode* node =
        (const SyntaqliteNode*)synq_arena_ptr(&p->ctx.ast, nid);
    uint32_t tag = node->tag;
    if (tag == 0 || tag >= SYNTAQLITE_NODE_COUNT) continue;

    const SynqFieldRangeMeta* fields = range_meta_table[tag].fields;
    uint8_t field_count = range_meta_table[tag].count;
    if (fields == NULL || field_count == 0) continue;  // List node or empty.

    // Check each macro region for straddle: some SourceSpan fields inside,
    // some outside.
    for (uint32_t mi = 0; mi < macro_count; mi++) {
      SyntaqliteMacroRegion r = synq_vec_at(&p->macros, mi);
      uint32_t r_start = r.call_offset;
      uint32_t r_end = r_start + r.call_length;

      int has_inside = 0;
      int has_outside = 0;

      for (uint8_t fi = 0; fi < field_count; fi++) {
        if (fields[fi].kind != 1) continue;  // Not a SourceSpan.
        const SyntaqliteSourceSpan* sp =
            (const SyntaqliteSourceSpan*)((const char*)node + fields[fi].offset);
        if (sp->length == 0) continue;

        uint32_t s_start = sp->offset;
        uint32_t s_end = sp->offset + sp->length;

        if (s_start >= r_start && s_end <= r_end) {
          has_inside = 1;
        } else {
          has_outside = 1;
        }
      }

      if (has_inside && has_outside) {
        snprintf(p->error_msg, sizeof(p->error_msg),
                 "macro expansion straddles node boundary");
        p->had_error = 1;
        return -1;
      }
    }
  }
  return 0;
}

// ---------------------------------------------------------------------------
// Internal: synthesize SEMI + EOF to finish parsing.
// Returns: 0 = done, 1 = final statement completed, -1 = error.
// ---------------------------------------------------------------------------

static int finish_input(SyntaqliteParser* p) {
  // Nothing to do if no tokens were ever fed.
  if (p->last_token_type == 0) {
    p->finished = 1;
    return 0;
  }

  // Synthesize SEMI if the last token wasn't one.
  if (p->last_token_type != SYNTAQLITE_TK_SEMI) {
    int rc = feed_one_token(p, SYNTAQLITE_TK_SEMI, NULL, 0);
    if (rc < 0) {
      p->finished = 1;
      snprintf(p->error_msg, sizeof(p->error_msg),
               "incomplete SQL statement");
      return -1;
    }
    if (rc == 1 && p->ctx.root != SYNTAQLITE_NULL_NODE) {
      p->finished = 1;
      return 1;
    }
  }

  // Send end-of-input (EOF) to flush the final reduction. LALR(1) parsers
  // need one token of lookahead — the EOF provides it, triggering any
  // pending reduce (e.g. ecmd ::= cmdx SEMI).
  SynqToken eof = {.z = NULL, .n = 0, .type = 0};
  SyntaqliteParse(p->lemon, 0, eof, &p->ctx);
  p->finished = 1;

  if (p->ctx.error) {
    p->had_error = 1;
    if (p->error_msg[0] == '\0') {
      snprintf(p->error_msg, sizeof(p->error_msg),
               "incomplete SQL statement");
    }
    return -1;
  }

  if (p->ctx.root != SYNTAQLITE_NULL_NODE) {
    if (check_macro_straddle(p) < 0) return -1;
    return 1;
  }

  return 0;
}

// ---------------------------------------------------------------------------
// High-level API
// ---------------------------------------------------------------------------

SyntaqliteParseResult syntaqlite_parser_next(SyntaqliteParser* p) {
  SyntaqliteParseResult result = {SYNTAQLITE_NULL_NODE, 0, NULL};

  if (p->finished) {
    if (p->had_error) {
      result.error = 1;
      result.error_msg = p->error_msg;
    }
    return result;
  }

  // Reset per-statement state.
  p->ctx.root = SYNTAQLITE_NULL_NODE;
  p->ctx.stmt_completed = 0;
  p->ctx.error = 0;

  const unsigned char* z = (const unsigned char*)p->source;

  while (p->offset < p->source_len && z[p->offset] != '\0') {
    int token_type = 0;
    i64 token_len = synq_sqlite3GetToken(z + p->offset, &token_type);
    if (token_len <= 0)
      break;

    uint32_t tok_offset = p->offset;
    p->offset += (uint32_t)token_len;

    // Skip whitespace.
    if (token_type == SYNTAQLITE_TK_SPACE) {
      continue;
    }

    // Capture comments as trivia when collect_tokens is enabled.
    if (token_type == SYNTAQLITE_TK_COMMENT) {
      if (p->collect_tokens) {
        SyntaqliteTrivia t = {
            tok_offset, (uint32_t)token_len,
            z[tok_offset] == '-' ? (uint8_t)0 : (uint8_t)1};
        synq_vec_push(&p->trivia, t, p->mem);
      }
      continue;
    }

    int rc = feed_one_token(p, token_type, p->source + tok_offset,
                            (int)token_len);
    if (rc < 0) {
      p->finished = 1;
      result.error = 1;
      result.error_msg = p->error_msg;
      return result;
    }

    if (rc == 1) {
      // Bare semicolons produce SYNTAQLITE_NULL_NODE — skip them.
      if (p->ctx.root == SYNTAQLITE_NULL_NODE) {
        continue;
      }
      result.root = p->ctx.root;
      return result;
    }
  }

  // End of input.
  int rc = finish_input(p);
  if (rc < 0) {
    result.error = 1;
    result.error_msg = p->error_msg;
  } else if (rc == 1) {
    result.root = p->ctx.root;
  }
  return result;
}

// ---------------------------------------------------------------------------
// Low-level token-feeding API
// ---------------------------------------------------------------------------

int syntaqlite_parser_feed_token(SyntaqliteParser* p,
                                  int token_type,
                                  const char* text,
                                  int len) {
  // Skip whitespace silently.
  if (token_type == SYNTAQLITE_TK_SPACE) {
    return 0;
  }

  // Record comments as trivia but don't feed to Lemon.
  if (token_type == SYNTAQLITE_TK_COMMENT) {
    if (p->collect_tokens && text) {
      uint32_t tok_offset = (uint32_t)(text - p->source);
      SyntaqliteTrivia t = {
          tok_offset, (uint32_t)len,
          (uint8_t)(text[0] == '-' ? 0 : 1)};
      synq_vec_push(&p->trivia, t, p->mem);
    }
    return 0;
  }

  // Reset per-statement state if starting fresh.
  if (p->last_token_type == 0 ||
      p->ctx.root != SYNTAQLITE_NULL_NODE) {
    p->ctx.root = SYNTAQLITE_NULL_NODE;
    p->ctx.stmt_completed = 0;
    p->ctx.error = 0;
  }

  int rc = feed_one_token(p, token_type, text, len);
  if (rc < 0) return rc;

  if (rc == 1 && p->ctx.root == SYNTAQLITE_NULL_NODE) {
    // Bare semicolon — not a real statement.
    return 0;
  }

  if (rc == 1 && check_macro_straddle(p) < 0) {
    return -1;
  }

  return rc;
}

SyntaqliteParseResult syntaqlite_parser_result(SyntaqliteParser* p) {
  SyntaqliteParseResult result = {SYNTAQLITE_NULL_NODE, 0, NULL};
  if (p->had_error) {
    result.error = 1;
    result.error_msg = p->error_msg;
  } else if (p->ctx.root != SYNTAQLITE_NULL_NODE) {
    result.root = p->ctx.root;
  }
  return result;
}

int syntaqlite_parser_finish(SyntaqliteParser* p) {
  return finish_input(p);
}

// ---------------------------------------------------------------------------
// Macro region tracking
// ---------------------------------------------------------------------------

void syntaqlite_parser_begin_macro(SyntaqliteParser* p,
                                    uint32_t call_offset,
                                    uint32_t call_length) {
  SyntaqliteMacroRegion region = {call_offset, call_length};
  synq_vec_push(&p->macros, region, p->mem);
  p->macro_depth++;
}

void syntaqlite_parser_end_macro(SyntaqliteParser* p) {
  if (p->macro_depth > 0) {
    p->macro_depth--;
  }
}

const SyntaqliteMacroRegion* syntaqlite_parser_macro_regions(
    SyntaqliteParser* p, uint32_t* count) {
  *count = synq_vec_len(&p->macros);
  return p->macros.data;
}

void syntaqlite_parser_destroy(SyntaqliteParser* p) {
  if (p) {
    SyntaqliteParseFree(p->lemon, p->mem.xFree);
    synq_parse_ctx_free(&p->ctx);
    synq_vec_free(&p->trivia, p->mem);
    synq_vec_free(&p->macros, p->mem);
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


const SyntaqliteTrivia* syntaqlite_parser_trivia(SyntaqliteParser* p,
                                                  uint32_t* count) {
  *count = synq_vec_len(&p->trivia);
  return p->trivia.data;
}

void syntaqlite_parser_set_extension(
    SyntaqliteParser* p,
    const SyntaqliteDialectExtension* ext) {
  p->extension = ext;
}
