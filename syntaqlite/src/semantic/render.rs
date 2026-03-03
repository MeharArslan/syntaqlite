// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

//! Rustc-style diagnostic rendering.

use super::diagnostics::{Diagnostic, Severity};

/// A source string paired with a display label (file path or `"<stdin>"`).
///
/// Provides methods for rendering diagnostics in rustc-style format:
///
/// ```text
/// error: syntax error near 'include'
///  --> file.sql:1:1
///   |
/// 1 | include ;
///   | ^~~~~~~
/// ```
pub struct DiagnosticRenderer<'a> {
    source: &'a str,
    file: &'a str,
}

impl<'a> DiagnosticRenderer<'a> {
    pub fn new(source: &'a str, file: &'a str) -> Self {
        Self { source, file }
    }

    fn offset_to_line_col(&self, offset: usize) -> (usize, usize) {
        let mut line = 1;
        let mut col = 1;
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

    /// Render a single diagnostic to stderr.
    pub fn render_diagnostic(
        &self,
        severity: &str,
        message: &str,
        start_offset: usize,
        end_offset: usize,
        help: Option<&str>,
    ) {
        let (line, col) = self.offset_to_line_col(start_offset);
        let line_text = self.source_line_at(start_offset);
        let gutter_width = line.to_string().len();

        eprintln!("{severity}: {message}");
        eprintln!("{:>gutter_width$}--> {}:{line}:{col}", " ", self.file);
        eprintln!("{:>gutter_width$} |", " ");
        eprintln!("{line} | {line_text}");

        let underline_len = if end_offset > start_offset {
            let line_end_offset = start_offset + (line_text.len() - (col - 1));
            (end_offset.min(line_end_offset) - start_offset).max(1)
        } else {
            1
        };
        let padding = col - 1;
        eprintln!(
            "{:>gutter_width$} | {:padding$}^{}",
            " ",
            "",
            "~".repeat(underline_len.saturating_sub(1))
        );
        if let Some(help) = help {
            eprintln!("{:>gutter_width$} = help: {help}", " ");
        }
    }

    /// Render a slice of [`Diagnostic`] values to stderr and return `true`
    /// if any have [`Severity::Error`].
    pub fn render_diagnostics(&self, diags: &[Diagnostic]) -> bool {
        let mut has_errors = false;
        for d in diags {
            let severity = match d.severity {
                Severity::Error => {
                    has_errors = true;
                    "error"
                }
                Severity::Warning => "warning",
                Severity::Info => "info",
                Severity::Hint => "hint",
            };
            let message = d.message.to_string();
            let help = d.help.as_ref().map(|h| h.to_string());
            self.render_diagnostic(
                severity,
                &message,
                d.start_offset,
                d.end_offset,
                help.as_deref(),
            );
        }
        has_errors
    }
}

/// Backward-compatible alias.
pub type SourceContext<'a> = DiagnosticRenderer<'a>;
