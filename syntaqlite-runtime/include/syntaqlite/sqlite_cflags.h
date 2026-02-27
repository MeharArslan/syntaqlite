// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// SQLite compile-time flag constants for use with SyntaqliteDialectConfig.cflags.
//
// This is the union of all cflags that affect parser keywords, SQL functions,
// and virtual table modules. Each flag occupies a single bit.
//
// Sorted alphabetically within OMIT and ENABLE groups, bits assigned sequentially.

#ifndef SYNTAQLITE_SQLITE_CFLAGS_H
#define SYNTAQLITE_SQLITE_CFLAGS_H

#include <stdint.h>

// ── OMIT flags (bits 0–24) ──────────────────────────────────────────────
#define SYNQ_SQLITE_OMIT_ALTERTABLE                 ((uint64_t)0x0000000000000001)
#define SYNQ_SQLITE_OMIT_ANALYZE                     ((uint64_t)0x0000000000000002)
#define SYNQ_SQLITE_OMIT_ATTACH                      ((uint64_t)0x0000000000000004)
#define SYNQ_SQLITE_OMIT_AUTOINCREMENT               ((uint64_t)0x0000000000000008)
#define SYNQ_SQLITE_OMIT_CAST                        ((uint64_t)0x0000000000000010)
#define SYNQ_SQLITE_OMIT_COMPILEOPTION_DIAGS         ((uint64_t)0x0000000000000020)
#define SYNQ_SQLITE_OMIT_COMPOUND_SELECT             ((uint64_t)0x0000000000000040)
#define SYNQ_SQLITE_OMIT_CTE                         ((uint64_t)0x0000000000000080)
#define SYNQ_SQLITE_OMIT_DATETIME_FUNCS              ((uint64_t)0x0000000000000100)
#define SYNQ_SQLITE_OMIT_EXPLAIN                     ((uint64_t)0x0000000000000200)
#define SYNQ_SQLITE_OMIT_FLOATING_POINT              ((uint64_t)0x0000000000000400)
#define SYNQ_SQLITE_OMIT_FOREIGN_KEY                 ((uint64_t)0x0000000000000800)
#define SYNQ_SQLITE_OMIT_GENERATED_COLUMNS           ((uint64_t)0x0000000000001000)
#define SYNQ_SQLITE_OMIT_JSON                        ((uint64_t)0x0000000000002000)
#define SYNQ_SQLITE_OMIT_LOAD_EXTENSION              ((uint64_t)0x0000000000004000)
#define SYNQ_SQLITE_OMIT_PRAGMA                      ((uint64_t)0x0000000000008000)
#define SYNQ_SQLITE_OMIT_REINDEX                     ((uint64_t)0x0000000000010000)
#define SYNQ_SQLITE_OMIT_RETURNING                   ((uint64_t)0x0000000000020000)
#define SYNQ_SQLITE_OMIT_SUBQUERY                    ((uint64_t)0x0000000000040000)
#define SYNQ_SQLITE_OMIT_TEMPDB                      ((uint64_t)0x0000000000080000)
#define SYNQ_SQLITE_OMIT_TRIGGER                     ((uint64_t)0x0000000000100000)
#define SYNQ_SQLITE_OMIT_VACUUM                      ((uint64_t)0x0000000000200000)
#define SYNQ_SQLITE_OMIT_VIEW                        ((uint64_t)0x0000000000400000)
#define SYNQ_SQLITE_OMIT_VIRTUALTABLE                ((uint64_t)0x0000000000800000)
#define SYNQ_SQLITE_OMIT_WINDOWFUNC                  ((uint64_t)0x0000000001000000)

// ── ENABLE / misc flags (bits 25–41) ────────────────────────────────────
#define SYNQ_SQLITE_ENABLE_BYTECODE_VTAB             ((uint64_t)0x0000000002000000)
#define SYNQ_SQLITE_ENABLE_CARRAY                    ((uint64_t)0x0000000004000000)
#define SYNQ_SQLITE_ENABLE_DBPAGE_VTAB               ((uint64_t)0x0000000008000000)
#define SYNQ_SQLITE_ENABLE_DBSTAT_VTAB               ((uint64_t)0x0000000010000000)
#define SYNQ_SQLITE_ENABLE_FTS3                      ((uint64_t)0x0000000020000000)
#define SYNQ_SQLITE_ENABLE_FTS4                      ((uint64_t)0x0000000040000000)
#define SYNQ_SQLITE_ENABLE_FTS5                      ((uint64_t)0x0000000080000000)
#define SYNQ_SQLITE_ENABLE_GEOPOLY                   ((uint64_t)0x0000000100000000)
#define SYNQ_SQLITE_ENABLE_JSON1                     ((uint64_t)0x0000000200000000)
#define SYNQ_SQLITE_ENABLE_MATH_FUNCTIONS            ((uint64_t)0x0000000400000000)
#define SYNQ_SQLITE_ENABLE_OFFSET_SQL_FUNC           ((uint64_t)0x0000000800000000)
#define SYNQ_SQLITE_ENABLE_ORDERED_SET_AGGREGATES    ((uint64_t)0x0000001000000000)
#define SYNQ_SQLITE_ENABLE_PERCENTILE                ((uint64_t)0x0000002000000000)
#define SYNQ_SQLITE_ENABLE_RTREE                     ((uint64_t)0x0000004000000000)
#define SYNQ_SQLITE_ENABLE_STMTVTAB                  ((uint64_t)0x0000008000000000)
#define SYNQ_SQLITE_ENABLE_UPDATE_DELETE_LIMIT        ((uint64_t)0x0000010000000000)
#define SYNQ_SQLITE_SOUNDEX                          ((uint64_t)0x0000020000000000)

#endif  // SYNTAQLITE_SQLITE_CFLAGS_H
