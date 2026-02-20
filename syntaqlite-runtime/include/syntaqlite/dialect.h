// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Unified dialect descriptor: parser vtable + AST metadata + formatter
// bytecode. A concrete dialect (e.g. SQLite) fills one static instance
// and exposes it via an entry-point function.
//
// Entry-point convention:
//   const SyntaqliteDialect* syntaqlite_<name>_dialect(void);

#ifndef SYNTAQLITE_DIALECT_H
#define SYNTAQLITE_DIALECT_H

#include <stddef.h>
#include <stdint.h>
#include <stdio.h>

#ifdef __cplusplus
extern "C" {
#endif

// ── Types used by the parser vtable ─────────────────────────────────────

// Forward-declared; full definition in csrc/parser.h.
typedef struct SynqParseCtx SynqParseCtx;

typedef struct SynqParseToken {
    const char* z;   // pointer to start of token in source text
    int n;           // length in bytes
    int type;        // token type ID (SYNTAQLITE_TK_*)
} SynqParseToken;

// ── Parse tables (Lemon data) ───────────────────────────────────────────

typedef struct SynqParseTables {
    const unsigned short *yy_action;
    const unsigned short *yy_lookahead;
    const unsigned short *yy_shift_ofst;
    const short *yy_reduce_ofst;
    const unsigned short *yy_default;
    const unsigned short *yy_fallback;     // NULL if no fallback
    const unsigned short *yy_rule_lhs;
    const signed char *yy_rule_nrhs;

    int n_action;
    int n_lookahead;
    int n_fallback;

    unsigned short nocode;
    unsigned short wildcard;            // 0 if none
    unsigned short nstate;
    unsigned short nrule;
    unsigned short nrule_with_action;
    unsigned short ntoken;
    unsigned short max_shift;
    unsigned short min_shiftreduce;
    unsigned short max_shiftreduce;
    unsigned short error_action;
    unsigned short accept_action;
    unsigned short no_action;
    unsigned short min_reduce;
    unsigned short max_reduce;
    unsigned short acttab_count;
    unsigned short shift_count;
    unsigned short reduce_count;

    const char *const *token_names;     // NULL in release
    const char *const *rule_names;      // NULL in release
} SynqParseTables;

typedef void (*SynqReduceActionsFn)(
    void *parser,              // yyParser*
    unsigned int ruleno,
    void *yymsp,               // yyStackEntry*
    int lookahead,
    SynqParseToken lookahead_token
);

typedef struct SyntaqliteFieldRangeMeta {
    uint16_t offset;
    uint8_t kind;
} SyntaqliteFieldRangeMeta;

typedef struct SyntaqliteRangeMetaEntry {
    const SyntaqliteFieldRangeMeta* fields;
    uint8_t count;
} SyntaqliteRangeMetaEntry;

// ── Field metadata (for AST dump / dynamic dialect loading) ─────────────

#define SYNTAQLITE_FIELD_NODE_ID  0
#define SYNTAQLITE_FIELD_SPAN     1
#define SYNTAQLITE_FIELD_BOOL     2
#define SYNTAQLITE_FIELD_FLAGS    3
#define SYNTAQLITE_FIELD_ENUM     4

typedef struct SyntaqliteFieldMeta {
    uint16_t    offset;           // byte offset in node struct
    uint8_t     kind;             // SYNTAQLITE_FIELD_*
    const char* name;             // field name for AST dump
    const char* const* display;   // enum: indexed by ordinal; flags: indexed by bit pos; else NULL
    uint8_t     display_count;    // number of entries in display[]
} SyntaqliteFieldMeta;

// ── The dialect descriptor ──────────────────────────────────────────────

typedef struct SyntaqliteDialect {
    const char* name;

    // Parse tables + reduce actions (replaces Lemon vtable).
    const SynqParseTables *tables;
    SynqReduceActionsFn reduce_actions;

    // Range metadata for the macro straddle check.
    const SyntaqliteRangeMetaEntry* range_meta;

    // Well-known token IDs.
    int32_t tk_space;
    int32_t tk_semi;
    int32_t tk_comment;

    // AST metadata — all arrays indexed by node tag, length = node_count.
    uint32_t                              node_count;
    const char* const*                    node_names;
    const SyntaqliteFieldMeta* const*     field_meta;
    const uint8_t*                        field_meta_counts;
    const uint8_t*                        list_tags;         // 1 = list node

    // Formatter data — all static arrays, NULL to skip formatting.
    const char* const*    fmt_strings;           // keyword/punctuation strings (null-terminated)
    uint16_t              fmt_string_count;
    const uint16_t*       fmt_enum_display;      // enum ordinal → string ID mapping
    uint16_t              fmt_enum_display_count;
    const uint8_t*        fmt_ops;               // packed 6-byte raw ops (opcode, a, b_lo, b_hi, c_lo, c_hi)
    uint16_t              fmt_op_count;
    const uint32_t*       fmt_dispatch;          // packed (u16 offset << 16 | u16 length) per node tag
    uint16_t              fmt_dispatch_count;
} SyntaqliteDialect;

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_DIALECT_H
