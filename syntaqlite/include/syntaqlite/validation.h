// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

// Semantic validation C API — validates SQL against a catalog of known
// tables, columns, and functions.
//
// The validator works incrementally: DDL statements (CREATE TABLE, etc.)
// accumulate in the catalog as they are analyzed, so later statements can
// reference earlier definitions.
//
// Lifecycle:
//   SyntaqliteValidator* v = syntaqlite_validator_create_sqlite();
//   uint32_t n = syntaqlite_validator_analyze(v, sql, len);
//   const SyntaqliteDiagnostic* d = syntaqlite_validator_diagnostics(v);
//   for (uint32_t i = 0; i < n; i++) {
//     d[i].severity, d[i].message, d[i].start_offset, d[i].end_offset
//   }
//   syntaqlite_validator_destroy(v);
//
// The catalog persists across analyze() calls — each call accumulates DDL
// from the analyzed source. Call syntaqlite_validator_reset_catalog() to
// clear accumulated schema.

#ifndef SYNTAQLITE_VALIDATION_H
#define SYNTAQLITE_VALIDATION_H

#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

// Opaque validator handle. Owns a SemanticAnalyzer + Catalog internally.
typedef struct SyntaqliteValidator SyntaqliteValidator;

// Diagnostic severity levels.
typedef enum {
  SYNTAQLITE_SEVERITY_ERROR = 0,
  SYNTAQLITE_SEVERITY_WARNING = 1,
  SYNTAQLITE_SEVERITY_INFO = 2,
  SYNTAQLITE_SEVERITY_HINT = 3,
} SyntaqliteSeverity;

// A single diagnostic from validation. Pointers are valid until the next
// analyze() or destroy() call.
typedef struct {
  SyntaqliteSeverity severity;
  const char* message;
  uint32_t start_offset;
  uint32_t end_offset;
} SyntaqliteDiagnostic;

// Table definition for batch catalog registration.
typedef struct {
  const char* name;
  const char* const* columns;  // NULL = columns unknown
  uint32_t column_count;       // ignored when columns is NULL
} SyntaqliteTableDef;

// Analysis mode — controls whether DDL persists across analyze() calls.
typedef enum {
  // Statements are being analyzed (e.g. editing a SQL file).
  // DDL resets between analyze() calls.
  SYNTAQLITE_MODE_DOCUMENT = 0,
  // Statements are being executed sequentially.
  // DDL accumulates across analyze() calls.
  SYNTAQLITE_MODE_EXECUTE = 1,
} SyntaqliteAnalysisMode;

// ---------------------------------------------------------------------------
// Lifecycle
// ---------------------------------------------------------------------------

// Free the validator and all associated resources. No-op if v is NULL.
void syntaqlite_validator_destroy(SyntaqliteValidator* v);

// Set the analysis mode. See SyntaqliteAnalysisMode for details.
void syntaqlite_validator_set_mode(SyntaqliteValidator* v,
                                   SyntaqliteAnalysisMode mode);

// ---------------------------------------------------------------------------
// Analysis
// ---------------------------------------------------------------------------

// Analyze a SQL source string. The source may contain multiple statements
// separated by semicolons. DDL statements (CREATE TABLE, etc.) accumulate
// in the internal catalog so that later statements can reference them.
//
// Returns the number of diagnostics produced.
// The source buffer must remain valid only for the duration of this call.
uint32_t syntaqlite_validator_analyze(SyntaqliteValidator* v,
                                      const char* source,
                                      uint32_t len);

// Clear accumulated DDL from the catalog (document + connection layers).
// The dialect layer (built-in functions, etc.) is preserved.
void syntaqlite_validator_reset_catalog(SyntaqliteValidator* v);

// Add tables to the database layer of the catalog. These tables will be
// visible to all subsequent analyze() calls until reset_catalog() is called.
void syntaqlite_validator_add_tables(SyntaqliteValidator* v,
                                     const SyntaqliteTableDef* tables,
                                     uint32_t count);

// ---------------------------------------------------------------------------
// Diagnostic access (valid until next analyze() or destroy())
// ---------------------------------------------------------------------------

// Number of diagnostics from the last analyze() call.
uint32_t syntaqlite_validator_diagnostic_count(const SyntaqliteValidator* v);

// Pointer to the diagnostic array from the last analyze() call.
// Returns NULL when diagnostic_count is 0.
const SyntaqliteDiagnostic* syntaqlite_validator_diagnostics(
    const SyntaqliteValidator* v);

// ---------------------------------------------------------------------------
// Diagnostic rendering
// ---------------------------------------------------------------------------

// Render all diagnostics from the last analyze() call as a rustc-style
// human-readable string. Example output:
//
//   error: unknown table 'usr'
//    --> query.sql:1:15
//     |
//   1 | SELECT id FROM usr WHERE id = 1
//     |               ^~~
//     = help: did you mean 'users'?
//
// `file` is a NUL-terminated label for the "--> file:line:col" header.
// Pass NULL to use the default label "<input>".
//
// Returns a NUL-terminated UTF-8 string. The pointer is valid until the
// next analyze(), render_diagnostics(), or destroy() call.
// Returns an empty string when there are no diagnostics.
const char* syntaqlite_validator_render_diagnostics(SyntaqliteValidator* v,
                                                     const char* file);

// Free a string returned by a syntaqlite_* function that documents
// ownership transfer. No-op if s is NULL.
void syntaqlite_string_destroy(char* s);

// ---------------------------------------------------------------------------
// SQLite convenience (opt-out: -DSYNTAQLITE_OMIT_SQLITE_API)
// ---------------------------------------------------------------------------

#ifndef SYNTAQLITE_OMIT_SQLITE_API

// Create a validator for the built-in SQLite dialect.
// The default analysis mode is SYNTAQLITE_MODE_DOCUMENT.
SyntaqliteValidator* syntaqlite_validator_create_sqlite(void);

#endif  // SYNTAQLITE_OMIT_SQLITE_API

#ifdef __cplusplus
}
#endif

#endif  // SYNTAQLITE_VALIDATION_H
