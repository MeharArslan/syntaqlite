// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Dynamic dialect loading from shared libraries.
//!
//! External dialect shared libraries expose a C function
//! `syntaqlite_<name>_grammar()` (or `syntaqlite_grammar()` for the default)
//! that returns a [`syntaqlite_syntax::typed::CGrammar`] value.
//!
//! [`load_dialect`] handles opening the library, resolving the symbol,
//! calling it, and wrapping the result in a [`Dialect`] that keeps the
//! library alive via an [`Arc`].

use std::sync::Arc;

use syntaqlite_syntax::any::AnyGrammar;
use syntaqlite_syntax::typed::CGrammar;

use crate::dialect::Dialect;

/// Load a dialect from a shared library (`.so` / `.dylib` / `.dll`).
///
/// Resolves `syntaqlite_<name>_grammar` (or `syntaqlite_grammar` when `name`
/// is `None`), calls it, and wraps the result in a [`Dialect`] that keeps
/// the library alive.
///
/// Dropping the last clone of the returned `Dialect` unloads the library.
///
/// Dynamically loaded dialects supply only the parser grammar; no formatter
/// bytecode or semantic role tables are present. This means `fmt` and
/// semantic validation are unavailable for dynamic dialects.
pub fn load_dialect(path: &str, name: Option<&str>) -> Result<Dialect, String> {
    // SAFETY: We keep `lib` alive in an `Arc` below and pass it to
    // `from_raw_parts` so the grammar pointer lives as long as the Dialect.
    let lib = unsafe {
        libloading::Library::new(path)
            .map_err(|e| format!("failed to load {path:?}: {e}"))?
    };

    let symbol = symbol_name(name);
    // SAFETY: We call the function immediately and drop `func` before `lib`
    // is moved into the Arc, so there is no lifetime overlap issue.
    let raw: CGrammar = unsafe {
        let func: libloading::Symbol<'_, unsafe extern "C" fn() -> CGrammar> = lib
            .get(symbol.as_bytes())
            .map_err(|e| format!("symbol {symbol:?} not found in {path:?}: {e}"))?;
        func()
    };

    // SAFETY: `raw.template` points to static C grammar tables embedded in
    // the shared library.  The `keep_alive` arc ensures the library stays
    // loaded for the entire lifetime of the returned Dialect.
    let grammar = unsafe { AnyGrammar::new(raw) };
    let keep_alive: Arc<dyn Send + Sync> = Arc::new(lib);

    Ok(unsafe { Dialect::from_raw_parts(grammar, &[], &[], &[], &[], &[], keep_alive) })
}

fn symbol_name(name: Option<&str>) -> String {
    match name {
        Some(n) => format!("syntaqlite_{n}_grammar"),
        None => "syntaqlite_grammar".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::symbol_name;

    #[test]
    fn default_symbol_name() {
        assert_eq!(symbol_name(None), "syntaqlite_grammar");
    }

    #[test]
    fn named_symbol_name() {
        assert_eq!(symbol_name(Some("sqlite")), "syntaqlite_sqlite_grammar");
    }
}
