use fallible_iterator::FallibleIterator;
use sqlite3_parser::lexer::sql::Parser;
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

    let mut parser = Parser::new(sql.as_bytes());
    let mut count = 0;
    let mut errors = 0;
    loop {
        match parser.next() {
            Ok(Some(cmd)) => {
                count += 1;
                if mode == "--print" {
                    println!("{:?}", cmd);
                }
            }
            Ok(None) => break,
            Err(e) => {
                errors += 1;
                eprintln!("Parse error: {}", e);
            }
        }
    }
    if mode != "--print" {
        eprintln!("{} statements parsed, {} errors", count, errors);
    }
    if errors > 0 {
        std::process::exit(1);
    }
}
