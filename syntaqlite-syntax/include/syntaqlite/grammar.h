// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Grammar descriptor: parser vtable + AST metadata. A concrete grammar
// (e.g. SQLite) fills one static instance and exposes it via an entry-point
// function.
//
// Entry-point convention:
//   const SyntaqliteGrammarTemplate* syntaqlite_<name>_grammar(void);
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

#ifndef SYNTAQLITE_GRAMMAR_H
#define SYNTAQLITE_GRAMMAR_H

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

// ── Types used by the parser vtable ─────────────────────────────────────
typedef struct SynqParseCtx SynqParseCtx;

typedef struct SynqParseToken {
  const char* z;       // pointer to start of token in source text
  int n;               // length in bytes
  int type;            // token type ID (SYNTAQLITE_TK_*)
  uint32_t token_idx;  // index into parser's token vec (0xFFFFFFFF if not
                       // collecting)
} SynqParseToken;

typedef struct SyntaqliteFieldRangeMeta SyntaqliteFieldRangeMeta;
typedef struct SyntaqliteRangeMetaEntry SyntaqliteRangeMetaEntry;
typedef struct SyntaqliteFieldMeta SyntaqliteFieldMeta;
typedef struct SyntaqliteGrammar SyntaqliteGrammar;

typedef struct SyntaqliteGrammarTemplate {
  const char* name;

  // Range metadata for the macro straddle check.
  const SyntaqliteRangeMetaEntry* range_meta;

  // AST metadata — all arrays indexed by node tag, length = node_count.
  uint32_t node_count;
  const char* const* node_names;
  const SyntaqliteFieldMeta* const* field_meta;
  const uint8_t* field_meta_counts;
  const uint8_t* list_tags;  // 1 = list node

  // Parser lifecycle (Lemon parser, provided by grammar)
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

  // Tokenizer (provided by grammar)
  int64_t (*get_token)(const SyntaqliteGrammar* env,
                       const unsigned char* z,
                       int* tokenType);

  // Keyword table exported by mkkeywordhash output (`sqlite_keyword.c`).
  const char* keyword_text;         // concatenated keyword bytes
  const uint16_t* keyword_offsets;  // keyword_count entries
  const uint8_t* keyword_lens;      // keyword_count entries
  const uint8_t* keyword_codes;   // keyword_count entries (token type ordinals)
  const uint32_t* keyword_count;  // points to keyword count scalar

  // Token metadata (indexed by token type ordinal)
  // length = token_type_count; NULL = no categories
  const uint8_t* token_categories;
  uint32_t token_type_count;
} SyntaqliteGrammarTemplate;

// ── Configured grammar handle ─────────────────────────────────────────────

typedef struct SyntaqliteGrammar {
  const SyntaqliteGrammarTemplate* tmpl;
  int32_t sqlite_version;   // Target version (e.g., 3035000). INT32_MAX =
                            // latest.
  SyntaqliteCflags cflags;  // Active compile-time flags.
} SyntaqliteGrammar;

// Default env: latest version, no cflags.
#define SYNQ_GRAMMAR_DEFAULT(g) {(g), INT32_MAX, SYNQ_CFLAGS_DEFAULT}

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_GRAMMAR_H
