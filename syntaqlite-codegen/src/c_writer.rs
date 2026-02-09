//! Minimal C code writer for syntaqlite codegen
//!
//! Focused on emitting the specific C patterns needed for AST node generation.
//! Uses a single String buffer to minimize allocations.

use std::fmt::Write as _;

/// Simple C code writer with single-buffer output and indentation tracking
pub struct CWriter {
    buffer: String,
    indent: usize,
    at_line_start: bool,
}

impl CWriter {
    // ========== Public API ==========

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

    // Basic output

    pub fn newline(&mut self) -> &mut Self {
        self.buffer.push('\n');
        self.at_line_start = true;
        self
    }

    pub fn line(&mut self, text: &str) -> &mut Self {
        self.write_indent();
        self.buffer.push_str(text);
        self.newline()
    }

    /// Write a fragment (anything that implements Display)
    pub fn fragment(&mut self, fragment: &impl std::fmt::Display) -> &mut Self {
        write!(self.buffer, "{}", fragment).unwrap();
        self.newline()
    }

    /// Start a block (increase indentation)
    pub fn indent(&mut self) -> &mut Self {
        self.indent += 1;
        self
    }

    /// End a block (decrease indentation)
    pub fn dedent(&mut self) -> &mut Self {
        self.indent = self.indent.saturating_sub(1);
        self
    }

    // Preprocessor directives

    pub fn include_system(&mut self, header: &str) -> &mut Self {
        writeln!(self.buffer, "#include <{}>", header).unwrap();
        self.at_line_start = true;
        self
    }

    pub fn include_local(&mut self, header: &str) -> &mut Self {
        writeln!(self.buffer, "#include \"{}\"", header).unwrap();
        self.at_line_start = true;
        self
    }

    /// Emit header guard start
    pub fn header_guard_start(&mut self, guard_name: &str) {
        write!(
            self.buffer,
            "#ifndef {}\n#define {}\n\n",
            guard_name, guard_name
        )
        .unwrap();
        self.at_line_start = true;
    }

    /// Emit header guard end
    pub fn header_guard_end(&mut self, guard_name: &str) {
        writeln!(self.buffer, "#endif  // {}", guard_name).unwrap();
        self.at_line_start = true;
    }

    /// Emit file header comment
    pub fn file_header(&mut self, source: &str, generator: &str) {
        write!(
            self.buffer,
            "// Generated from {} by {}\n// DO NOT EDIT - changes will be overwritten\n\n",
            source, generator
        )
        .unwrap();
        self.at_line_start = true;
    }

    /// Start extern "C" block
    pub fn extern_c_start(&mut self) {
        self.buffer
            .push_str("#ifdef __cplusplus\nextern \"C\" {\n#endif\n\n");
        self.at_line_start = true;
    }

    /// End extern "C" block
    pub fn extern_c_end(&mut self) {
        self.buffer.push_str("#ifdef __cplusplus\n}\n#endif\n");
        self.at_line_start = true;
    }

    /// Emit a typedef enum
    pub fn typedef_enum(&mut self, name: &str, variants: &[(&str, Option<i32>)]) {
        self.write_indent();
        writeln!(self.buffer, "typedef enum {} {{", name).unwrap();
        self.indent();

        for (i, (variant, value)) in variants.iter().enumerate() {
            self.write_indent();
            self.buffer.push_str(variant);
            if let Some(val) = value {
                write!(self.buffer, " = {}", val).unwrap();
            }
            if i < variants.len() - 1 {
                self.buffer.push(',');
            }
            self.newline();
        }

        self.dedent();
        self.write_indent();
        writeln!(self.buffer, "}} {};", name).unwrap();
        self.at_line_start = true;
    }

    /// Emit a comment line
    pub fn comment(&mut self, text: &str) -> &mut Self {
        self.write_indent();
        writeln!(self.buffer, "// {}", text).unwrap();
        self.at_line_start = true;
        self
    }

    /// Emit a section header comment
    pub fn section(&mut self, title: &str) -> &mut Self {
        self.write_indent();
        write!(self.buffer, "// ============ {} ============\n\n", title).unwrap();
        self.at_line_start = true;
        self
    }

    // ========== Private helpers ==========

    fn write_indent(&mut self) {
        if self.at_line_start && self.indent > 0 {
            for _ in 0..self.indent {
                self.buffer.push_str("    ");
            }
        }
        self.at_line_start = false;
    }
}

impl Default for CWriter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_enum() {
        let mut w = CWriter::new();
        w.typedef_enum(
            "Color",
            &[("RED", Some(0)), ("GREEN", Some(1)), ("BLUE", Some(2))],
        );

        let output = w.finish();
        assert!(output.contains("typedef enum Color {"));
        assert!(output.contains("RED = 0,"));
        assert!(output.contains("BLUE = 2"));
    }

    #[test]
    fn test_header_guard() {
        let mut w = CWriter::new();
        w.header_guard_start("MY_HEADER_H");
        w.line("// content");
        w.header_guard_end("MY_HEADER_H");

        let output = w.finish();
        assert!(output.contains("#ifndef MY_HEADER_H"));
        assert!(output.contains("#define MY_HEADER_H"));
        assert!(output.contains("#endif  // MY_HEADER_H"));
    }
}
