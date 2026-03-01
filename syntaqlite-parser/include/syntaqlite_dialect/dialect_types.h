// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Dialect-implementation types: full definitions of structs that appear in
// SyntaqliteDialect by pointer and are only needed when building a dialect
// descriptor. Consumer code (code that merely *uses* a dialect) needs only the
// forward declarations in syntaqlite/dialect.h.

#ifndef SYNTAQLITE_DIALECT_TYPES_H
#define SYNTAQLITE_DIALECT_TYPES_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// ── Range metadata ───────────────────────────────────────────────────────────

typedef struct SyntaqliteFieldRangeMeta {
  uint16_t offset;
  uint8_t kind;
} SyntaqliteFieldRangeMeta;

typedef struct SyntaqliteRangeMetaEntry {
  const SyntaqliteFieldRangeMeta* fields;
  uint8_t count;
} SyntaqliteRangeMetaEntry;

// ── Field metadata (for AST dump / dynamic dialect loading) ──────────────────

#define SYNTAQLITE_FIELD_NODE_ID 0
#define SYNTAQLITE_FIELD_SPAN 1
#define SYNTAQLITE_FIELD_BOOL 2
#define SYNTAQLITE_FIELD_FLAGS 3
#define SYNTAQLITE_FIELD_ENUM 4

typedef struct SyntaqliteFieldMeta {
  uint16_t offset;             // byte offset in node struct
  uint8_t kind;                // SYNTAQLITE_FIELD_*
  const char* name;            // field name for AST dump
  const char* const* display;  // enum: indexed by ordinal; flags: indexed by
                               // bit pos; else NULL
  uint8_t display_count;       // number of entries in display[]
} SyntaqliteFieldMeta;

// ── Schema contribution types ─────────────────────────────────────────────────

#define SYNTAQLITE_SCHEMA_TABLE 0
#define SYNTAQLITE_SCHEMA_VIEW 1
#define SYNTAQLITE_SCHEMA_FUNCTION 2
#define SYNTAQLITE_SCHEMA_IMPORT 3

typedef struct SyntaqliteSchemaContribution {
  uint32_t node_tag;
  uint8_t kind;        // SYNTAQLITE_SCHEMA_*
  uint8_t name_field;  // field index -> must be SPAN
  uint8_t
      columns_field;  // field index -> NODE_ID to column list (0xFF = absent)
  uint8_t select_field;  // field index -> NODE_ID to Select (0xFF = absent)
  uint8_t args_field;    // field index -> NODE_ID to args list (0xFF = absent)
  uint8_t _pad[3];
} SyntaqliteSchemaContribution;

// ── Function extension types ──────────────────────────────────────────────────

typedef struct SyntaqliteFunctionInfo {
  const char* name;
  const int16_t* arities;
  uint16_t arity_count;
  uint8_t category;  // 0=Scalar, 1=Aggregate, 2=Window
} SyntaqliteFunctionInfo;

typedef struct SyntaqliteAvailabilityRule {
  int32_t since;
  int32_t until;
  uint32_t cflag_index;
  uint8_t cflag_polarity;  // 0=Enable, 1=Omit
} SyntaqliteAvailabilityRule;

typedef struct SyntaqliteFunctionEntry {
  SyntaqliteFunctionInfo info;
  const SyntaqliteAvailabilityRule* availability;
  uint16_t availability_count;
} SyntaqliteFunctionEntry;

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_DIALECT_TYPES_H
