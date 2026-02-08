// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// SQLite compatibility definitions for syntaqlite.
// Provides type aliases, macros, and structures needed by tokenizer and parser.

#ifndef SYNQ_SRC_COMMON_SQLITE_COMPAT_H
#define SYNQ_SRC_COMMON_SQLITE_COMPAT_H

#include <stdint.h>

// SQLite type aliases
typedef int64_t i64;
typedef uint8_t u8;
typedef uint32_t u32;

// Default to SQLITE_ASCII if not defined and SQLITE_EBCDIC also not defined
#if !defined(SQLITE_ASCII) && !defined(SQLITE_EBCDIC)
#define SQLITE_ASCII 1
#endif

// Default digit separator for numeric literals (e.g., 1_000_000)
#ifndef SQLITE_DIGIT_SEPARATOR
#define SQLITE_DIGIT_SEPARATOR '_'
#endif

// C++17 fallthrough, C no-op
#ifdef __cplusplus
#define deliberate_fall_through [[fallthrough]]
#else
#define deliberate_fall_through
#endif // SYNQ_SRC_COMMON_SQLITE_COMPAT_H

// No-op for testcase macro if not defined
#ifndef testcase
#define testcase(X)
#endif

#endif // SYNQ_SRC_COMMON_SQLITE_COMPAT_H
