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
        /// Grammar file to process (e.g., parse.y)
        grammar: String,

        /// Additional arguments to pass to lemon
        #[arg(last = true)]
        args: Vec<String>,
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

                if args.verbose && output.is_some() {
                    eprintln!("Wrote output to: {}", output.unwrap());
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

        Command::Lemon { grammar, args: lemon_args } => {
            if args.verbose {
                eprintln!("Running lemon on: {}", grammar);
            }

            let arg_refs: Vec<&str> = lemon_args.iter().map(|s| s.as_str()).collect();
            let res = syntaqlite_codegen::call::run_lemon(&grammar, &arg_refs);

            if args.verbose {
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
