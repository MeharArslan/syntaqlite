// Internal AST builder — generic node allocation with range computation.

#ifndef SYNQ_AST_H
#define SYNQ_AST_H

#include <stdint.h>

#include "csrc/arena.h"
#include "csrc/vec.h"
#include "syntaqlite/ast.h"

#ifdef __cplusplus
extern "C" {
#endif

// List descriptor: lightweight metadata for one in-progress list.
typedef struct SynqListDesc {
  uint32_t node_id;  // reserved arena ID
  uint32_t offset;   // start index into child_buf
  uint32_t tag;
} SynqListDesc;

typedef struct SynqAstContext {
  SyntaqliteMemMethods mem;
  SynqArena ast;

  // Shared child-ID buffer for all in-progress lists.
  // Each list owns a contiguous slice starting at its offset.
  SYNQ_VEC(uint32_t) child_buf;

  // Stack of list descriptors.  Inner lists sit above outer lists.
  SYNQ_VEC(SynqListDesc) list_stack;
} SynqAstContext;

void synq_ast_ctx_init(SynqAstContext* ctx, SyntaqliteMemMethods mem);
void synq_ast_ctx_free(SynqAstContext* ctx);

// Generic node builder: copy node data into the arena.
uint32_t synq_ast_build(SynqAstContext* ctx,
                        const void* node_data,
                        uint32_t node_size);

uint32_t synq_ast_list_append(SynqAstContext* ctx,
                              uint32_t tag,
                              uint32_t list_id,
                              uint32_t child);

void synq_ast_list_flush(SynqAstContext* ctx);

#ifdef __cplusplus
}
#endif

#endif  // SYNQ_AST_H
