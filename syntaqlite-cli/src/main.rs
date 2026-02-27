// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

fn main() {
    #[cfg(feature = "builtin-sqlite")]
    {
        syntaqlite_cli::run("syntaqlite", Some(syntaqlite::sqlite::low_level::dialect()));
    }
    #[cfg(not(feature = "builtin-sqlite"))]
    {
        syntaqlite_cli::run("syntaqlite", None);
    }
}
