// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Version-dependent token reclassification wrapper.
//
// This wraps SynqSqliteGetToken_base (generated from SQLite's sqlite3GetToken)
// and reclassifies tokens that didn't exist in older SQLite versions.
//
// When SQLite adds new token types in future versions, add a new version
// gate here following the existing pattern.

#include "csrc/sqlite/sqlite_tokenize.h"
#include "syntaqlite_sqlite/sqlite_tokens.h"

i64 SynqSqliteGetToken(const SyntaqliteDialectConfig* config,
                       const unsigned char* z,
                       int* tokenType) {
  i64 len = SynqSqliteGetToken_base(config, z, tokenType);

  if (SYNQ_VER_LT(config, 3038000) && *tokenType == SYNTAQLITE_TK_PTR) {
    /* -> and ->> operators added in 3.38.
    ** Return just the '-' as TK_MINUS; next call picks up '>' naturally. */
    *tokenType = SYNTAQLITE_TK_MINUS;
    return 1;
  }

  if (SYNQ_VER_LT(config, 3046000) && *tokenType == SYNTAQLITE_TK_QNUMBER) {
    /* Digit separators added in 3.46.
    ** Truncate to the first underscore. */
    i64 j;
    int saw_dot = 0;
    for (j = 0; j < len; j++) {
      if (z[j] == '_')
        break;
      if (z[j] == '.')
        saw_dot = 1;
    }
    *tokenType = saw_dot ? SYNTAQLITE_TK_FLOAT : SYNTAQLITE_TK_INTEGER;
    return j;
  }

  return len;
}
