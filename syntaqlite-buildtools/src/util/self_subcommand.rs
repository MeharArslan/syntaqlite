// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

pub fn self_subcommand(subcommand: &str) -> Result<std::process::Command, String> {
    let exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current executable: {e}"))?;
    let mut cmd = std::process::Command::new(&exe);
    cmd.arg(subcommand);
    Ok(cmd)
}
