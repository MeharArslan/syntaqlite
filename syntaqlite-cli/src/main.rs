use syntaqlite::ast::dump_node;
use syntaqlite::fmt::{ctx, dispatch, NODE_INFO};
use syntaqlite_cli::DialectCli;

fn main() {
    syntaqlite_cli::run(&DialectCli {
        name: "syntaqlite",
        create_parser: || syntaqlite_runtime::Parser::new(syntaqlite::sqlite_dialect()),
        dump_node,
        dispatch: dispatch(),
        ctx: ctx(),
        node_info: &NODE_INFO,
    });
}
