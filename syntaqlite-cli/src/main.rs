// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! `syntaqlite` command-line interface.

fn main() {
    #[cfg(feature = "builtin-sqlite")]
    syntaqlite_cli::run("syntaqlite", Some(syntaqlite::sqlite_dialect().into()));
}
