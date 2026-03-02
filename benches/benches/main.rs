// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};

// ── SQL fixtures ────────────────────────────────────────────────────────

/// ~45 B — single simple SELECT
const SMALL_SQL: &str = "SELECT id, name FROM users WHERE active = 1;";

/// ~430 B — multi-join query with GROUP BY/HAVING/ORDER/LIMIT
const MEDIUM_SQL: &str = "\
SELECT u.id, u.name, u.email, o.order_id, o.total, p.product_name
FROM users u
INNER JOIN orders o ON o.user_id = u.id
LEFT JOIN order_items oi ON oi.order_id = o.order_id
LEFT JOIN products p ON p.product_id = oi.product_id
WHERE u.active = 1
  AND o.created_at > '2024-01-01'
  AND o.total > 100.00
GROUP BY u.id, u.name, u.email, o.order_id, o.total, p.product_name
HAVING COUNT(oi.item_id) > 2
ORDER BY o.total DESC
LIMIT 50 OFFSET 10;";

/// ~50 KB — 500 mixed statements (INSERTs, SELECTs, CREATEs, UPDATEs)
fn large_sql() -> String {
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

struct Fixture {
    name: &'static str,
    sql: String,
}

fn fixtures() -> Vec<Fixture> {
    vec![
        Fixture {
            name: "small",
            sql: SMALL_SQL.to_string(),
        },
        Fixture {
            name: "medium",
            sql: MEDIUM_SQL.to_string(),
        },
        Fixture {
            name: "large",
            sql: large_sql(),
        },
    ]
}

// ── Tokenizer benchmarks ────────────────────────────────────────────────

fn bench_tokenizer(c: &mut Criterion) {
    let mut group = c.benchmark_group("tokenizer");
    let fixtures = fixtures();

    for f in &fixtures {
        group.throughput(Throughput::Bytes(f.sql.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(f.name), &f.sql, |b, sql| {
            let mut tok = syntaqlite::ext::RawTokenizer::new(syntaqlite::dialect::sqlite());
            b.iter(|| {
                let cursor = tok.tokenize(black_box(sql));
                for item in cursor {
                    black_box(item);
                }
            });
        });
    }

    group.finish();
}

// ── Parser benchmarks ───────────────────────────────────────────────────

fn bench_parser(c: &mut Criterion) {
    let mut group = c.benchmark_group("parser");
    let fixtures = fixtures();

    for f in &fixtures {
        group.throughput(Throughput::Bytes(f.sql.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(f.name), &f.sql, |b, sql| {
            let mut parser = syntaqlite::ext::RawParser::new(syntaqlite::dialect::sqlite());
            b.iter(|| {
                let mut cursor = parser.parse(black_box(sql));
                while let Some(stmt) = cursor.next_statement() {
                    black_box(stmt.ok());
                }
            });
        });
    }

    group.finish();
}

// ── Formatter benchmarks ────────────────────────────────────────────────

fn bench_formatter(c: &mut Criterion) {
    let mut group = c.benchmark_group("formatter");
    let fixtures = fixtures();

    for f in &fixtures {
        group.throughput(Throughput::Bytes(f.sql.len() as u64));
        group.bench_with_input(BenchmarkId::from_parameter(f.name), &f.sql, |b, sql| {
            let mut fmt = syntaqlite::Formatter::new();
            b.iter(|| {
                black_box(fmt.format(black_box(sql)).unwrap());
            });
        });
    }

    group.finish();
}

// ── LSP Host benchmarks ────────────────────────────────────────────────

fn bench_lsp_host(c: &mut Criterion) {
    let mut group = c.benchmark_group("lsp_host");
    let fixtures = fixtures();

    for f in &fixtures {
        group.throughput(Throughput::Bytes(f.sql.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("open_analyze", f.name),
            &f.sql,
            |b, sql| {
                b.iter(|| {
                    let mut host = syntaqlite::lsp::AnalysisHost::new();
                    host.open_document("test://file.sql", 1, sql.clone());
                    black_box(host.diagnostics("test://file.sql"));
                    black_box(host.semantic_tokens_encoded("test://file.sql", None));
                });
            },
        );
    }

    for f in &fixtures {
        group.throughput(Throughput::Bytes(f.sql.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("update_cycle", f.name),
            &f.sql,
            |b, sql| {
                let mut host = syntaqlite::lsp::AnalysisHost::new();
                host.open_document("test://file.sql", 1, sql.clone());
                host.diagnostics("test://file.sql");
                let mut version = 2;
                b.iter(|| {
                    host.update_document("test://file.sql", version, sql.clone());
                    version += 1;
                    black_box(host.diagnostics("test://file.sql"));
                    black_box(host.semantic_tokens_encoded("test://file.sql", None));
                });
            },
        );
    }

    group.finish();
}

// ── Criterion setup ─────────────────────────────────────────────────────

criterion_group!(
    benches,
    bench_tokenizer,
    bench_parser,
    bench_formatter,
    bench_lsp_host,
);
criterion_main!(benches);
