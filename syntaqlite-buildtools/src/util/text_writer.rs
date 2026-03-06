// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fmt::{Display, Write as _};

/// Shared single-buffer text writer with indentation tracking.
pub(crate) struct TextWriterCore {
    buffer: String,
    indent: usize,
    at_line_start: bool,
}

impl TextWriterCore {
    pub(crate) const fn new() -> Self {
        Self {
            buffer: String::new(),
            indent: 0,
            at_line_start: true,
        }
    }

    pub(crate) fn finish(self) -> String {
        self.buffer
    }

    pub(crate) fn newline(&mut self) {
        self.buffer.push('\n');
        self.at_line_start = true;
    }

    pub(crate) fn line(&mut self, text: &str) {
        self.write_indent();
        self.buffer.push_str(text);
        self.newline();
    }

    pub(crate) fn raw_line(&mut self, text: &str) {
        self.buffer.push_str(text);
        self.newline();
    }

    pub(crate) fn fragment(&mut self, fragment: &impl Display) {
        write!(self.buffer, "{fragment}").expect("write to String cannot fail");
        self.newline();
    }

    pub(crate) const fn indent(&mut self) {
        self.indent += 1;
    }

    pub(crate) const fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    pub(crate) fn push_raw(&mut self, text: &str) {
        self.buffer.push_str(text);
        self.at_line_start = text.ends_with('\n');
    }

    pub(crate) fn write_indent(&mut self) {
        if self.at_line_start && self.indent > 0 {
            for _ in 0..self.indent {
                self.buffer.push_str("    ");
            }
        }
        self.at_line_start = false;
    }

    /// Write an arbitrary string with indent-aware newline handling.
    ///
    /// Used to implement `fmt::Write` on writer wrappers: each `\n` triggers
    /// indentation for the next non-empty chunk, matching `line()`'s behaviour.
    pub(crate) fn write_fmt_str(&mut self, s: &str) {
        let mut chunks = s.split('\n');
        if let Some(first) = chunks.next() {
            if !first.is_empty() {
                self.write_indent();
                self.buffer.push_str(first);
                self.at_line_start = false;
            }
        }
        for chunk in chunks {
            self.newline();
            if !chunk.is_empty() {
                self.write_indent();
                self.buffer.push_str(chunk);
                self.at_line_start = false;
            }
        }
    }
}
