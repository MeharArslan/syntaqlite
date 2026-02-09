
// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Shared arena allocator with offset table.

#include <string.h>

#include "csrc/arena.h"

void synq_arena_init(SynqArena* a) {
  synq_vec_init(&a->data);
  synq_vec_init(&a->offsets);
}

void synq_arena_free(SynqArena* a) {
  synq_vec_free(&a->data);
  synq_vec_free(&a->offsets);
}

uint32_t synq_arena_alloc(SynqArena* a, uint32_t tag, uint32_t size) {
  uint32_t node_id = synq_vec_len(&a->offsets);
  uint32_t offset = synq_vec_len(&a->data);
  synq_vec_push(&a->offsets, offset);

  uint8_t* dest;
  synq_vec_extend(&a->data, (uint32_t)size, dest);
  memcpy(dest, &tag, sizeof(tag));

  return node_id;
}

uint32_t synq_arena_reserve_id(SynqArena* a) {
  uint32_t node_id = synq_vec_len(&a->offsets);
  synq_vec_push(&a->offsets, 0);
  return node_id;
}

void* synq_arena_commit(SynqArena* a, uint32_t node_id, uint32_t size) {
  synq_vec_at(&a->offsets, node_id) = synq_vec_len(&a->data);
  uint8_t* dest;
  synq_vec_extend(&a->data, (uint32_t)size, dest);
  return dest;
}
