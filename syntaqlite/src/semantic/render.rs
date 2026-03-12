// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Rustc-style diagnostic rendering.

use std::io::{self, Write};

use super::diagnostics::{Diagnostic, Severity};
use crate::util::render_source_error;

/// Renders diagnostics in rustc-style format to any `Write` target.
///
/// ```text
/// error: unknown table 'usr'
///  --> query.sql:1:15
///   |
/// 1 | SELECT id FROM usr WHERE id = 1
///   |               ^~~
///   = help: did you mean 'users'?
/// ```
pub struct DiagnosticRenderer<'a> {
    source: &'a str,
    file: &'a str,
}

impl<'a> DiagnosticRenderer<'a> {
    /// Create a renderer bound to a source string and display file label.
    pub fn new(source: &'a str, file: &'a str) -> Self {
        Self { source, file }
    }

    /// Render a single diagnostic to `out`.
    ///
    /// # Errors
    /// Returns `Err` if writing to `out` fails.
    pub fn render_diagnostic(&self, diag: &Diagnostic, out: &mut impl Write) -> io::Result<()> {
        let severity = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };
        let message = diag.message.to_string();
        let help = diag.help.as_ref().map(ToString::to_string);
        render_source_error(
            out,
            &crate::util::SourceError {
                source: self.source,
                file: self.file,
                severity,
                message: &message,
                start_offset: diag.start_offset,
                end_offset: diag.end_offset,
                help: help.as_deref(),
            },
        )
    }

    /// Render all diagnostics to `out`. Returns `true` if any had `Severity::Error`.
    ///
    /// # Errors
    /// Returns `Err` if writing to `out` fails.
    pub fn render_diagnostics(
        &self,
        diags: &[Diagnostic],
        out: &mut impl Write,
    ) -> io::Result<bool> {
        let mut has_errors = false;
        for d in diags {
            if d.severity == Severity::Error {
                has_errors = true;
            }
            self.render_diagnostic(d, out)?;
        }
        Ok(has_errors)
    }
}

/// Backward-compatible alias used by `syntaqlite-cli`.
pub type SourceContext<'a> = DiagnosticRenderer<'a>;
