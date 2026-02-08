use clap::{Parser, Subcommand};
use std::process;

// Embed the SQLite grammar file and template
const PARSE_Y: &[u8] = include_bytes!("../sqlite/parse.y");
const LEMPAR_C: &[u8] = include_bytes!("../sqlite/lempar.c");

#[derive(Parser)]
#[command(name = "syntaqlite-codegen")]
#[command(about = "SQLite grammar extraction and code generation")]
struct Args {
    #[command(subcommand)]
    command: Command,
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Command {
    /// Generate all code from SQLite sources (parser, tokenizer, etc.)
    Codegen {
        /// Output directory for generated files
        #[arg(short, long, default_value = "gen")]
        output: String,
    },
    /// Run lemon parser generator (pass-through to lemon binary)
    Lemon {
        /// Arguments to pass through to lemon
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

fn main() {
    let args = Args::parse();

    let result = (|| -> Result<(), String> {
        match args.command {
            Command::Codegen { output } => {
                if args.verbose {
                    eprintln!("=== Starting Code Generation ===");
                    eprintln!("Output directory: {}", output);
                }

                // Step 1: Generate parser
                if args.verbose {
                    eprintln!("\n=== Generating Parser ===");
                }
                let parser_dir = syntaqlite_codegen::generate_parser(PARSE_Y, LEMPAR_C, Some(&output))?;
                if args.verbose {
                    eprintln!("Generated parser files in: {}", parser_dir.display());
                }

                if args.verbose {
                    eprintln!("\n=== Code Generation Complete ===");
                }

                Ok(())
            }
            Command::Lemon { args: lemon_args } => {
                if args.verbose {
                    eprintln!("Running lemon with args: {:?}", lemon_args);
                }

                // This function never returns - it always exits the process
                syntaqlite_codegen::lemon::run_lemon(&lemon_args);
            }
        }
    })();

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
