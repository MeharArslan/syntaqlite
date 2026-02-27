// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Dialect configuration for version/cflag-gated tokenization and parsing.
//
// ── Custom include ──────────────────────────────────────────────────────
//
// Define SYNTAQLITE_CUSTOM_INCLUDE to a filename to have it included before
// any macro decisions. This follows the SQLite SQLITE_CUSTOM_INCLUDE pattern.
//
//   cc -DSYNTAQLITE_CUSTOM_INCLUDE=synq_config.h -I. ...
//
// The config file can set SYNTAQLITE_SQLITE_VERSION, SYNTAQLITE_SQLITE_CFLAGS,
// and individual SYNTAQLITE_CFLAG_* defines.

#ifndef SYNTAQLITE_DIALECT_CONFIG_H
#define SYNTAQLITE_DIALECT_CONFIG_H

#ifdef SYNTAQLITE_CUSTOM_INCLUDE
# define SYNQ_STRINGIFY_(x) #x
# define SYNQ_STRINGIFY(x)  SYNQ_STRINGIFY_(x)
# include SYNQ_STRINGIFY(SYNTAQLITE_CUSTOM_INCLUDE)
#endif

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
// When SYNTAQLITE_SQLITE_VERSION is defined (compile-time pinning), these
// expand to integer constants and the compiler eliminates dead branches.
// When not defined, they check through the runtime config pointer.

// True if the target version is older than `ver`.
#ifdef SYNTAQLITE_SQLITE_VERSION
  #define SYNQ_VER_LT(config, ver) (SYNTAQLITE_SQLITE_VERSION < (ver))
#else
  #define SYNQ_VER_LT(config, ver) ((config)->sqlite_version < (ver))
#endif

// True if cflag at index `idx` is set in the config.
//
// When SYNTAQLITE_SQLITE_CFLAGS is defined (compile-time cflag pinning),
// reads from the synq_pinned_cflags struct built in sqlite_cflags.h from
// individual SYNTAQLITE_CFLAG_* defines. The compiler constant-folds the
// bit extraction and eliminates dead branches.
#ifdef SYNTAQLITE_SQLITE_CFLAGS
  #define SYNQ_HAS_CFLAG(config, idx) synq_has_cflag(&synq_pinned_cflags, (idx))
#else
  #define SYNQ_HAS_CFLAG(config, idx) synq_has_cflag(&(config)->cflags, (idx))
#endif

#endif  // SYNTAQLITE_DIALECT_CONFIG_H
