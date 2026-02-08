use clap::Parser;
use std::process;

#[derive(Parser)]
#[command(name = "syntaqlite-codegen")]
#[command(about = "SQLite grammar extraction and code generation")]
struct Args {
    /// Input grammar file (e.g., parse.y)
    input: String,

    /// Output file path (prints to stdout if not specified)
    #[arg(short, long)]
    output: Option<String>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,
}

fn main() {
    let args = Args::parse();

    if args.verbose {
        eprintln!("Reading grammar from: {}", args.input);
    }

    if let Err(e) = syntaqlite_codegen::extract_grammar(&args.input, args.output.as_deref()) {
        eprintln!("Error: {}", e);
        process::exit(1);
    }

    if args.verbose && args.output.is_some() {
        eprintln!("Wrote output to: {}", args.output.unwrap());
    }
}
