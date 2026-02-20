// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#ifndef SYNTAQLITE_INTERNAL_SQLITE_TOKENIZE_H
#define SYNTAQLITE_INTERNAL_SQLITE_TOKENIZE_H

#include "csrc/sqlite_compat.h"

i64 synq_sqlite3GetToken(const unsigned char* z, int* tokenType);

#endif  // SYNTAQLITE_INTERNAL_SQLITE_TOKENIZE_H
