// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Shared utility helpers used by codegen modules.

/// Convert PascalCase to snake_case.
pub fn pascal_to_snake(name: &str) -> String {
    let mut out = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.push(c.to_ascii_lowercase());
    }
    out
}

/// Convert PascalCase to UPPER_SNAKE_CASE.
pub fn upper_snake(name: &str) -> String {
    pascal_to_snake(name).to_uppercase()
}

/// Convert UPPER_SNAKE to PascalCase.
pub fn upper_snake_to_pascal(name: &str) -> String {
    name.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    let mut s = first.to_uppercase().to_string();
                    s.extend(chars.map(|c| c.to_ascii_lowercase()));
                    s
                }
                None => String::new(),
            }
        })
        .collect()
}

/// Convert snake_case to PascalCase.
pub fn pascal_case(s: &str) -> String {
    s.split('_')
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}
