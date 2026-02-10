// Internal AST builder implementation.

#include "csrc/ast.h"

// Common header for all list nodes in the arena.
typedef struct {
  uint32_t tag;
  uint32_t count;
} SynqListHeader;

// Flush the topmost list from the stack into the arena.
static void list_flush_top(SynqAstContext* ctx) {
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

void synq_ast_ctx_init(SynqAstContext* ctx, SyntaqliteMemMethods mem) {
  ctx->mem = mem;
  synq_arena_init(&ctx->ast);
  synq_vec_init(&ctx->child_buf);
  synq_vec_init(&ctx->list_stack);
}

void synq_ast_ctx_free(SynqAstContext* ctx) {
  synq_vec_free(&ctx->child_buf, ctx->mem);
  synq_vec_free(&ctx->list_stack, ctx->mem);
  synq_arena_free(&ctx->ast, ctx->mem);
}

uint32_t synq_ast_build(SynqAstContext* ctx,
                        const void* node_data,
                        uint32_t node_size) {
  return synq_arena_alloc(&ctx->ast, node_data, node_size, ctx->mem);
}

uint32_t synq_ast_list_append(SynqAstContext* ctx,
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

void synq_ast_list_flush(SynqAstContext* ctx) {
  while (synq_vec_len(&ctx->list_stack) > 0) {
    list_flush_top(ctx);
  }
}
