//! Lemon parser generator integration

use std::os::raw::{c_char, c_int};

use crate::run;

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
    let c_args = run::prepare_c_args("lemon", args);

    // SAFETY:
    // - lemon_main is a valid C function compiled into our binary
    // - argc matches the length of argv (excluding null terminator)
    // - All CStrings are valid null-terminated C strings
    // - CStrings are kept alive in c_args until after the call
    // - argv pointers are valid for the duration of the call
    // - argv array is null-terminated for C
    let exit_code = unsafe { lemon_main(c_args.argc, c_args.argv()) };

    std::process::exit(exit_code);
}
