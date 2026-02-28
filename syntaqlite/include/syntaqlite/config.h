// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Shared types for syntaqlite.
//
// SyntaqliteMemMethods lets callers override the allocator used for all
// internal allocations. Pass a pointer to one when creating a parser or
// tokenizer, or pass NULL to use the default (malloc/free).
//
// Usage:
//   // Pure defaults — no allocator needed:
//   SyntaqliteParser* p = syntaqlite_create_parser_with_dialect(NULL, syntaqlite_sqlite_dialect());
//
//   // Custom allocator:
//   SyntaqliteMemMethods mem = { my_malloc, my_free };
//   SyntaqliteParser* p = syntaqlite_create_parser_with_dialect(&mem, syntaqlite_sqlite_dialect());

#ifndef SYNTAQLITE_CONFIG_H
#define SYNTAQLITE_CONFIG_H

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

// ---------------------------------------------------------------------------
// Memory methods
// ---------------------------------------------------------------------------

// Allocator pair for all syntaqlite-internal allocations. Both function
// pointers must be non-NULL. Pass a pointer to one of these to
// syntaqlite_*_create() functions, or pass NULL for malloc/free.
typedef struct SyntaqliteMemMethods {
  void* (*xMalloc)(size_t);  // Allocate size bytes. Must not return NULL.
  void (*xFree)(void*);      // Free a pointer returned by xMalloc.
} SyntaqliteMemMethods;

// Default allocator (system malloc/free). Use this when you need an
// explicit allocator but have no custom one:
//   SyntaqliteParser* p =
//   syntaqlite_parser_create(&SYNTAQLITE_MEM_METHODS_DEFAULT);
extern void* malloc(size_t);
extern void free(void*);
#define SYNTAQLITE_MEM_METHODS_DEFAULT ((SyntaqliteMemMethods){malloc, free})

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_CONFIG_H
