// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Fragment normalization and SHA-256 hashing.
//!
//! Normalization strips C comments and collapses whitespace so that
//! semantically identical fragments (differing only in formatting or
//! comments) produce the same hash.

use sha2::{Digest, Sha256};

/// Normalize a C source fragment and return its SHA-256 hex digest.
pub(super) fn normalized_hash(text: &str) -> String {
    let normalized = normalize(text);
    let mut hasher = Sha256::new();
    hasher.update(normalized.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

/// Normalize C source text for comparison:
/// 1. Strip `/* ... */` and `// ...` comments
/// 2. Collapse runs of whitespace to a single space
/// 3. Trim each line
fn normalize(text: &str) -> String {
    let without_comments = strip_c_comments(text);
    without_comments
        .lines()
        .map(|line| {
            // Collapse internal whitespace runs.
            let mut result = String::new();
            let mut prev_space = false;
            for ch in line.trim().chars() {
                if ch.is_whitespace() {
                    if !prev_space {
                        result.push(' ');
                    }
                    prev_space = true;
                } else {
                    result.push(ch);
                    prev_space = false;
                }
            }
            result
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Strip C-style block comments (`/* ... */`) and line comments (`// ...`).
fn strip_c_comments(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'*' {
            // Skip block comment.
            i += 2;
            while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                i += 1;
            }
            i += 2; // skip */
            result.push(' ');
        } else if i + 1 < len && bytes[i] == b'/' && bytes[i + 1] == b'/' {
            // Skip line comment.
            i += 2;
            while i < len && bytes[i] != b'\n' {
                i += 1;
            }
        } else if bytes[i] == b'"' {
            // Preserve string literals.
            result.push('"');
            i += 1;
            while i < len && bytes[i] != b'"' {
                if bytes[i] == b'\\' && i + 1 < len {
                    result.push(bytes[i] as char);
                    result.push(bytes[i + 1] as char);
                    i += 2;
                } else {
                    result.push(bytes[i] as char);
                    i += 1;
                }
            }
            if i < len {
                result.push('"');
                i += 1;
            }
        } else if bytes[i] == b'\'' {
            // Preserve character literals.
            result.push('\'');
            i += 1;
            while i < len && bytes[i] != b'\'' {
                if bytes[i] == b'\\' && i + 1 < len {
                    result.push(bytes[i] as char);
                    result.push(bytes[i + 1] as char);
                    i += 2;
                } else {
                    result.push(bytes[i] as char);
                    i += 1;
                }
            }
            if i < len {
                result.push('\'');
                i += 1;
            }
        } else {
            result.push(bytes[i] as char);
            i += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_strips_comments() {
        let input = "int x = 1; /* comment */\nint y = 2; // another";
        let norm = normalize(input);
        assert_eq!(norm, "int x = 1;\nint y = 2;");
    }

    #[test]
    fn normalize_collapses_whitespace() {
        let input = "int   x  =   1;";
        let norm = normalize(input);
        assert_eq!(norm, "int x = 1;");
    }

    #[test]
    fn normalize_preserves_strings() {
        let input = r#"char *s = "hello /* not a comment */";"#;
        let norm = normalize(input);
        assert!(norm.contains("hello /* not a comment */"));
    }

    #[test]
    fn identical_fragments_same_hash() {
        let a = "int x = 1; /* comment A */";
        let b = "int x = 1; /* comment B */";
        assert_eq!(normalized_hash(a), normalized_hash(b));
    }

    #[test]
    fn different_fragments_different_hash() {
        let a = "int x = 1;";
        let b = "int x = 2;";
        assert_ne!(normalized_hash(a), normalized_hash(b));
    }
}
