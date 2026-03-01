// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Version-compatibility wrapper for the SQLite tokenizer.
//
// SynqSqliteGetTokenVersionWrapped reclassifies tokens that were
// introduced in newer SQLite versions, so the parser can target
// an older version of the grammar.

#ifndef SYNTAQLITE_INTERNAL_TOKEN_WRAPPED_H
#define SYNTAQLITE_INTERNAL_TOKEN_WRAPPED_H

#include <stdint.h>

#include "syntaqlite/dialect.h"

typedef struct SyntaqliteDialect SyntaqliteDialect;

int64_t SynqSqliteGetTokenVersionWrapped(const SyntaqliteDialect* d,
                                         const SyntaqliteDialectConfig* config,
                                         const unsigned char* z,
                                         int* tokenType);

#endif  // SYNTAQLITE_INTERNAL_TOKEN_WRAPPED_H
