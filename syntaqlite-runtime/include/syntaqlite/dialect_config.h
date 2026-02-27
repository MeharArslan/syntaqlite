// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Dialect configuration for version/cflag-gated tokenization and parsing.

#ifndef SYNTAQLITE_DIALECT_CONFIG_H
#define SYNTAQLITE_DIALECT_CONFIG_H

#include <stdint.h>
#include "syntaqlite/sqlite_cflags.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct SyntaqliteDialectConfig {
    int32_t           sqlite_version;  // Target version (e.g., 3035000). INT32_MAX = latest.
    SyntaqliteCflags  cflags;          // Active compile-time flags.
} SyntaqliteDialectConfig;

// Default config: latest version, no cflags.
#define SYNQ_DIALECT_CONFIG_DEFAULT { INT32_MAX, SYNQ_CFLAGS_DEFAULT }

#ifdef __cplusplus
}
#endif

// ── Compile-time / runtime gating macros ────────────────────────────────
//
// When SYNQ_SQLITE_VERSION is defined (compile-time pinning), these expand
// to integer constants and the compiler eliminates dead branches.
// When not defined, they check through the runtime config pointer.

// True if the target version is older than `ver`.
#ifdef SYNQ_SQLITE_VERSION
  #define SYNQ_VER_LT(config, ver) (SYNQ_SQLITE_VERSION < (ver))
#else
  #define SYNQ_VER_LT(config, ver) ((config)->sqlite_version < (ver))
#endif

// True if cflag at index `idx` is set in the config.
#define SYNQ_HAS_CFLAG(config, idx) synq_has_cflag(&(config)->cflags, (idx))

#endif  // SYNTAQLITE_DIALECT_CONFIG_H
