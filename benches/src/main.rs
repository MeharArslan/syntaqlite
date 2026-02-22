// Profiling harness — run with: samply record cargo run -p benches --release
fn main() {
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

    let mut fmt = syntaqlite::Formatter::new().unwrap();
    for _ in 0..1000 {
        std::hint::black_box(fmt.format(std::hint::black_box(&sql)).unwrap());
    }
}
