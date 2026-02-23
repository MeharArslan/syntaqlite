// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// SQLite compile-time flag constants for use with SyntaqliteDialectConfig.cflags.

#ifndef SYNTAQLITE_SQLITE_CFLAGS_H
#define SYNTAQLITE_SQLITE_CFLAGS_H

#define SYNQ_SQLITE_OMIT_EXPLAIN                    0x00000001
#define SYNQ_SQLITE_OMIT_TEMPDB                     0x00000002
#define SYNQ_SQLITE_OMIT_COMPOUND_SELECT            0x00000004
#define SYNQ_SQLITE_OMIT_WINDOWFUNC                 0x00000008
#define SYNQ_SQLITE_OMIT_GENERATED_COLUMNS          0x00000010
#define SYNQ_SQLITE_OMIT_VIEW                       0x00000020
#define SYNQ_SQLITE_OMIT_CTE                        0x00000040
#define SYNQ_SQLITE_OMIT_SUBQUERY                   0x00000080
#define SYNQ_SQLITE_OMIT_CAST                       0x00000100
#define SYNQ_SQLITE_OMIT_PRAGMA                     0x00000200
#define SYNQ_SQLITE_OMIT_TRIGGER                    0x00000400
#define SYNQ_SQLITE_OMIT_ATTACH                     0x00000800
#define SYNQ_SQLITE_OMIT_REINDEX                    0x00001000
#define SYNQ_SQLITE_OMIT_ANALYZE                    0x00002000
#define SYNQ_SQLITE_OMIT_ALTERTABLE                 0x00004000
#define SYNQ_SQLITE_OMIT_VIRTUALTABLE               0x00008000
#define SYNQ_SQLITE_OMIT_RETURNING                  0x00010000
#define SYNQ_SQLITE_ENABLE_ORDERED_SET_AGGREGATES   0x00020000

#endif  // SYNTAQLITE_SQLITE_CFLAGS_H
