// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! C FFI layer for the semantic validator.
//!
//! Exposes [`SemanticAnalyzer`] and [`Catalog`] to C via opaque handle +
//! accessor functions, following the same pattern as the parser FFI in
//! `syntaqlite-syntax`.

use std::ffi::{CStr, CString, c_char};

use crate::dialect::AnyDialect;

use super::{AnalysisMode, ValidationConfig};
use super::analyzer::SemanticAnalyzer;
use super::catalog::{Catalog, CatalogLayer};
use super::diagnostics::Severity;

// ── C-compatible diagnostic struct ──────────────────────────────────────────

/// Mirrors `SyntaqliteDiagnostic` from the C header.
#[repr(C)]
pub struct SyntaqliteDiagnostic {
    pub severity: u32,
    pub message: *const c_char,
    pub start_offset: u32,
    pub end_offset: u32,
}

/// Mirrors `SyntaqliteTableDef` from the C header.
#[repr(C)]
pub struct SyntaqliteTableDef {
    pub name: *const c_char,
    pub columns: *const *const c_char,
    pub column_count: u32,
}

/// Opaque validator handle exposed to C.
///
/// Owns a `SemanticAnalyzer`, a user `Catalog` (for persistent schema), and
/// the most recent diagnostics + rendered messages.
struct ValidatorState {
    analyzer: SemanticAnalyzer,
    user_catalog: Catalog,
    dialect: AnyDialect,
    /// C-compatible diagnostics from the most recent `analyze()` call.
    c_diagnostics: Vec<SyntaqliteDiagnostic>,
    /// Rendered message strings, kept alive for the C pointers.
    rendered_messages: Vec<CString>,
}

/// Opaque C handle — the pointer target of `SyntaqliteValidator*`.
///
/// This is a zero-variant enum so that Rust cannot construct it directly;
/// all access goes through raw pointer casts to `ValidatorState`.
pub enum SyntaqliteValidator {}

impl SyntaqliteValidator {
    fn state(&self) -> &ValidatorState {
        // SAFETY: `self` was created from a `Box<ValidatorState>` via
        // `Box::into_raw` cast in `syntaqlite_validator_create_sqlite`.
        unsafe { &*std::ptr::from_ref::<Self>(self).cast::<ValidatorState>() }
    }

    fn state_mut(&mut self) -> &mut ValidatorState {
        // SAFETY: `self` was created from a `Box<ValidatorState>` via
        // `Box::into_raw` cast in `syntaqlite_validator_create_sqlite`.
        unsafe { &mut *std::ptr::from_mut::<Self>(self).cast::<ValidatorState>() }
    }
}

// ── Severity mapping ────────────────────────────────────────────────────────

const SEVERITY_ERROR: u32 = 0;
const SEVERITY_WARNING: u32 = 1;
const SEVERITY_INFO: u32 = 2;
const SEVERITY_HINT: u32 = 3;

fn severity_to_c(s: Severity) -> u32 {
    match s {
        Severity::Error => SEVERITY_ERROR,
        Severity::Warning => SEVERITY_WARNING,
        Severity::Info => SEVERITY_INFO,
        Severity::Hint => SEVERITY_HINT,
    }
}

// ── Exported C functions ────────────────────────────────────────────────────

/// Create a validator for the built-in `SQLite` dialect.
#[cfg(feature = "sqlite")]
#[unsafe(no_mangle)]
pub extern "C" fn syntaqlite_validator_create_sqlite() -> *mut SyntaqliteValidator {
    let dialect: AnyDialect = crate::sqlite_dialect().into();
    let analyzer = SemanticAnalyzer::with_dialect(dialect.clone());
    let user_catalog = Catalog::new(dialect.clone());

    let state = Box::new(ValidatorState {
        analyzer,
        user_catalog,
        dialect,
        c_diagnostics: Vec::new(),
        rendered_messages: Vec::new(),
    });
    Box::into_raw(state).cast::<SyntaqliteValidator>()
}

/// Free a validator. No-op if `v` is NULL.
///
/// # Safety
///
/// `v` must be NULL or a valid pointer from `syntaqlite_validator_create_sqlite`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syntaqlite_validator_destroy(v: *mut SyntaqliteValidator) {
    if !v.is_null() {
        // SAFETY: `v` was created by `Box::into_raw` in `create_sqlite`.
        drop(unsafe { Box::from_raw(v.cast::<ValidatorState>()) });
    }
}

/// Set the analysis mode.
///
/// - `SYNTAQLITE_MODE_DOCUMENT` (0): DDL resets between `analyze()` calls.
/// - `SYNTAQLITE_MODE_EXECUTE` (1): DDL accumulates across `analyze()` calls.
///
/// # Safety
///
/// `v` must be a valid pointer from `syntaqlite_validator_create_sqlite`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syntaqlite_validator_set_mode(
    v: *mut SyntaqliteValidator,
    mode: u32,
) {
    let v = unsafe { &mut *v };
    let state = v.state_mut();
    state.analyzer.set_mode(match mode {
        1 => AnalysisMode::Execute,
        _ => AnalysisMode::Document,
    });
}

/// Analyze a SQL source string. Returns the number of diagnostics.
///
/// # Safety
///
/// - `v` must be a valid pointer from `syntaqlite_validator_create_sqlite`.
/// - `source` must point to `len` bytes of valid UTF-8.
#[unsafe(no_mangle)]
#[expect(clippy::cast_possible_truncation)]
pub unsafe extern "C" fn syntaqlite_validator_analyze(
    v: *mut SyntaqliteValidator,
    source: *const c_char,
    len: u32,
) -> u32 {
    // SAFETY: caller guarantees `v` is valid.
    let v = unsafe { &mut *v };
    let state = v.state_mut();

    // SAFETY: caller guarantees `source` points to `len` bytes of valid UTF-8.
    let src = unsafe {
        std::str::from_utf8_unchecked(std::slice::from_raw_parts(source.cast(), len as usize))
    };

    let config = ValidationConfig::default();
    let model = state.analyzer.analyze(src, &state.user_catalog, &config);

    // Reuse existing Vec capacity — clear + push avoids reallocating
    // on steady-state calls.
    state.rendered_messages.clear();
    state.c_diagnostics.clear();

    // First pass: render messages (must be done before building
    // SyntaqliteDiagnostic so the CString pointers are stable).
    for d in model.diagnostics() {
        state
            .rendered_messages
            .push(CString::new(d.message.to_string()).unwrap_or_default());
    }

    // Second pass: build C structs pointing into rendered_messages.
    for (d, msg) in model.diagnostics().iter().zip(state.rendered_messages.iter()) {
        state.c_diagnostics.push(SyntaqliteDiagnostic {
            severity: severity_to_c(d.severity),
            message: msg.as_ptr(),
            start_offset: d.start_offset as u32,
            end_offset: d.end_offset as u32,
        });
    }

    state.c_diagnostics.len() as u32
}

/// Clear accumulated DDL from the catalog.
///
/// # Safety
///
/// `v` must be a valid pointer from `syntaqlite_validator_create_sqlite`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syntaqlite_validator_reset_catalog(v: *mut SyntaqliteValidator) {
    // SAFETY: caller guarantees `v` is valid.
    let v = unsafe { &mut *v };
    let state = v.state_mut();
    state.user_catalog = Catalog::new(state.dialect.clone());
}

/// Add tables to the database layer of the catalog.
///
/// # Safety
///
/// - `v` must be a valid pointer from `syntaqlite_validator_create_sqlite`.
/// - `tables` must point to `count` valid `SyntaqliteTableDef` entries.
/// - Each `name` must be a valid NUL-terminated C string.
/// - Each `columns` may be NULL. If non-NULL, must point to `column_count`
///   valid NUL-terminated C string pointers.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syntaqlite_validator_add_tables(
    v: *mut SyntaqliteValidator,
    tables: *const SyntaqliteTableDef,
    count: u32,
) {
    // SAFETY: caller guarantees `v` is valid.
    let v = unsafe { &mut *v };
    let state = v.state_mut();

    for i in 0..count as usize {
        // SAFETY: caller guarantees `tables[i]` is valid.
        let def = unsafe { &*tables.add(i) };

        // SAFETY: caller guarantees `name` is a valid NUL-terminated C string.
        let name = unsafe { CStr::from_ptr(def.name) }
            .to_str()
            .unwrap_or("")
            .to_owned();

        let columns = if def.columns.is_null() {
            None
        } else {
            let cols: Vec<String> = (0..def.column_count as usize)
                .map(|j| {
                    // SAFETY: caller guarantees `columns[j]` is valid.
                    unsafe { CStr::from_ptr(*def.columns.add(j)) }
                        .to_str()
                        .unwrap_or("")
                        .to_owned()
                })
                .collect();
            Some(cols)
        };

        state
            .user_catalog
            .layer_mut(CatalogLayer::Database)
            .insert_relation(name, columns);
    }
}

/// Number of diagnostics from the last `analyze()` call.
///
/// # Safety
///
/// `v` must be a valid pointer from `syntaqlite_validator_create_sqlite`.
#[unsafe(no_mangle)]
#[expect(clippy::cast_possible_truncation)]
pub unsafe extern "C" fn syntaqlite_validator_diagnostic_count(
    v: *const SyntaqliteValidator,
) -> u32 {
    // SAFETY: caller guarantees `v` is valid.
    let v = unsafe { &*v };
    v.state().c_diagnostics.len() as u32
}

/// Pointer to the diagnostic array from the last `analyze()` call.
/// Returns NULL when diagnostic count is 0.
///
/// # Safety
///
/// `v` must be a valid pointer from `syntaqlite_validator_create_sqlite`.
/// The returned pointer is valid until the next `analyze()` or `destroy()`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn syntaqlite_validator_diagnostics(
    v: *const SyntaqliteValidator,
) -> *const SyntaqliteDiagnostic {
    // SAFETY: caller guarantees `v` is valid.
    let v = unsafe { &*v };
    let state = v.state();
    if state.c_diagnostics.is_empty() {
        std::ptr::null()
    } else {
        state.c_diagnostics.as_ptr()
    }
}
