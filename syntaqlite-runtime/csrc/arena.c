
// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Shared arena allocator with offset table.

#include "csrc/arena.h"

void synq_arena_init(SynqArena* a) {
  synq_vec_init(&a->data);
  synq_vec_init(&a->offsets);
}

void synq_arena_free(SynqArena* a, SyntaqliteMemMethods mem) {
  synq_vec_free(&a->data, mem);
  synq_vec_free(&a->offsets, mem);
}

void synq_arena_clear(SynqArena* a) {
  synq_vec_clear(&a->data);
  synq_vec_clear(&a->offsets);
}

uint32_t synq_arena_alloc(SynqArena* a, const void* data, uint32_t size,
                          SyntaqliteMemMethods mem) {
  uint32_t node_id = synq_vec_len(&a->offsets);
  synq_vec_push(&a->offsets, synq_vec_len(&a->data), mem);
  synq_vec_push_n(&a->data, data, size, mem);
  return node_id;
}

uint32_t synq_arena_reserve_id(SynqArena* a, SyntaqliteMemMethods mem) {
  uint32_t node_id = synq_vec_len(&a->offsets);
  synq_vec_push(&a->offsets, 0, mem);
  return node_id;
}

void synq_arena_commit(SynqArena* a, uint32_t node_id, const void* data,
                       uint32_t size, SyntaqliteMemMethods mem) {
  synq_vec_at(&a->offsets, node_id) = synq_vec_len(&a->data);
  synq_vec_push_n(&a->data, data, size, mem);
}

void synq_arena_append(SynqArena* a, const void* data, uint32_t size,
                       SyntaqliteMemMethods mem) {
  synq_vec_push_n(&a->data, data, size, mem);
}
