//! Lemon parser generator integration

use std::ffi::CString;
use std::os::raw::{c_char, c_int};

// External C function compiled into the binary
unsafe extern "C" {
    fn lemon_main(argc: c_int, argv: *const *const c_char) -> c_int;
}

/// Run lemon parser generator on a grammar file
///
/// # Arguments
/// * `input_path` - Path to the input grammar file (.y format)
/// * `output_path` - Optional output file base name (without extension)
///
/// # Errors
/// Returns error if lemon fails
pub fn run_lemon_on_file(input_path: &str, output_path: Option<&str>) -> Result<(), String> {
    // Convert arguments to C strings
    let program_name = CString::new("lemon").map_err(|e| format!("Invalid program name: {}", e))?;

    let input_c = CString::new(input_path).map_err(|e| format!("Invalid input path: {}", e))?;

    // Build argv array
    let mut argv_vec = vec![program_name.as_ptr(), input_c.as_ptr()];

    // Add output argument if provided
    let output_c = if let Some(output) = output_path {
        Some(CString::new(output).map_err(|e| format!("Invalid output path: {}", e))?)
    } else {
        None
    };

    if let Some(ref out_c) = output_c {
        argv_vec.push(out_c.as_ptr());
    }

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
