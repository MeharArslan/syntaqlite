use sqlparser::dialect::SQLiteDialect;
use sqlparser::parser::Parser;
use std::io::Read;

fn main() {
    let path = std::env::args().nth(1);
    let sql = match path {
        Some(p) => std::fs::read_to_string(&p).unwrap(),
        None => {
            let mut s = String::new();
            std::io::stdin().read_to_string(&mut s).unwrap();
            s
        }
    };

    let mode = std::env::args().nth(2).unwrap_or_default();
    let dialect = SQLiteDialect {};

    match Parser::parse_sql(&dialect, &sql) {
        Ok(stmts) => {
            if mode == "--print" {
                for stmt in &stmts {
                    println!("{stmt};");
                }
            }
            eprintln!("{} statements parsed", stmts.len());
        }
        Err(e) => {
            eprintln!("Parse error: {e}");
            std::process::exit(1);
        }
    }
}
