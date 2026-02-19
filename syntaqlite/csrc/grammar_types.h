// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Grammar-specific struct types for multi-valued grammar nonterminals.
// These are used by the Lemon-generated parser actions and are
// specific to the SQLite dialect.

#ifndef SYNQ_GRAMMAR_TYPES_H
#define SYNQ_GRAMMAR_TYPES_H

#include "syntaqlite/types.h"

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// columnname: passes name span + typetoken span from column definition.
typedef struct SynqColumnNameValue {
  SyntaqliteSourceSpan name;
  SyntaqliteSourceSpan typetoken;
} SynqColumnNameValue;

// ccons / tcons / generated: a constraint node + pending constraint name.
typedef struct SynqConstraintValue {
  uint32_t node;
  SyntaqliteSourceSpan pending_name;
} SynqConstraintValue;

// carglist / conslist: accumulated constraint list + pending name for next.
typedef struct SynqConstraintListValue {
  uint32_t list;
  SyntaqliteSourceSpan pending_name;
} SynqConstraintListValue;

// on_using: ON expr / USING column-list discriminator.
typedef struct SynqOnUsingValue {
  uint32_t on_expr;
  uint32_t using_cols;
} SynqOnUsingValue;

// with: recursive flag + CTE list node ID.
typedef struct SynqWithValue {
  uint32_t cte_list;
  int is_recursive;
} SynqWithValue;

#ifdef __cplusplus
}
#endif

#endif  // SYNQ_GRAMMAR_TYPES_H
