// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

#ifndef SYNQ_CSRC_SQLITE_TOKENIZE_H
#define SYNQ_CSRC_SQLITE_TOKENIZE_H

#include "csrc/sqlite_compat.h"

i64 synq_sqlite3GetToken(const unsigned char* z, int* tokenType);

#endif  // SYNQ_CSRC_SQLITE_TOKENIZE_H
