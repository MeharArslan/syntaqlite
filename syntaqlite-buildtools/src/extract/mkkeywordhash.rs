// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Stage 1: Transform mkkeywordhash.c for embedding as a library.
//!
//! Produces `mkkeywordhash_modified.c` which:
//! - Renames `main` to `mkkeyword_main`
//! - Makes `aKeywordTable` and `nKeyword` non-static + const
//! - Passes keyword table as parameters (so callers can supply extra keywords)
//! - Appends struct layout export constants for FFI

use std::fs;
use std::path::Path;

/// Transform raw `mkkeywordhash.c` for use as a linked library.
pub fn transform_mkkeywordhash(source: &str) -> String {
    let mut s = source.to_string();

    // remove_static_first("aKeywordTable") + add_const("Keyword aKeywordTable")
    s = s.replacen(
        "static Keyword aKeywordTable",
        "const Keyword aKeywordTable",
        1,
    );

    // remove_static_first("nKeyword") + add_const("int nKeyword")
    s = s.replacen("static int nKeyword", "const int nKeyword", 1);

    // add_function_parameters: findById, reorder, main
    s = s.replacen(
        "findById(int id){",
        "findById(int id, Keyword *aKeywordTable, int nKeyword){",
        1,
    );
    s = s.replacen(
        "reorder(int *pFrom){",
        "reorder(int *pFrom, Keyword *aKeywordTable, int nKeyword){",
        1,
    );
    s = s.replacen(
        "main(int argc, char **argv){",
        "main(int argc, char **argv, Keyword *aKeywordTable, int nKeyword){",
        1,
    );

    // Fix call sites: append keyword args after existing args.
    s = s.replace(
        "findById(p->id)",
        "findById(p->id, aKeywordTable, nKeyword)",
    );
    s = s.replace(
        "findById(p->substrId)",
        "findById(p->substrId, aKeywordTable, nKeyword)",
    );
    s = s.replace(
        "reorder(&aKWHash[h])",
        "reorder(&aKWHash[h], aKeywordTable, nKeyword)",
    );
    s = s.replace(
        "reorder(&aKeywordTable[i].iNext)",
        "reorder(&aKeywordTable[i].iNext, aKeywordTable, nKeyword)",
    );

    // rename_function("main", "mkkeyword_main")
    s = s.replace("main(int argc", "mkkeyword_main(int argc");

    // add_system_include("stddef.h")
    s = format!("#include <stddef.h>\n{s}");

    // Append struct layout exports for FFI.
    s.push_str(concat!(
        "const size_t keyword_sizeof = sizeof(struct Keyword);\n",
        "const size_t keyword_offsetof_zName = offsetof(struct Keyword, zName);\n",
        "const size_t keyword_offsetof_zTokenType = offsetof(struct Keyword, zTokenType);\n",
        "const size_t keyword_offsetof_mask = offsetof(struct Keyword, mask);\n",
        "const size_t keyword_offsetof_priority = offsetof(struct Keyword, priority);\n",
        "const size_t keyword_offsetof_id = offsetof(struct Keyword, id);\n",
        "const size_t keyword_offsetof_hash = offsetof(struct Keyword, hash);\n",
        "const size_t keyword_offsetof_offset = offsetof(struct Keyword, offset);\n",
        "const size_t keyword_offsetof_len = offsetof(struct Keyword, len);\n",
        "const size_t keyword_offsetof_prefix = offsetof(struct Keyword, prefix);\n",
        "const size_t keyword_offsetof_longestSuffix = offsetof(struct Keyword, longestSuffix);\n",
        "const size_t keyword_offsetof_iNext = offsetof(struct Keyword, iNext);\n",
        "const size_t keyword_offsetof_substrId = offsetof(struct Keyword, substrId);\n",
        "const size_t keyword_offsetof_substrOffset = offsetof(struct Keyword, substrOffset);\n",
        "const size_t keyword_offsetof_zOrigName = offsetof(struct Keyword, zOrigName);\n",
    ));

    s
}

/// Write the transformed mkkeywordhash.c to the output path.
pub fn write_modified_mkkeywordhash(source: &str, output_path: &Path) -> Result<(), String> {
    let transformed = transform_mkkeywordhash(source);
    fs::write(output_path, transformed)
        .map_err(|e| format!("writing {}: {e}", output_path.display()))
}
