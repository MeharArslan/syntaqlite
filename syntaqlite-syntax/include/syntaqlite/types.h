// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Core types shared between the engine and dialect layers.

#ifndef SYNTAQLITE_TYPES_H
#define SYNTAQLITE_TYPES_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define SYNTAQLITE_NULL_NODE 0xFFFFFFFFu

typedef uint32_t SyntaqliteCompletionContext;
#define SYNTAQLITE_COMPLETION_CONTEXT_UNKNOWN    \
  ((SyntaqliteCompletionContext)0)
#define SYNTAQLITE_COMPLETION_CONTEXT_EXPRESSION \
  ((SyntaqliteCompletionContext)1)
#define SYNTAQLITE_COMPLETION_CONTEXT_TABLE_REF  \
  ((SyntaqliteCompletionContext)2)

typedef struct SyntaqliteSourceSpan {
  uint32_t offset;
  uint16_t length;
} SyntaqliteSourceSpan;

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_TYPES_H
