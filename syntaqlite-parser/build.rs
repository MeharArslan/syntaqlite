use std::env;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let csrc = manifest_dir.join("csrc");

    cc::Build::new()
        .file(csrc.join("sqlite_tokenize.c"))
        .include(&manifest_dir)
        .compile("syntaqlite_tokenize");

    println!("cargo:rerun-if-changed=csrc/sqlite_tokenize.c");
    println!("cargo:rerun-if-changed=csrc/sqlite_compat.h");
    println!("cargo:rerun-if-changed=csrc/sqlite_tokens.h");
}
