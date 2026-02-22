// Copyright 2025 The syntaqlite Authors. All rights reserved.
// Licensed under the Apache License, Version 2.0.

use std::fmt::{Display, Write as _};

/// Shared single-buffer text writer with indentation tracking.
pub struct TextWriterCore {
    buffer: String,
    indent: usize,
    at_line_start: bool,
}

impl TextWriterCore {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            indent: 0,
            at_line_start: true,
        }
    }

    pub fn finish(self) -> String {
        self.buffer
    }

    pub fn newline(&mut self) {
        self.buffer.push('\n');
        self.at_line_start = true;
    }

    pub fn line(&mut self, text: &str) {
        self.write_indent();
        self.buffer.push_str(text);
        self.newline();
    }

    pub fn raw_line(&mut self, text: &str) {
        self.buffer.push_str(text);
        self.newline();
    }

    pub fn fragment(&mut self, fragment: &impl Display) {
        write!(self.buffer, "{}", fragment).unwrap();
        self.newline();
    }

    pub fn indent(&mut self) {
        self.indent += 1;
    }

    pub fn dedent(&mut self) {
        self.indent = self.indent.saturating_sub(1);
    }

    pub fn push_raw(&mut self, text: &str) {
        self.buffer.push_str(text);
        self.at_line_start = text.ends_with('\n');
    }

    pub fn write_indent(&mut self) {
        if self.at_line_start && self.indent > 0 {
            for _ in 0..self.indent {
                self.buffer.push_str("    ");
            }
        }
        self.at_line_start = false;
    }
}
