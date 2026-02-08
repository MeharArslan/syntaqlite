use std::env;
use std::path::PathBuf;
use std::fs;
use syntaqlite_codegen_utils::c_extractor::CExtractor;
use syntaqlite_codegen_utils::c_transform::{ChangeNameTransform, AddParametersTransform};
use syntaqlite_codegen_utils::c_writer::CWriter;

fn main() {
    // Get the path to SQLite tools relative to the workspace root
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let workspace_root = PathBuf::from(&manifest_dir)
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();

    let tools_dir = workspace_root
        .join("third_party")
        .join("src")
        .join("sqlite")
        .join("tool");

    let lemon_path = tools_dir.join("lemon.c");
    let mkkeywordhash_path = tools_dir.join("mkkeywordhash.c");

    println!("cargo:rerun-if-changed={}", lemon_path.display());
    println!("cargo:rerun-if-changed={}", mkkeywordhash_path.display());

    // Compile lemon.c with main renamed to lemon_main
    cc::Build::new()
        .file(&lemon_path)
        .define("main", "lemon_main")
        .compile("lemon");

    // Transform mkkeywordhash.c to accept custom keywords
    let out_dir = env::var("OUT_DIR").unwrap();
    let modified_mkkeywordhash = PathBuf::from(&out_dir).join("mkkeywordhash_modified.c");

    transform_mkkeywordhash(&mkkeywordhash_path, &modified_mkkeywordhash)
        .expect("Failed to transform mkkeywordhash.c");

    // Compile the modified mkkeywordhash.c
    cc::Build::new()
        .file(&modified_mkkeywordhash)
        .define("main", "mkkeywordhash_main")
        .compile("mkkeywordhash");

    println!("cargo:rerun-if-changed=build.rs");
}

fn transform_mkkeywordhash(input: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let content = fs::read_to_string(input)
        .map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

    // Split source by main function
    let extractor = CExtractor::new(&content);
    let split = extractor.split_by_function("main")?;

    // Transform: add parameters with names that shadow the globals
    let transformed = split.function
        .add_parameters("Keyword *aKeywordTable, int nKeyword");

    // Write the transformed file (finish() takes ownership so we can't chain perfectly)
    fs::write(output, {
        let mut w = CWriter::new();
        w.raw(&split.before)
         .raw(&transformed.text)
         .raw(&split.after);
        w.finish()
    }).map_err(|e| format!("Failed to write {}: {}", output.display(), e))?;

    Ok(())
}
