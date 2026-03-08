// Profiling harness — run with:
//   samply record --save-only -o tok.json.gz cargo run -p benches --release -- tokenizer
//   samply record --save-only -o parse.json.gz cargo run -p benches --release -- parser
//   samply record --save-only -o fmt.json.gz cargo run -p benches --release -- formatter
#![expect(clippy::unwrap_used)]

use syntaqlite::ParseOutcome;

fn build_sql() -> String {
    let mut sql = String::with_capacity(60_000);
    for i in 0..500 {
        match i % 4 {
            0 => sql.push_str(&format!(
                "INSERT INTO metrics (ts, sensor_id, value, label) VALUES ('2024-01-{:02}', {}, {:.2}, 'sensor_{}');\n",
                (i % 28) + 1, i, i as f64 * 1.5, i
            )),
            1 => sql.push_str(&format!(
                "SELECT m.ts, m.value, s.name FROM metrics m JOIN sensors s ON s.id = m.sensor_id WHERE m.sensor_id = {} AND m.value > {:.1} ORDER BY m.ts;\n",
                i, i as f64 * 0.5
            )),
            2 => sql.push_str(&format!(
                "UPDATE metrics SET value = value + 1.0, label = 'updated_{}' WHERE sensor_id = {} AND ts > '2024-01-01';\n",
                i, i
            )),
            _ => sql.push_str(&format!(
                "DELETE FROM metrics WHERE sensor_id = {} AND value < {:.1};\n",
                i, i as f64 * 0.1
            )),
        }
    }
    sql
}

fn profile_tokenizer(sql: &str) {
    let tok = syntaqlite::Tokenizer::new();
    for _ in 0..5000 {
        for item in tok.tokenize(std::hint::black_box(sql)) {
            std::hint::black_box(item);
        }
    }
}

fn profile_parser(sql: &str) {
    let parser = syntaqlite::Parser::new();
    for _ in 0..5000 {
        let mut session = parser.parse(std::hint::black_box(sql));
        loop {
            match session.next() {
                ParseOutcome::Done => break,
                ParseOutcome::Ok(stmt) => { std::hint::black_box(stmt); }
                ParseOutcome::Err(err) => { std::hint::black_box(err); }
            }
        }
    }
}

fn profile_formatter(sql: &str) {
    let mut fmt = syntaqlite::Formatter::new();
    for _ in 0..1000 {
        std::hint::black_box(fmt.format(std::hint::black_box(sql)).unwrap());
    }
}

fn main() {
    let sql = build_sql();
    let mode = std::env::args().nth(1).unwrap_or_else(|| "formatter".into());
    match mode.as_str() {
        "tokenizer" | "tok" => profile_tokenizer(&sql),
        "parser" | "parse" => profile_parser(&sql),
        "formatter" | "fmt" => profile_formatter(&sql),
        other => eprintln!("unknown mode: {other} (use tokenizer, parser, or formatter)"),
    }
}
