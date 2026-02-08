//! Lemon parser generator integration

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

// External C function compiled into the binary
unsafe extern "C" {
    fn lemon_main(argc: c_int, argv: *const *const c_char) -> c_int;
}

/// Run lemon with arbitrary arguments (pass-through)
///
/// **Warning: This function ALWAYS exits the process and never returns!**
///
/// This function calls the embedded lemon C code and then exits the process
/// with lemon's exit code. This ensures consistent behavior since lemon can
/// call `exit()` directly anyway.
///
/// **Only use this function if:**
/// - You're calling it from a CLI tool where exiting is acceptable
/// - You're running it in a subprocess that can be terminated
///
/// # Arguments
/// * `args` - Arguments to pass to lemon (not including program name)
pub fn run_lemon(args: &[String]) -> ! {
    // Convert arguments to C strings
    let program_name = CString::new("lemon").unwrap_or_else(|e| {
        eprintln!("Invalid program name: {}", e);
        std::process::exit(1);
    });

    // Convert all arguments to CStrings
    let arg_cstrings: Vec<CString> = args
        .iter()
        .map(|arg| {
            CString::new(arg.as_str()).unwrap_or_else(|e| {
                eprintln!("Invalid argument '{}': {}", arg, e);
                std::process::exit(1);
            })
        })
        .collect();

    // Build argv array: ["lemon", arg1, arg2, ..., NULL]
    let mut argv_vec = vec![program_name.as_ptr()];
    argv_vec.extend(arg_cstrings.iter().map(|cs| cs.as_ptr()));
    argv_vec.push(std::ptr::null()); // null terminator

    let argc = (argv_vec.len() - 1) as c_int; // Don't count the null terminator
    let argv = argv_vec.as_ptr();

    // SAFETY:
    // - lemon_main is a valid C function compiled into our binary
    // - argc matches the length of argv (excluding null terminator)
    // - All CStrings are valid null-terminated C strings
    // - CStrings are kept alive until after the call via their variables
    // - argv pointers are valid for the duration of the call
    // - argv array is null-terminated for C
    let exit_code = unsafe { lemon_main(argc, argv) };

    // Always exit with lemon's exit code
    std::process::exit(exit_code);
}
