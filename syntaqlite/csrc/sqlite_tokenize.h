// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#ifndef SYNTAQLITE_INTERNAL_SQLITE_TOKENIZE_H
#define SYNTAQLITE_INTERNAL_SQLITE_TOKENIZE_H

#include "syntaqlite_ext/sqlite_compat.h"
#include "syntaqlite/dialect_config.h"

i64 SynqSqliteGetToken_base(const SyntaqliteDialectConfig* config, const unsigned char* z, int* tokenType);
i64 SynqSqliteGetToken(const SyntaqliteDialectConfig* config, const unsigned char* z, int* tokenType);

#endif  // SYNTAQLITE_INTERNAL_SQLITE_TOKENIZE_H
