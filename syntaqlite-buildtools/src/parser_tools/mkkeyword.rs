// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! mkkeywordhash integration

use std::os::raw::{c_char, c_int};

use crate::util::tool_run;

/// Rust representation of the C Keyword struct.
///
/// Layout is verified against the C definition in tests below.
#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub(crate) struct Keyword {
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
    pub(crate) static aKeywordTable: [Keyword; 148];

    // The keyword count variable (static removed, const added by build.rs)
    pub(crate) static nKeyword: c_int;

    // The main function (renamed from main to mkkeyword_main)
    fn mkkeyword_main(
        argc: c_int,
        argv: *const *const c_char,
        keywords: *const Keyword,
        n_keywords: c_int,
    ) -> c_int;
}

/// Return the set of token names that are keywords in the base `SQLite` table.
///
/// Reads the compiled-in `aKeywordTable` and strips the `TK_` prefix from
/// each entry's `z_token_type` field, yielding names like `"SELECT"`,
/// `"FUNCTION"`, etc.
pub(crate) fn base_keyword_token_names() -> std::collections::HashSet<String> {
    let table_ptr = std::ptr::addr_of!(aKeywordTable);
    let n_keyword_ptr = std::ptr::addr_of!(nKeyword);
    // SAFETY: `aKeywordTable` and `nKeyword` are extern statics from compiled C
    // code that are always valid for reads. We read them once to extract keyword data.
    unsafe {
        #[allow(clippy::cast_sign_loss)]
        let n = n_keyword_ptr.read() as usize;
        let arr = std::ptr::read(table_ptr);
        arr[..n]
            .iter()
            .map(|kw| {
                let token_type = std::ffi::CStr::from_ptr(kw.z_token_type)
                    .to_string_lossy()
                    .to_string();
                token_type
                    .strip_prefix("TK_")
                    .unwrap_or(&token_type)
                    .to_string()
            })
            .collect()
    }
}

/// Run mkkeywordhash with arbitrary arguments (pass-through).
///
/// When `--extra-file <path>` is provided, reads extra keyword names from
/// that file (one per line) and appends them to the base keyword table.
/// Duplicates (keywords already in the base table) are silently skipped.
///
/// Without `--extra-file`, uses only the compiled-in base table.
///
/// **Warning: This function ALWAYS exits the process and never returns!**
pub(crate) fn run_mkkeyword(args: &[String]) -> ! {
    use std::ffi::CString;

    // Parse --extra-file from args.
    let mut extra_file: Option<String> = None;
    let mut forward_args: Vec<String> = Vec::new();
    let mut skip_next = false;
    for (i, arg) in args.iter().enumerate() {
        if skip_next {
            skip_next = false;
            continue;
        }
        if arg == "--extra-file" {
            extra_file = args.get(i + 1).cloned();
            skip_next = true;
        } else {
            forward_args.push(arg.clone());
        }
    }

    let c_args = tool_run::prepare_c_args("mkkeywordhash", &forward_args);

    // Read the compiled-in base keyword table.
    let table_ptr = std::ptr::addr_of!(aKeywordTable);
    let n_keyword_ptr = std::ptr::addr_of!(nKeyword);
    // SAFETY: `aKeywordTable` and `nKeyword` are extern statics from compiled C
    // code that are always valid for reads.
    let (keywords_array, n_keywords) = unsafe {
        let n = n_keyword_ptr.read();
        let arr = std::ptr::read(table_ptr);
        (arr, n)
    };
    let mut keywords_copy = keywords_array.to_vec();

    // Collect base keyword names for deduplication.
    // z_name is a pointer to a string literal (z_orig_name is zero-initialized
    // at this point — it gets filled in later by mkkeyword_main).
    let base_names: std::collections::HashSet<String> = keywords_copy
        .iter()
        .map(|kw| {
            // SAFETY: `z_name` is a valid C string pointer from the compiled-in keyword table.
            unsafe {
                std::ffi::CStr::from_ptr(kw.z_name)
                    .to_string_lossy()
                    .to_string()
            }
        })
        .collect();

    // Read extra keywords from file and append (skipping duplicates).
    let mut extra_cstrings: Vec<(CString, CString)> = Vec::new();
    if let Some(path) = &extra_file {
        let content =
            std::fs::read_to_string(path).unwrap_or_else(|e| panic!("reading {path}: {e}"));
        for line in content.lines() {
            let name = line.trim().to_uppercase();
            if name.is_empty() || base_names.contains(&name) {
                continue;
            }

            let name_c = CString::new(name.as_str()).expect("keyword contains NUL");
            let token_c = CString::new(format!("TK_{name}")).expect("token type contains NUL");
            extra_cstrings.push((name_c, token_c));
        }
    }

    for (name_c, token_c) in &extra_cstrings {
        let bytes = name_c.to_bytes();
        assert!(
            bytes.len() < 20,
            "keyword {name_c:?} too long for z_orig_name[20]"
        );

        let mut z_orig_name = [0i8; 20];
        for (i, &b) in bytes.iter().enumerate() {
            z_orig_name[i] = b.cast_signed();
        }

        keywords_copy.push(Keyword {
            z_name: name_c.as_ptr().cast_mut(),
            z_token_type: token_c.as_ptr().cast_mut(),
            mask: 0x0000_0002, // ALWAYS
            priority: 0,
            id: 0,
            hash: 0,
            offset: 0,
            len: 0,
            prefix: 0,
            longest_suffix: 0,
            i_next: 0,
            substr_id: 0,
            substr_offset: 0,
            z_orig_name,
        });
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_possible_wrap)]
    let total = n_keywords + extra_cstrings.len() as c_int;
    let arg_count = c_args.argc;
    let arg_vec = c_args.argv();
    let keywords_ptr = keywords_copy.as_ptr();

    // SAFETY: mkkeyword_main is a valid C function; all pointers and CStrings
    // are kept alive in keywords_copy, extra_cstrings, and c_args.
    let exit_code = unsafe { mkkeyword_main(arg_count, arg_vec, keywords_ptr, total) };

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
