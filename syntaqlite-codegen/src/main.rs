use clap::{Parser, Subcommand};
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
        parse_y: String,
        #[arg(long, required = true)]
        tokenize_c: String,
        #[arg(long, default_value = "syntaqlite-parser/csrc/sqlite_tokenize.c")]
        tokenize_output: String,
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
            Command::Codegen { parse_y, tokenize_c, tokenize_output } => {
                if args.verbose {
                    eprintln!("Generating parser...");
                }
                let tokens_output = "syntaqlite-parser/csrc/sqlite_tokens.h";
                syntaqlite_codegen::generate_parser(&parse_y, None, Some(tokens_output))?;

                if args.verbose {
                    eprintln!("Extracting tokenizer...");
                }
                let extract_result = syntaqlite_codegen::extract_tokenizer(&tokenize_c, &tokenize_output)?;

                if args.verbose {
                    eprintln!("Generating keyword hash...");
                }
                let keyword_output = "syntaqlite-parser/csrc/sqlite_keyword.c";
                syntaqlite_codegen::generate_keyword_hash(keyword_output, &extract_result)?;

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
            Command::Mkkeyword { args: mkkeyword_args } => {
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
