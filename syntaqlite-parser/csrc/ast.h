// Internal AST builder — generic node allocation with range computation.

#ifndef SYNQ_AST_H
#define SYNQ_AST_H

#include <stddef.h>
#include <stdint.h>

#include "syntaqlite/ast.h"

#ifdef __cplusplus
extern "C" {
#endif

// TODO: replace with real definitions once wired up
typedef struct SynqArena {
    uint8_t *data;
    uint32_t *offsets;
} SynqArena;

typedef struct SynqSourceRange {
    uint32_t first;
    uint32_t last;
} SynqSourceRange;

typedef struct SynqRanges {
    SynqSourceRange *data;
} SynqRanges;

typedef struct SynqAstContext {
    SynqArena ast;
    SynqRanges ranges;
} SynqAstContext;

uint32_t synq_arena_alloc(SynqArena *arena, uint8_t tag, size_t size);
uint32_t synq_ast_list_empty(SynqAstContext *ctx, uint8_t tag, size_t size);
uint32_t synq_ast_list_start(SynqAstContext *ctx, uint8_t tag, uint32_t first_child);
uint32_t synq_ast_list_append(SynqAstContext *ctx, uint32_t list_id, uint32_t child, uint8_t tag);
void synq_ast_ranges_sync(SynqAstContext *ctx);
void synq_ast_range_union(SynqAstContext *ctx, SynqSourceRange *r, uint32_t child_id);
void synq_ast_range_union_span(SynqSourceRange *r, SyntaqliteSourceSpan span);

// Generic node builder: arena alloc + memcpy + table-driven range computation.
// All per-node static inline builders delegate to this.
uint32_t synq_ast_build(SynqAstContext *ctx, uint8_t tag,
                        const void *node_data, size_t node_size);

#ifdef __cplusplus
}
#endif

#endif  // SYNQ_AST_H
