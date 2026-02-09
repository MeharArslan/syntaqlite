use clap::{Parser, Subcommand};
use std::fs;
use std::path::Path;
use std::process;

#[derive(Parser)]
#[command(name = "syntaqlite-codegen")]
#[command(about = "SQLite grammar extraction and code generation")]
struct Args {
    #[command(subcommand)]
    command: Command,
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    Codegen {
        #[arg(long, required = true)]
        actions_dir: String,
        #[arg(long, required = true)]
        tokenize_c: String,
        #[arg(long, default_value = "syntaqlite-parser/csrc")]
        output_dir: String,
    },
    Lemon {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    Mkkeyword {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() {
    let args = Args::parse();

    let result = (|| -> Result<(), String> {
        match args.command {
            Command::Codegen {
                actions_dir,
                tokenize_c,
                output_dir,
            } => {
                let temp_dir = tempfile::TempDir::new()
                    .map_err(|e| format!("Failed to create temp directory: {}", e))?;
                let work_dir = temp_dir.path().to_str()
                    .ok_or_else(|| "Invalid temp directory path".to_string())?
                    .to_string();

                // Step 1: Generate parser (writes parse.c, parse.h into temp dir)
                if args.verbose {
                    eprintln!("Generating parser...");
                }
                syntaqlite_codegen::generate_parser(&actions_dir, &work_dir)?;

                // Step 2: Extract tokenizer
                if args.verbose {
                    eprintln!("Extracting tokenizer...");
                }
                let (tokenize_content, extract_result) =
                    syntaqlite_codegen::extract_tokenizer(&tokenize_c)?;

                // Step 3: Generate keyword hash
                if args.verbose {
                    eprintln!("Generating keyword hash...");
                }
                let (keyword_tables, keyword_func) =
                    syntaqlite_codegen::generate_keyword_hash(&extract_result)?;

                // Step 4: Copy all outputs to final destination
                if args.verbose {
                    eprintln!("Writing output files...");
                }
                let out = Path::new(&output_dir);
                fs::create_dir_all(out)
                    .map_err(|e| format!("Failed to create output directory: {}", e))?;

                let include_dir = Path::new(&output_dir)
                    .parent()
                    .unwrap_or(Path::new("."))
                    .join("include/syntaqlite");
                fs::create_dir_all(&include_dir)
                    .map_err(|e| format!("Failed to create include directory: {}", e))?;
                fs::copy(
                    temp_dir.path().join("parse.h"),
                    include_dir.join("tokens.h"),
                ).map_err(|e| format!("Failed to write tokens.h: {}", e))?;

                let raw_parse_c = fs::read_to_string(temp_dir.path().join("parse.c"))
                    .map_err(|e| format!("Failed to read parse.c: {}", e))?;
                let (parse_c, parse_data_h) =
                    syntaqlite_codegen::split_parse_c(&raw_parse_c)?;
                fs::write(out.join("sqlite_parse.c"), parse_c)
                    .map_err(|e| format!("Failed to write sqlite_parse.c: {}", e))?;
                fs::write(out.join("sqlite_parse_data.h"), parse_data_h)
                    .map_err(|e| format!("Failed to write sqlite_parse_data.h: {}", e))?;

                fs::write(out.join("sqlite_tokenize.c"), tokenize_content)
                    .map_err(|e| format!("Failed to write sqlite_tokenize.c: {}", e))?;

                fs::write(out.join("sqlite_keyword_tables.h"), keyword_tables)
                    .map_err(|e| format!("Failed to write sqlite_keyword_tables.h: {}", e))?;

                fs::write(out.join("sqlite_keyword.c"), keyword_func)
                    .map_err(|e| format!("Failed to write sqlite_keyword.c: {}", e))?;

                if args.verbose {
                    eprintln!("Code generation complete");
                }

                Ok(())
            }
            Command::Lemon { args: lemon_args } => {
                if args.verbose {
                    eprintln!("Running lemon with args: {:?}", lemon_args);
                }

                syntaqlite_codegen::lemon::run_lemon(&lemon_args);
            }
            Command::Mkkeyword {
                args: mkkeyword_args,
            } => {
                if args.verbose {
                    eprintln!("Running mkkeyword with args: {:?}", mkkeyword_args);
                }

                syntaqlite_codegen::mkkeyword::run_mkkeyword(&mkkeyword_args);
            }
        }
    })();

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
