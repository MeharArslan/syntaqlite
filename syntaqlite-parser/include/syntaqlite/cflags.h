// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// SQLite compile-time flag constants for use with
// SyntaqliteDialectEnv.cflags.
//
// This is the union of all cflags that affect parser keywords, SQL functions,
// and virtual table modules.
//
// Sorted alphabetically within OMIT and ENABLE groups.

#ifndef SYNTAQLITE_SQLITE_CFLAGS_H
#define SYNTAQLITE_SQLITE_CFLAGS_H

#include <stdint.h>
#include <string.h>

// ── Cflag index constants ───────────────────────────────────────────────
//
// These are used for dynamic lookup (e.g. in keyword/function tables).

// OMIT flags (indices 0–24):
#define SYNQ_CFLAG_IDX_OMIT_ALTERTABLE 0
#define SYNQ_CFLAG_IDX_OMIT_ANALYZE 1
#define SYNQ_CFLAG_IDX_OMIT_ATTACH 2
#define SYNQ_CFLAG_IDX_OMIT_AUTOINCREMENT 3
#define SYNQ_CFLAG_IDX_OMIT_CAST 4
#define SYNQ_CFLAG_IDX_OMIT_COMPILEOPTION_DIAGS 5
#define SYNQ_CFLAG_IDX_OMIT_COMPOUND_SELECT 6
#define SYNQ_CFLAG_IDX_OMIT_CTE 7
#define SYNQ_CFLAG_IDX_OMIT_DATETIME_FUNCS 8
#define SYNQ_CFLAG_IDX_OMIT_EXPLAIN 9
#define SYNQ_CFLAG_IDX_OMIT_FLOATING_POINT 10
#define SYNQ_CFLAG_IDX_OMIT_FOREIGN_KEY 11
#define SYNQ_CFLAG_IDX_OMIT_GENERATED_COLUMNS 12
#define SYNQ_CFLAG_IDX_OMIT_JSON 13
#define SYNQ_CFLAG_IDX_OMIT_LOAD_EXTENSION 14
#define SYNQ_CFLAG_IDX_OMIT_PRAGMA 15
#define SYNQ_CFLAG_IDX_OMIT_REINDEX 16
#define SYNQ_CFLAG_IDX_OMIT_RETURNING 17
#define SYNQ_CFLAG_IDX_OMIT_SUBQUERY 18
#define SYNQ_CFLAG_IDX_OMIT_TEMPDB 19
#define SYNQ_CFLAG_IDX_OMIT_TRIGGER 20
#define SYNQ_CFLAG_IDX_OMIT_VACUUM 21
#define SYNQ_CFLAG_IDX_OMIT_VIEW 22
#define SYNQ_CFLAG_IDX_OMIT_VIRTUALTABLE 23
#define SYNQ_CFLAG_IDX_OMIT_WINDOWFUNC 24
// ENABLE / misc flags (indices 25–41):
#define SYNQ_CFLAG_IDX_ENABLE_BYTECODE_VTAB 25
#define SYNQ_CFLAG_IDX_ENABLE_CARRAY 26
#define SYNQ_CFLAG_IDX_ENABLE_DBPAGE_VTAB 27
#define SYNQ_CFLAG_IDX_ENABLE_DBSTAT_VTAB 28
#define SYNQ_CFLAG_IDX_ENABLE_FTS3 29
#define SYNQ_CFLAG_IDX_ENABLE_FTS4 30
#define SYNQ_CFLAG_IDX_ENABLE_FTS5 31
#define SYNQ_CFLAG_IDX_ENABLE_GEOPOLY 32
#define SYNQ_CFLAG_IDX_ENABLE_JSON1 33
#define SYNQ_CFLAG_IDX_ENABLE_MATH_FUNCTIONS 34
#define SYNQ_CFLAG_IDX_ENABLE_OFFSET_SQL_FUNC 35
#define SYNQ_CFLAG_IDX_ENABLE_ORDERED_SET_AGGREGATES 36
#define SYNQ_CFLAG_IDX_ENABLE_PERCENTILE 37
#define SYNQ_CFLAG_IDX_ENABLE_RTREE 38
#define SYNQ_CFLAG_IDX_ENABLE_STMTVTAB 39
#define SYNQ_CFLAG_IDX_ENABLE_UPDATE_DELETE_LIMIT 40
#define SYNQ_CFLAG_IDX_SOUNDEX 41

#define SYNQ_CFLAG_IDX_COUNT 42

// ── Named bitfield struct ───────────────────────────────────────────────

typedef struct SyntaqliteCflags {
  // OMIT flags:
  uint8_t omit_altertable : 1;
  uint8_t omit_analyze : 1;
  uint8_t omit_attach : 1;
  uint8_t omit_autoincrement : 1;
  uint8_t omit_cast : 1;
  uint8_t omit_compileoption_diags : 1;
  uint8_t omit_compound_select : 1;
  uint8_t omit_cte : 1;
  uint8_t omit_datetime_funcs : 1;
  uint8_t omit_explain : 1;
  uint8_t omit_floating_point : 1;
  uint8_t omit_foreign_key : 1;
  uint8_t omit_generated_columns : 1;
  uint8_t omit_json : 1;
  uint8_t omit_load_extension : 1;
  uint8_t omit_pragma : 1;
  uint8_t omit_reindex : 1;
  uint8_t omit_returning : 1;
  uint8_t omit_subquery : 1;
  uint8_t omit_tempdb : 1;
  uint8_t omit_trigger : 1;
  uint8_t omit_vacuum : 1;
  uint8_t omit_view : 1;
  uint8_t omit_virtualtable : 1;
  uint8_t omit_windowfunc : 1;
  // ENABLE / misc flags:
  uint8_t enable_bytecode_vtab : 1;
  uint8_t enable_carray : 1;
  uint8_t enable_dbpage_vtab : 1;
  uint8_t enable_dbstat_vtab : 1;
  uint8_t enable_fts3 : 1;
  uint8_t enable_fts4 : 1;
  uint8_t enable_fts5 : 1;
  uint8_t enable_geopoly : 1;
  uint8_t enable_json1 : 1;
  uint8_t enable_math_functions : 1;
  uint8_t enable_offset_sql_func : 1;
  uint8_t enable_ordered_set_aggregates : 1;
  uint8_t enable_percentile : 1;
  uint8_t enable_rtree : 1;
  uint8_t enable_stmtvtab : 1;
  uint8_t enable_update_delete_limit : 1;
  uint8_t soundex : 1;
  // Padding to 48 bits (6 bytes):
  uint8_t _reserved : 5;
} SyntaqliteCflags;

#define SYNQ_CFLAGS_DEFAULT {0}

// ── Indexed accessor ────────────────────────────────────────────────────
//
// For dynamic cflag lookup (keyword tables etc.). Uses the index constants
// above. Implementation: bit ops on raw bytes — field declaration order
// matches index order, verified by static assert in tests.

static inline int synq_has_cflag(const SyntaqliteCflags* c, int idx) {
  const uint8_t* bytes = (const uint8_t*)c;
  return (bytes[idx / 8] >> (idx % 8)) & 1;
}

static inline void synq_set_cflag(SyntaqliteCflags* c, int idx) {
  uint8_t* bytes = (uint8_t*)c;
  bytes[idx / 8] |= (uint8_t)(1u << (idx % 8));
}

// ── Compile-time cflag pinning ──────────────────────────────────────────
//
// When SYNTAQLITE_SQLITE_CFLAGS is defined, a static const struct is built
// from individual SYNTAQLITE_CFLAG_* defines. This lets SYNQ_HAS_CFLAG()
// become a compile-time constant, enabling dead-branch elimination.
//
// Usage (command line):
//   cc -DSYNTAQLITE_SQLITE_CFLAGS -DSYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC ...
//
// Usage (config file via SYNTAQLITE_CUSTOM_INCLUDE):
//   #define SYNTAQLITE_SQLITE_CFLAGS
//   #define SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC

#ifdef SYNTAQLITE_SQLITE_CFLAGS
static const SyntaqliteCflags synq_pinned_cflags = {
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_ALTERTABLE
    .omit_altertable = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_ANALYZE
    .omit_analyze = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_ATTACH
    .omit_attach = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_AUTOINCREMENT
    .omit_autoincrement = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_CAST
    .omit_cast = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_COMPILEOPTION_DIAGS
    .omit_compileoption_diags = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_COMPOUND_SELECT
    .omit_compound_select = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_CTE
    .omit_cte = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_DATETIME_FUNCS
    .omit_datetime_funcs = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_EXPLAIN
    .omit_explain = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_FLOATING_POINT
    .omit_floating_point = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_FOREIGN_KEY
    .omit_foreign_key = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_GENERATED_COLUMNS
    .omit_generated_columns = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_JSON
    .omit_json = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_LOAD_EXTENSION
    .omit_load_extension = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_PRAGMA
    .omit_pragma = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_REINDEX
    .omit_reindex = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_RETURNING
    .omit_returning = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_SUBQUERY
    .omit_subquery = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_TEMPDB
    .omit_tempdb = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_TRIGGER
    .omit_trigger = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_VACUUM
    .omit_vacuum = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_VIEW
    .omit_view = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_VIRTUALTABLE
    .omit_virtualtable = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_OMIT_WINDOWFUNC
    .omit_windowfunc = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_BYTECODE_VTAB
    .enable_bytecode_vtab = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_CARRAY
    .enable_carray = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_DBPAGE_VTAB
    .enable_dbpage_vtab = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_DBSTAT_VTAB
    .enable_dbstat_vtab = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_FTS3
    .enable_fts3 = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_FTS4
    .enable_fts4 = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_FTS5
    .enable_fts5 = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_GEOPOLY
    .enable_geopoly = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_JSON1
    .enable_json1 = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_MATH_FUNCTIONS
    .enable_math_functions = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_OFFSET_SQL_FUNC
    .enable_offset_sql_func = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_ORDERED_SET_AGGREGATES
    .enable_ordered_set_aggregates = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_PERCENTILE
    .enable_percentile = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_RTREE
    .enable_rtree = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_STMTVTAB
    .enable_stmtvtab = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_ENABLE_UPDATE_DELETE_LIMIT
    .enable_update_delete_limit = 1,
#endif
#ifdef SYNTAQLITE_CFLAG_SQLITE_SOUNDEX
    .soundex = 1,
#endif
};
#endif  // SYNTAQLITE_SQLITE_CFLAGS

#endif  // SYNTAQLITE_SQLITE_CFLAGS_H
