// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Header-only open-addressing hashmap for the macro registry.
// Uses FNV-1a hashing and linear probing with tombstone deletion.
//
// Each entry has a `uint8_t state` field:
//   0 = empty, 1 = live, 2 = tombstone.
//
// The table size is always a power of two.

#ifndef SYNTAQLITE_INTERNAL_HASHMAP_H
#define SYNTAQLITE_INTERNAL_HASHMAP_H

#include <stdint.h>
#include <string.h>

#include "syntaqlite/config.h"

// Entry states.
#define SYNQ_MAP_EMPTY     0
#define SYNQ_MAP_LIVE      1
#define SYNQ_MAP_TOMBSTONE 2

// Case-insensitive FNV-1a hash.
static inline uint32_t synq_hash_ci(const char* data, uint32_t len) {
  uint32_t h = 2166136261u;
  for (uint32_t i = 0; i < len; i++) {
    uint8_t c = (uint8_t)data[i];
    if (c >= 'A' && c <= 'Z') c += 32;
    h ^= c;
    h *= 16777619u;
  }
  return h;
}

// Case-insensitive name comparison.
static inline int synq_name_eq_ci(const char* a, uint32_t alen,
                                   const char* b, uint32_t blen) {
  if (alen != blen) return 0;
  for (uint32_t i = 0; i < alen; i++) {
    uint8_t ca = (uint8_t)a[i], cb = (uint8_t)b[i];
    if (ca >= 'A' && ca <= 'Z') ca += 32;
    if (cb >= 'A' && cb <= 'Z') cb += 32;
    if (ca != cb) return 0;
  }
  return 1;
}

// ---------------------------------------------------------------------------
// Generic hashmap operations via macros.
//
// The entry type must have:
//   char*    name;
//   uint32_t name_len;
//   uint8_t  state;      // SYNQ_MAP_EMPTY / LIVE / TOMBSTONE
//
// Usage:
//   SYNQ_MAP_FIND(table, table_size, name, name_len, EntryType, result)
//     Sets `result` to the matching live entry pointer, or NULL.
//
//   SYNQ_MAP_INSERT(table, table_size, table_count, name, name_len,
//                   EntryType, mem, initial_cap, result)
//     Grows if needed and sets `result` to the slot to fill.
//     If the name already exists, returns the existing live entry.
// ---------------------------------------------------------------------------

// Find a live entry by name.  Sets `out` to the entry or NULL.
#define SYNQ_MAP_FIND(tbl, tbl_size, key, key_len, out)                    \
  do {                                                                     \
    (out) = NULL;                                                          \
    if ((tbl) && (tbl_size) > 0) {                                         \
      uint32_t _mask = (tbl_size) - 1;                                     \
      uint32_t _idx = synq_hash_ci((key), (key_len)) & _mask;             \
      for (uint32_t _i = 0; _i < (tbl_size); _i++) {                      \
        __typeof__(&(tbl)[0]) _e = &(tbl)[(_idx + _i) & _mask];           \
        if (_e->state == SYNQ_MAP_EMPTY) break;                            \
        if (_e->state == SYNQ_MAP_LIVE &&                                  \
            synq_name_eq_ci(_e->name, _e->name_len, (key), (key_len))) {   \
          (out) = _e;                                                      \
          break;                                                           \
        }                                                                  \
      }                                                                    \
    }                                                                      \
  } while (0)

// Grow + reinsert helper (used internally by SYNQ_MAP_INSERT).
#define SYNQ_MAP_GROW(tbl, tbl_size, tbl_count, mem, init_cap)             \
  do {                                                                     \
    uint32_t _old_sz = (tbl_size);                                         \
    __typeof__((tbl)) _old = (tbl);                                        \
    uint32_t _new_sz = _old_sz ? _old_sz * 2 : (init_cap);                \
    (tbl) = (__typeof__((tbl)))(mem).xMalloc(_new_sz * sizeof(*(tbl)));    \
    memset((tbl), 0, _new_sz * sizeof(*(tbl)));                            \
    (tbl_size) = _new_sz;                                                  \
    (tbl_count) = 0;                                                       \
    if (_old) {                                                            \
      uint32_t _m2 = _new_sz - 1;                                         \
      for (uint32_t _j = 0; _j < _old_sz; _j++) {                         \
        if (_old[_j].state != SYNQ_MAP_LIVE) continue;                     \
        uint32_t _k = synq_hash_ci(_old[_j].name, _old[_j].name_len) & _m2; \
        while ((tbl)[_k].state == SYNQ_MAP_LIVE) _k = (_k + 1) & _m2;     \
        (tbl)[_k] = _old[_j];                                             \
        (tbl_count)++;                                                     \
      }                                                                    \
      (mem).xFree(_old);                                                   \
    }                                                                      \
  } while (0)

// Insert or find an existing entry.  Sets `out` to the slot.
// If the name already exists as LIVE, `out` points to that entry (no dup).
// Otherwise `out` points to the new empty/tombstone slot with state = LIVE.
#define SYNQ_MAP_INSERT(tbl, tbl_size, tbl_count, key, key_len,            \
                        mem, init_cap, out)                                 \
  do {                                                                     \
    if ((tbl_size) == 0 ||                                                 \
        ((tbl_count) + 1) * 10 > (tbl_size) * 7) {                        \
      SYNQ_MAP_GROW(tbl, tbl_size, tbl_count, mem, init_cap);             \
    }                                                                      \
    uint32_t _mask = (tbl_size) - 1;                                       \
    uint32_t _idx = synq_hash_ci((key), (key_len)) & _mask;               \
    __typeof__(&(tbl)[0]) _tomb = NULL;                                    \
    (out) = NULL;                                                          \
    for (;;) {                                                             \
      __typeof__(&(tbl)[0]) _e = &(tbl)[_idx];                            \
      if (_e->state == SYNQ_MAP_EMPTY) {                                   \
        (out) = _tomb ? _tomb : _e;                                        \
        (out)->state = SYNQ_MAP_LIVE;                                      \
        (tbl_count)++;                                                     \
        break;                                                             \
      }                                                                    \
      if (_e->state == SYNQ_MAP_TOMBSTONE && !_tomb) _tomb = _e;           \
      if (_e->state == SYNQ_MAP_LIVE &&                                    \
          synq_name_eq_ci(_e->name, _e->name_len, (key), (key_len))) {     \
        (out) = _e;                                                        \
        break;                                                             \
      }                                                                    \
      _idx = (_idx + 1) & _mask;                                           \
    }                                                                      \
  } while (0)

#endif  // SYNTAQLITE_INTERNAL_HASHMAP_H
