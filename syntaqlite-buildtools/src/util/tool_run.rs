// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Common utilities for running embedded C programs

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

/// Prepared C arguments ready to pass to a C main function
///
/// This struct holds the converted CStrings and pointers needed to call
/// a C function with argc/argv signature. The CStrings must be kept alive
/// for the duration of the C function call.
pub struct CArgs {
    /// The CStrings that must be kept alive
    _strings: Vec<CString>,
    /// The argv array (null-terminated)
    argv: Vec<*const c_char>,
    /// The argc count (excludes null terminator)
    pub(crate) argc: c_int,
}

impl CArgs {
    /// Get the argv pointer for passing to C
    pub(crate) fn argv(&self) -> *const *const c_char {
        self.argv.as_ptr()
    }
}

/// Convert Rust strings to C-compatible argc/argv
///
/// This prepares arguments for invoking C programs that expect standard main() signature.
/// The returned `CArgs` must be kept alive for the duration of the C function call.
///
/// # Arguments
/// * `program_name` - Name of the program (will be argv[0])
/// * `args` - Arguments to pass (will be argv[1..])
///
/// # Returns
/// A `CArgs` struct containing argc and argv pointers
///
/// # Panics
/// Panics if any string contains null bytes (invalid for C strings)
pub fn prepare_c_args(program_name: &str, args: &[String]) -> CArgs {
    // Convert program name to C string
    let program_cstring = CString::new(program_name).unwrap_or_else(|e| {
        eprintln!("Invalid program name '{}': {}", program_name, e);
        std::process::exit(1);
    });

    // Convert all arguments to CStrings
    let mut cstrings = vec![program_cstring];
    cstrings.extend(args.iter().map(|arg| {
        CString::new(arg.as_str()).unwrap_or_else(|e| {
            eprintln!("Invalid argument '{}': {}", arg, e);
            std::process::exit(1);
        })
    }));

    // Build argv array: [program_name, arg1, arg2, ..., NULL]
    let mut argv = Vec::with_capacity(cstrings.len() + 1);
    argv.extend(cstrings.iter().map(|cs| cs.as_ptr()));
    argv.push(std::ptr::null()); // null terminator

    let argc = (argv.len() - 1) as c_int; // Don't count the null terminator

    CArgs {
        _strings: cstrings,
        argv,
        argc,
    }
}
