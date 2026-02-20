// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! mkkeywordhash integration

use std::os::raw::{c_char, c_int};

use crate::run;

/// Rust representation of the C Keyword struct.
///
/// Layout is verified against the C definition in tests below.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct Keyword {
    pub z_name: *mut c_char,
    pub z_token_type: *mut c_char,
    pub mask: c_int,
    pub priority: c_int,
    pub id: c_int,
    pub hash: c_int,
    pub offset: c_int,
    pub len: c_int,
    pub prefix: c_int,
    pub longest_suffix: c_int,
    pub i_next: c_int,
    pub substr_id: c_int,
    pub substr_offset: c_int,
    pub z_orig_name: [c_char; 20],
}

// External symbols from compiled C code
unsafe extern "C" {
    // The keyword table array (static removed, const added by build.rs)
    pub static aKeywordTable: [Keyword; 148];

    // The keyword count variable (static removed, const added by build.rs)
    pub static nKeyword: c_int;

    // The main function (renamed from main to mkkeyword_main)
    fn mkkeyword_main(
        argc: c_int,
        argv: *const *const c_char,
        keywords: *const Keyword,
        n_keywords: c_int,
    ) -> c_int;
}

/// Run mkkeywordhash with arbitrary arguments (pass-through)
///
/// **Warning: This function ALWAYS exits the process and never returns!**
///
/// This function calls the embedded mkkeywordhash C code and then exits the process
/// with mkkeywordhash's exit code. This ensures consistent behavior since C code can
/// call `exit()` directly anyway.
///
/// **Only use this function if:**
/// - You're calling it from a CLI tool where exiting is acceptable
/// - You're running it in a subprocess that can be terminated
///
/// # Arguments
/// * `args` - Arguments to pass to mkkeywordhash (not including program name)
pub fn run_mkkeyword(args: &[String]) -> ! {
    let c_args = run::prepare_c_args("mkkeywordhash", args);

    // Get pointers to the global data
    let table_ptr = std::ptr::addr_of!(aKeywordTable);
    let n_keyword_ptr = std::ptr::addr_of!(nKeyword);

    // Read the global data
    // SAFETY:
    // - aKeywordTable and nKeyword are valid global symbols compiled into our binary
    // - read() copies the data without creating references
    let (keywords_array, n_keywords) = unsafe {
        let n_keywords = n_keyword_ptr.read();
        let keywords_array = std::ptr::read(table_ptr);
        (keywords_array, n_keywords)
    };

    // Create a mutable copy for C to modify
    let keywords_copy = keywords_array.to_vec();

    // Prepare arguments for C function call
    let argc = c_args.argc;
    let argv = c_args.argv();
    let keywords_ptr = keywords_copy.as_ptr();

    // Call the C function with the mutable copy
    // SAFETY:
    // - mkkeyword_main is a valid C function compiled into our binary
    // - argc matches the length of argv (excluding null terminator)
    // - All CStrings are valid null-terminated C strings
    // - CStrings are kept alive in c_args until after the call
    // - argv pointers are valid for the duration of the call
    // - argv array is null-terminated for C
    // - keywords_copy is kept alive for the duration of the call
    let exit_code = unsafe { mkkeyword_main(argc, argv, keywords_ptr, n_keywords) };

    std::process::exit(exit_code);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    unsafe extern "C" {
        static keyword_sizeof: usize;
        static keyword_offsetof_zName: usize;
        static keyword_offsetof_zTokenType: usize;
        static keyword_offsetof_mask: usize;
        static keyword_offsetof_priority: usize;
        static keyword_offsetof_id: usize;
        static keyword_offsetof_hash: usize;
        static keyword_offsetof_offset: usize;
        static keyword_offsetof_len: usize;
        static keyword_offsetof_prefix: usize;
        static keyword_offsetof_longestSuffix: usize;
        static keyword_offsetof_iNext: usize;
        static keyword_offsetof_substrId: usize;
        static keyword_offsetof_substrOffset: usize;
        static keyword_offsetof_zOrigName: usize;
    }

    #[test]
    fn keyword_struct_matches_c_layout() {
        unsafe {
            assert_eq!(mem::size_of::<Keyword>(), keyword_sizeof, "sizeof mismatch");
            assert_eq!(mem::offset_of!(Keyword, z_name), keyword_offsetof_zName);
            assert_eq!(
                mem::offset_of!(Keyword, z_token_type),
                keyword_offsetof_zTokenType
            );
            assert_eq!(mem::offset_of!(Keyword, mask), keyword_offsetof_mask);
            assert_eq!(
                mem::offset_of!(Keyword, priority),
                keyword_offsetof_priority
            );
            assert_eq!(mem::offset_of!(Keyword, id), keyword_offsetof_id);
            assert_eq!(mem::offset_of!(Keyword, hash), keyword_offsetof_hash);
            assert_eq!(mem::offset_of!(Keyword, offset), keyword_offsetof_offset);
            assert_eq!(mem::offset_of!(Keyword, len), keyword_offsetof_len);
            assert_eq!(mem::offset_of!(Keyword, prefix), keyword_offsetof_prefix);
            assert_eq!(
                mem::offset_of!(Keyword, longest_suffix),
                keyword_offsetof_longestSuffix
            );
            assert_eq!(mem::offset_of!(Keyword, i_next), keyword_offsetof_iNext);
            assert_eq!(
                mem::offset_of!(Keyword, substr_id),
                keyword_offsetof_substrId
            );
            assert_eq!(
                mem::offset_of!(Keyword, substr_offset),
                keyword_offsetof_substrOffset
            );
            assert_eq!(
                mem::offset_of!(Keyword, z_orig_name),
                keyword_offsetof_zOrigName
            );
        }
    }
}
