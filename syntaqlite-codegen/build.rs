use std::env;
use std::fs;
use std::path::PathBuf;
use syntaqlite_codegen_utils::c_transformer::CTransformer;

fn main() {
    // Get the path to vendored SQLite tools in this crate
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let sqlite_dir = PathBuf::from(&manifest_dir).join("sqlite");

    let lemon_path = sqlite_dir.join("lemon.c");
    let mkkeywordhash_path = sqlite_dir.join("mkkeywordhash.c");

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
        .flag_if_supported("-Wno-missing-field-initializers")
        .flag_if_supported("-Wno-unused-parameter")
        .flag_if_supported("-Wno-unused-variable")
        .flag_if_supported("-Wno-sign-compare")
        .compile("mkkeywordhash");

    println!("cargo:rerun-if-changed=build.rs");
}

fn transform_mkkeywordhash(input: &PathBuf, output: &PathBuf) -> Result<(), String> {
    let content = fs::read_to_string(input)
        .map_err(|e| format!("Failed to read {}: {}", input.display(), e))?;

    let transformed = CTransformer::new(&content)
        .remove_static("aKeywordTable")
        .add_const("Keyword aKeywordTable")
        .remove_static("nKeyword")
        .add_const("int nKeyword")
        .add_function_parameters("findById", "Keyword *aKeywordTable, int nKeyword")
        .add_function_parameters("reorder", "Keyword *aKeywordTable, int nKeyword")
        .add_function_parameters("main", "Keyword *aKeywordTable, int nKeyword")
        // Fix call sites: append keyword args after existing args
        .replace_in_function(
            "main",
            "findById(p->id)",
            "findById(p->id, aKeywordTable, nKeyword)",
        )
        .replace_in_function(
            "main",
            "findById(p->substrId)",
            "findById(p->substrId, aKeywordTable, nKeyword)",
        )
        .replace_in_function(
            "main",
            "reorder(&aKWHash[h])",
            "reorder(&aKWHash[h], aKeywordTable, nKeyword)",
        )
        // Fix recursive call in reorder function itself
        .replace_in_function(
            "reorder",
            "reorder(&aKeywordTable[i].iNext)",
            "reorder(&aKeywordTable[i].iNext, aKeywordTable, nKeyword)",
        )
        .rename_function("main", "mkkeyword_main")
        .finish();

    fs::write(output, transformed)
        .map_err(|e| format!("Failed to write {}: {}", output.display(), e))?;

    Ok(())
}
