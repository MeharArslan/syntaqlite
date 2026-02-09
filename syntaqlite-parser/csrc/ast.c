// Internal AST builder implementation.

#include "csrc/ast.h"
#include "csrc/ast_builder.h"

#include <string.h>

uint32_t synq_ast_list_empty(SynqAstContext *ctx, uint8_t tag, size_t size) {
    uint32_t id = synq_arena_alloc(&ctx->ast, tag, size);
    uint8_t *base = ctx->ast.data + ctx->ast.offsets[id];
    memset(base, 0, size);
    base[0] = tag;
    synq_ast_ranges_sync(ctx);
    return id;
}

uint32_t synq_ast_build(SynqAstContext *ctx, uint8_t tag,
                        const void *node_data, size_t node_size) {
    uint32_t id = synq_arena_alloc(&ctx->ast, tag, node_size);
    void *dest = ctx->ast.data + ctx->ast.offsets[id];
    memcpy(dest, node_data, node_size);

    // Table-driven range computation
    if (tag < SYNTAQLITE_NODE_COUNT && range_meta_table[tag].count > 0) {
        synq_ast_ranges_sync(ctx);
        SynqSourceRange _r = {UINT32_MAX, 0};
        const SynqFieldRangeMeta *fields = range_meta_table[tag].fields;
        uint8_t count = range_meta_table[tag].count;
        const uint8_t *base = (const uint8_t *)dest;
        for (uint8_t i = 0; i < count; i++) {
            if (fields[i].kind == 0) {
                uint32_t child_id;
                memcpy(&child_id, base + fields[i].offset, sizeof(uint32_t));
                synq_ast_range_union(ctx, &_r, child_id);
            } else {
                SyntaqliteSourceSpan span;
                memcpy(&span, base + fields[i].offset, sizeof(SyntaqliteSourceSpan));
                synq_ast_range_union_span(&_r, span);
            }
        }
        if (_r.first != UINT32_MAX) ctx->ranges.data[id] = _r;
    }

    return id;
}
