use clap::{Parser, Subcommand};
use std::process;

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
    /// Code generation and extraction utilities
    Codegen {
        #[command(subcommand)]
        subcommand: CodegenCommand,
    },
    /// Run lemon parser generator on a grammar file
    Lemon {
        /// Input grammar file (e.g., parse.y)
        input: String,
        /// Output file path (base name without extension)
        #[arg(short, long)]
        output: Option<String>,
    },
}

#[derive(Subcommand)]
enum CodegenCommand {
    /// Extract grammar tokens from a Lemon grammar file
    ExtractGrammar {
        /// Input grammar file (e.g., parse.y)
        input: String,
        /// Output file path (prints to stdout if not specified)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Extract tokenizer code from SQLite's tokenize.c
    ExtractTokenizer {
        /// Path to SQLite's tokenize.c file
        #[arg(default_value = "third_party/src/sqlite/src/tokenize.c")]
        input: String,
        /// Output file path
        #[arg(short, long, default_value = "src/tokenizer/sqlite_tokenize.c")]
        output: String,
    },
}

fn main() {
    let args = Args::parse();

    let result = match args.command {
        Command::Codegen { subcommand } => match subcommand {
            CodegenCommand::ExtractGrammar { input, output } => {
                if args.verbose {
                    eprintln!("Reading grammar from: {}", input);
                }
                let res = syntaqlite_codegen::extract_grammar(&input, output.as_deref());
                if args.verbose && let Some(ref out) = output {
                    eprintln!("Wrote output to: {}", out);
                }
                res
            }
            CodegenCommand::ExtractTokenizer { input, output } => {
                if args.verbose {
                    eprintln!("Extracting tokenizer from: {}", input);
                }
                let res = syntaqlite_codegen::extract_tokenizer(&input, &output);
                if args.verbose {
                    eprintln!("Wrote tokenizer to: {}", output);
                }
                res
            }
        },
        Command::Lemon { input, output } => {
            if args.verbose {
                eprintln!("Running lemon on: {}", input);
            }
            let res = syntaqlite_codegen::lemon::run_lemon_on_file(&input, output.as_deref());
            if args.verbose && res.is_ok() {
                eprintln!("Lemon completed successfully");
            }
            res
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}
