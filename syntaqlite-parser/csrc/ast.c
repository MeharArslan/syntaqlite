// Internal AST builder implementation.

#include "csrc/ast.h"

// Common header for all list nodes in the arena.
typedef struct {
    uint32_t tag;
    uint32_t count;
} SynqListHeader;

// Flush the topmost list from the stack into the arena.
static void list_flush_top(SynqAstContext *ctx) {
    SynqListDesc *desc = &ctx->list_stack.data[ctx->list_stack.count - 1];
    uint32_t count = ctx->child_buf.count - desc->offset;
    uint32_t children_size = count * (uint32_t)sizeof(uint32_t);

    SynqListHeader hdr = { .tag = desc->tag, .count = count };
    synq_arena_commit(&ctx->ast, desc->node_id, &hdr, (uint32_t)sizeof(hdr));
    synq_arena_append(&ctx->ast, ctx->child_buf.data + desc->offset, children_size);

    // Truncate child_buf and pop descriptor
    ctx->child_buf.count = desc->offset;
    ctx->list_stack.count--;
}

void synq_ast_ctx_init(SynqAstContext *ctx) {
    synq_arena_init(&ctx->ast);
    synq_vec_init(&ctx->child_buf);
    synq_vec_init(&ctx->list_stack);
}

void synq_ast_ctx_free(SynqAstContext *ctx) {
    synq_vec_free(&ctx->child_buf);
    synq_vec_free(&ctx->list_stack);
    synq_arena_free(&ctx->ast);
}

uint32_t synq_ast_build(SynqAstContext *ctx,
                        const void *node_data, uint32_t node_size) {
    return synq_arena_alloc(&ctx->ast, node_data, node_size);
}

uint32_t synq_ast_list_start(SynqAstContext *ctx, uint32_t tag, uint32_t first_child) {
    SynqListDesc desc;
    desc.node_id = synq_arena_reserve_id(&ctx->ast);
    desc.offset = ctx->child_buf.count;
    desc.tag = tag;

    synq_vec_push(&ctx->list_stack, desc);
    synq_vec_push(&ctx->child_buf, first_child);
    return desc.node_id;
}

void synq_ast_list_append(SynqAstContext *ctx, uint32_t list_id, uint32_t child) {
    // Auto-flush completed inner lists above the target
    while (ctx->list_stack.data[ctx->list_stack.count - 1].node_id != list_id) {
        list_flush_top(ctx);
    }
    synq_vec_push(&ctx->child_buf, child);
}

void synq_ast_list_flush(SynqAstContext *ctx) {
    while (ctx->list_stack.count > 0) {
        list_flush_top(ctx);
    }
}
