use syntaqlite::{DialectTypes, Sqlite};
use syntaqlite_cli::DialectCli;

fn main() {
    syntaqlite_cli::run(&DialectCli {
        name: "syntaqlite",
        info: Sqlite::info(),
    });
}
