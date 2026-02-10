use std::fs;
use std::io::{self, Read};

use clap::{Parser, Subcommand};
use syntaqlite_parser::dump;

#[derive(Parser)]
#[command(name = "syntaqlite", about = "Tools for SQLite SQL")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Parse SQL and print the AST
    Ast {
        /// SQL file to parse (reads stdin if omitted)
        file: Option<String>,
    },
}

fn read_source(file: Option<String>) -> Result<String, String> {
    match file {
        Some(path) => {
            fs::read_to_string(&path).map_err(|e| format!("{path}: {e}"))
        }
        None => {
            let mut buf = String::new();
            io::stdin()
                .read_to_string(&mut buf)
                .map_err(|e| format!("reading stdin: {e}"))?;
            Ok(buf)
        }
    }
}

fn cmd_ast(file: Option<String>) -> Result<(), String> {
    let source = read_source(file)?;
    let mut parser = syntaqlite_parser::Parser::new();
    let mut session = parser.parse(&source);
    let mut buf = String::new();
    let mut count = 0;

    while let Some(result) = session.next_statement() {
        let root_id = result.map_err(|e| format!("parse error: {e}"))?;
        if count > 0 {
            buf.push_str("----\n");
        }
        dump::dump_node(&session, root_id, &mut buf, 0);
        count += 1;
    }

    print!("{buf}");
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Ast { file } => cmd_ast(file),
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
