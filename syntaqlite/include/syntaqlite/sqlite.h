// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Public API for the SQLite dialect.

#ifndef SYNTAQLITE_SQLITE_H
#define SYNTAQLITE_SQLITE_H

#include "syntaqlite/config.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SyntaqliteDialect SyntaqliteDialect;
typedef struct SyntaqliteParser SyntaqliteParser;

const SyntaqliteDialect* syntaqlite_sqlite_dialect(void);
SyntaqliteParser* syntaqlite_create_parser(const SyntaqliteMemMethods* mem);

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_SQLITE_H
