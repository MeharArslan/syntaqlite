
// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Arena allocator with offset table for node-based data structures.

#ifndef SYNQ_SRC_BASE_ARENA_H
#define SYNQ_SRC_BASE_ARENA_H

#include <stdint.h>

#include "csrc/vec.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SynqArena {
  SYNQ_VEC(uint8_t) data;
  SYNQ_VEC(uint32_t) offsets;
} SynqArena;

// Get pointer to node data by offset-table ID.
#define synq_arena_ptr(a, id) \
  (&synq_vec_at(&(a)->data, synq_vec_at(&(a)->offsets, id)))

void synq_arena_init(SynqArena* a);
void synq_arena_free(SynqArena* a);

// Copy data into the arena and register in the offset table.
// Returns the node ID.
uint32_t synq_arena_alloc(SynqArena* a, const void* data, uint32_t size);

// Reserve a node ID in the offset table without allocating arena bytes.
// The offset is written later by synq_arena_commit.
uint32_t synq_arena_reserve_id(SynqArena* a);

// Commit data at a previously reserved node ID.
void synq_arena_commit(SynqArena* a, uint32_t node_id,
                       const void* data, uint32_t size);

// Append raw bytes to the arena without registering an offset entry.
void synq_arena_append(SynqArena* a, const void* data, uint32_t size);

#ifdef __cplusplus
}
#endif

#endif  // SYNQ_SRC_BASE_ARENA_H
