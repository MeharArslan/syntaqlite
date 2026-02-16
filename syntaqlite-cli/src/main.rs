use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use syntaqlite_fmt::generated::fmt_ops::{CTX, DISPATCH};
use syntaqlite_fmt::{format_node, render, DocArena, FormatConfig, KeywordCase};
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
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
    },
    /// Format SQL
    Fmt {
        /// SQL files or glob patterns (reads stdin if omitted)
        files: Vec<String>,
        /// Maximum line width
        #[arg(short = 'w', long, default_value_t = 80)]
        line_width: usize,
        /// Keyword casing
        #[arg(short = 'k', long, value_enum, default_value_t = CasingArg::Upper)]
        keyword_case: CasingArg,
        /// Write formatted output back to file(s) in place
        #[arg(short = 'i', long)]
        in_place: bool,
        /// Append semicolons after each statement
        #[arg(long)]
        semicolons: bool,
    },
}

#[derive(Clone, Copy, ValueEnum)]
enum CasingArg {
    Preserve,
    Upper,
    Lower,
}

/// Expand a list of file paths / glob patterns into concrete paths.
/// Returns an empty vec when the input is empty (meaning: read stdin).
fn expand_paths(patterns: &[String]) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for pat in patterns {
        let matches: Vec<_> = glob::glob(pat)
            .map_err(|e| format!("bad glob pattern {pat:?}: {e}"))?
            .collect();
        if matches.is_empty() {
            return Err(format!("no files matched: {pat}"));
        }
        for entry in matches {
            let path = entry.map_err(|e| format!("glob error: {e}"))?;
            if path.is_file() {
                out.push(path);
            }
        }
    }
    Ok(out)
}

fn format_source(source: &str, config: &FormatConfig, semicolons: bool) -> Result<String, String> {
    let mut parser = syntaqlite_parser::Parser::new();
    let mut session = parser.parse(source);
    let mut out = String::new();
    let mut first = true;

    while let Some(result) = session.next_statement() {
        let root_id = result.map_err(|e| format!("parse error: {e}"))?;
        if !first {
            if semicolons {
                out.push(';');
            }
            out.push_str("\n\n");
        }
        let mut arena = DocArena::new();
        let doc = format_node(&DISPATCH, &CTX, &session, root_id, &mut arena);
        out.push_str(&render(&arena, doc, config));
        first = false;
    }

    if !first {
        if semicolons {
            out.push(';');
        }
        out.push('\n');
    }
    Ok(out)
}

fn cmd_ast(files: Vec<String>) -> Result<(), String> {
    let paths = expand_paths(&files)?;

    if paths.is_empty() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("reading stdin: {e}"))?;
        return cmd_ast_source(&buf);
    }

    for path in &paths {
        let source =
            fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if paths.len() > 1 {
            println!("==> {} <==", path.display());
        }
        cmd_ast_source(&source)?;
    }
    Ok(())
}

fn cmd_ast_source(source: &str) -> Result<(), String> {
    let mut parser = syntaqlite_parser::Parser::new();
    let mut session = parser.parse(source);
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

fn cmd_fmt(
    files: Vec<String>,
    config: &FormatConfig,
    in_place: bool,
    semicolons: bool,
) -> Result<(), String> {
    let paths = expand_paths(&files)?;

    if paths.is_empty() {
        if in_place {
            return Err("--in-place requires file arguments".to_string());
        }
        let mut source = String::new();
        io::stdin()
            .read_to_string(&mut source)
            .map_err(|e| format!("reading stdin: {e}"))?;
        let out = format_source(&source, config, semicolons)?;
        print!("{out}");
        return Ok(());
    }

    let mut errors = Vec::new();
    for path in &paths {
        let source =
            fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        match format_source(&source, config, semicolons) {
            Ok(out) => {
                if in_place {
                    if out != source {
                        fs::write(path, &out)
                            .map_err(|e| format!("{}: {e}", path.display()))?;
                        eprintln!("formatted {}", path.display());
                    }
                } else {
                    if paths.len() > 1 {
                        println!("==> {} <==", path.display());
                    }
                    print!("{out}");
                }
            }
            Err(e) => {
                errors.push(format!("{}: {e}", path.display()));
            }
        }
    }

    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Command::Ast { files } => cmd_ast(files),
        Command::Fmt {
            files,
            line_width,
            keyword_case,
            in_place,
            semicolons,
        } => {
            let config = FormatConfig {
                line_width,
                keyword_case: match keyword_case {
                    CasingArg::Preserve => KeywordCase::Preserve,
                    CasingArg::Upper => KeywordCase::Upper,
                    CasingArg::Lower => KeywordCase::Lower,
                },
                ..Default::default()
            };
            cmd_fmt(files, &config, in_place, semicolons)
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
