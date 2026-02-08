//! Safe wrappers around compiled C tools (lemon, mkkeywordhash)

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

// External C functions compiled into the binary
unsafe extern "C" {
    fn lemon_main(argc: c_int, argv: *const *const c_char) -> c_int;
    // mkkeywordhash_main will be used later
    // fn mkkeywordhash_main(argc: c_int, argv: *const *const c_char,
    //                       aKeywordTable: *const Keyword, nKeyword: c_int) -> c_int;
}

/// Run lemon parser generator on a grammar file
///
/// # Arguments
/// * `grammar_path` - Path to the .y grammar file
/// * `args` - Additional command-line arguments to pass to lemon
///
/// # Errors
/// Returns error if lemon exits with non-zero status
pub fn run_lemon(grammar_path: &str, args: &[&str]) -> Result<(), String> {
    // Convert arguments to C strings
    let program_name = CString::new("lemon")
        .map_err(|e| format!("Invalid program name: {}", e))?;

    let grammar_path_c = CString::new(grammar_path)
        .map_err(|e| format!("Invalid grammar path: {}", e))?;

    let c_args: Vec<CString> = args.iter()
        .map(|&s| CString::new(s))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Invalid argument: {}", e))?;

    // Build argv array: [program_name, ...args, grammar_path]
    let mut argv_vec = vec![program_name.as_ptr()];
    argv_vec.extend(c_args.iter().map(|s| s.as_ptr()));
    argv_vec.push(grammar_path_c.as_ptr());

    let argc = argv_vec.len() as c_int;
    let argv = argv_vec.as_ptr();

    // SAFETY:
    // - lemon_main is a valid C function compiled into our binary
    // - argc matches the length of argv
    // - All CStrings are valid null-terminated C strings
    // - CStrings are kept alive until after the call via their variables
    // - argv pointers are valid for the duration of the call
    let result = unsafe { lemon_main(argc, argv) };

    if result != 0 {
        return Err(format!("lemon exited with code: {}", result));
    }

    Ok(())
}
