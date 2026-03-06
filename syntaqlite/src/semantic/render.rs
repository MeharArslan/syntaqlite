// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Rustc-style diagnostic rendering.

use std::io::{self, Write};

use super::diagnostics::{Diagnostic, Severity};

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

    fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1usize;
        let mut col = 1usize;
        for (i, ch) in self.source.char_indices() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 1;
            } else {
                col += 1;
            }
        }
        (line, col)
    }

    fn source_line_at(&self, offset: usize) -> &str {
        let start = self.source[..offset].rfind('\n').map_or(0, |i| i + 1);
        let end = self.source[offset..]
            .find('\n')
            .map_or(self.source.len(), |i| offset + i);
        &self.source[start..end]
    }

    /// Render a single diagnostic to `out`.
    pub fn render_diagnostic(
        &self,
        diag: &Diagnostic,
        out: &mut impl Write,
    ) -> io::Result<()> {
        let severity = match diag.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
            Severity::Hint => "hint",
        };
        let message = diag.message.to_string();
        let (line, col) = self.offset_to_line_col(diag.start_offset);
        let line_text = self.source_line_at(diag.start_offset);
        let gutter_width = line.to_string().len();

        writeln!(out, "{severity}: {message}")?;
        writeln!(out, "{:>gutter_width$}--> {}:{line}:{col}", " ", self.file)?;
        writeln!(out, "{:>gutter_width$} |", " ")?;
        writeln!(out, "{line} | {line_text}")?;

        let underline_len = if diag.end_offset > diag.start_offset {
            let line_end = diag.start_offset + (line_text.len().saturating_sub(col - 1));
            (diag.end_offset.min(line_end) - diag.start_offset).max(1)
        } else {
            1
        };
        writeln!(
            out,
            "{:>gutter_width$} | {:padding$}^{}",
            " ",
            "",
            "~".repeat(underline_len.saturating_sub(1)),
            padding = col - 1,
        )?;

        if let Some(ref help) = diag.help {
            writeln!(out, "{:>gutter_width$} = help: {help}", " ")?;
        }

        Ok(())
    }

    /// Render all diagnostics to `out`. Returns `true` if any had `Severity::Error`.
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
