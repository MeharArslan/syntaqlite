use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");

    cc::Build::new()
        .file(csrc.join("sqlite_tokenize.c"))
        .file(csrc.join("sqlite_keyword.c"))
        .file(csrc.join("ast.c"))
        .include(&manifest_dir)
        .include(manifest_dir.join("include"))
        .compile("syntaqlite_parser");

    println!("cargo:rerun-if-changed=csrc/sqlite_tokenize.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_keyword.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_compat.h");
    println!("cargo:rerun-if-changed=include/syntaqlite/tokens.h");
    println!("cargo:rerun-if-changed=csrc/sqlite_keyword.h");
    println!("cargo:rerun-if-changed=csrc/sqlite_keyword_tables.h");
    println!("cargo:rerun-if-changed=csrc/ast.h");
    println!("cargo:rerun-if-changed=csrc/ast.c");
    println!("cargo:rerun-if-changed=csrc/ast_builder.h");
    println!("cargo:rerun-if-changed=csrc/ast_range_meta.h");
    println!("cargo:rerun-if-changed=include/syntaqlite/ast.h");
    println!("cargo:rerun-if-changed=include/syntaqlite/ast_nodes.h");
}
