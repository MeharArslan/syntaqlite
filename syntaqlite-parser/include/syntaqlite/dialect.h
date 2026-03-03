// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Unified dialect descriptor: parser vtable + AST metadata + formatter
// bytecode. A concrete dialect (e.g. SQLite) fills one static instance
// and exposes it via an entry-point function.
//
// Entry-point convention:
//   const SyntaqliteDialect* syntaqlite_<name>_dialect(void);
//
// ── Custom include ──────────────────────────────────────────────────────
//
// Define SYNTAQLITE_CUSTOM_INCLUDE to a filename to have it included before
// any macro decisions. This follows the SQLite SQLITE_CUSTOM_INCLUDE pattern.
//
//   cc -DSYNTAQLITE_CUSTOM_INCLUDE=synq_config.h -I. ...
//
// The config file can set SYNTAQLITE_SQLITE_VERSION, SYNTAQLITE_SQLITE_CFLAGS,
// and individual SYNTAQLITE_CFLAG_* defines.

#ifndef SYNTAQLITE_DIALECT_H
#define SYNTAQLITE_DIALECT_H

#ifdef SYNTAQLITE_CUSTOM_INCLUDE
#define SYNQ_STRINGIFY_(x) #x
#define SYNQ_STRINGIFY(x) SYNQ_STRINGIFY_(x)
#include SYNQ_STRINGIFY(SYNTAQLITE_CUSTOM_INCLUDE)
#endif

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#include "syntaqlite/cflags.h"

#ifdef __cplusplus
extern "C" {
#endif

// ── Forward-declare the dialect descriptor (full definition below) ──────

typedef struct SyntaqliteDialect SyntaqliteDialect;

// ── Configured dialect handle ────────────────────────────────────────────

typedef struct SyntaqliteDialectEnv {
  const SyntaqliteDialect*
      dialect;              // Grammar descriptor (must outlive the env).
  int32_t sqlite_version;   // Target version (e.g., 3035000). INT32_MAX =
                            // latest.
  SyntaqliteCflags cflags;  // Active compile-time flags.
} SyntaqliteDialectEnv;

// Default env: latest version, no cflags.
#define SYNQ_DIALECT_ENV_DEFAULT(d) {(d), INT32_MAX, SYNQ_CFLAGS_DEFAULT}

// ── Types used by the parser vtable ─────────────────────────────────────

// Forward-declared; full definition in csrc/parser.h.
typedef struct SynqParseCtx SynqParseCtx;

typedef struct SynqParseToken {
  const char* z;       // pointer to start of token in source text
  int n;               // length in bytes
  int type;            // token type ID (SYNTAQLITE_TK_*)
  uint32_t token_idx;  // index into parser's token vec (0xFFFFFFFF if not
                       // collecting)
} SynqParseToken;

// ── Forward declarations (full definitions in
// syntaqlite_dialect/dialect_types.h)

// Range metadata — used by pointer in SyntaqliteDialect.range_meta.
typedef struct SyntaqliteFieldRangeMeta SyntaqliteFieldRangeMeta;
typedef struct SyntaqliteRangeMetaEntry SyntaqliteRangeMetaEntry;

// Field metadata — used by pointer in SyntaqliteDialect.field_meta.
typedef struct SyntaqliteFieldMeta SyntaqliteFieldMeta;

// Schema contributions — used by pointer in
// SyntaqliteDialect.schema_contributions.
typedef struct SyntaqliteSchemaContribution SyntaqliteSchemaContribution;

// Function extensions — used by pointer in
// SyntaqliteDialect.function_extensions.
typedef struct SyntaqliteFunctionInfo SyntaqliteFunctionInfo;
typedef struct SyntaqliteAvailabilityRule SyntaqliteAvailabilityRule;
typedef struct SyntaqliteFunctionEntry SyntaqliteFunctionEntry;

// ── The dialect descriptor ──────────────────────────────────────────────

typedef struct SyntaqliteDialect {
  const char* name;

  // Range metadata for the macro straddle check.
  const SyntaqliteRangeMetaEntry* range_meta;

  // Well-known token IDs.
  int32_t tk_space;
  int32_t tk_semi;
  int32_t tk_comment;

  // AST metadata — all arrays indexed by node tag, length = node_count.
  uint32_t node_count;
  const char* const* node_names;
  const SyntaqliteFieldMeta* const* field_meta;
  const uint8_t* field_meta_counts;
  const uint8_t* list_tags;  // 1 = list node

  // Formatter data — all static arrays, NULL to skip formatting.
  const char* const*
      fmt_strings;  // keyword/punctuation strings (null-terminated)
  const uint16_t*
      fmt_string_lens;  // precomputed strlen for each fmt_strings entry
  uint16_t fmt_string_count;
  const uint16_t* fmt_enum_display;  // enum ordinal → string ID mapping
  uint16_t fmt_enum_display_count;
  const uint8_t*
      fmt_ops;  // packed 6-byte raw ops (opcode, a, b_lo, b_hi, c_lo, c_hi)
  uint16_t fmt_op_count;
  const uint32_t*
      fmt_dispatch;  // packed (u16 offset << 16 | u16 length) per node tag
  uint16_t fmt_dispatch_count;

  // Parser lifecycle (Lemon parser, provided by dialect)
  void* (*parser_alloc)(void* (*mallocProc)(size_t));
  void (*parser_init)(void* parser);
  void (*parser_finalize)(void* parser);
  void (*parser_free)(void* parser, void (*freeProc)(void*));
  void (*parser_feed)(void* parser,
                      int token_type,
                      SynqParseToken minor,
                      SynqParseCtx* pCtx);
  void (*parser_trace)(FILE* trace_file, char* prompt);
  int (*parser_expected_tokens)(void* parser, int* out_tokens, int out_cap);
  uint32_t (*parser_completion_context)(void* parser);

  // Tokenizer (provided by dialect)
  int64_t (*get_token)(const SyntaqliteDialectEnv* env,
                       const unsigned char* z,
                       int* tokenType);

  // Keyword table exported by mkkeywordhash output (`sqlite_keyword.c`).
  const char* keyword_text;         // concatenated keyword bytes
  const uint16_t* keyword_offsets;  // keyword_count entries
  const uint8_t* keyword_lens;      // keyword_count entries
  const uint8_t* keyword_codes;   // keyword_count entries (token type ordinals)
  const uint32_t* keyword_count;  // points to keyword count scalar

  // Token metadata (indexed by token type ordinal)
  const uint8_t*
      token_categories;  // length = token_type_count; NULL = no categories
  uint32_t token_type_count;

  // Dialect function extensions (additional functions beyond the SQLite base
  // catalog)
  const SyntaqliteFunctionEntry* function_extensions;
  uint32_t function_extension_count;

  // Schema contributions (nodes that define tables/views/functions)
  const SyntaqliteSchemaContribution* schema_contributions;
  uint32_t schema_contribution_count;
} SyntaqliteDialect;

#if UINTPTR_MAX == 0xFFFFFFFFu
_Static_assert(sizeof(SyntaqliteDialect) == 156,
               "SyntaqliteDialect size changed — update Rust mirror in "
               "dialect/ffi.rs");
#else
_Static_assert(sizeof(SyntaqliteDialect) == 296,
               "SyntaqliteDialect size changed — update Rust mirror in "
               "dialect/ffi.rs");
#endif

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_DIALECT_H
