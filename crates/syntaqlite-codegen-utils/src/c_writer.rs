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

    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
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

    /// Add a raw line without indentation
    pub fn raw(&mut self, text: &str) -> &mut Self {
        self.buffer.push_str(text);
        self.newline()
    }

    /// Write a fragment (anything that implements Display)
    pub fn fragment(&mut self, fragment: &impl std::fmt::Display) -> &mut Self {
        write!(self.buffer, "{}", fragment).unwrap();
        self.newline()
    }

    /// Write text without newline (will be indented if at line start)
    pub fn write(&mut self, text: &str) -> &mut Self {
        self.write_indent();
        self.buffer.push_str(text);
        self
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

    /// Start a typedef struct
    pub fn typedef_struct_start(&mut self, name: &str) {
        self.write_indent();
        writeln!(self.buffer, "typedef struct {} {{", name).unwrap();
        self.indent();
        self.at_line_start = true;
    }

    /// Add a struct field
    pub fn struct_field(&mut self, type_name: &str, field_name: &str, comment: Option<&str>) {
        self.write_indent();
        write!(self.buffer, "{} {};", type_name, field_name).unwrap();
        if let Some(cmt) = comment {
            write!(self.buffer, "  // {}", cmt).unwrap();
        }
        self.newline();
    }

    /// End a typedef struct
    pub fn typedef_struct_end(&mut self, name: &str) {
        self.dedent();
        self.write_indent();
        writeln!(self.buffer, "}} {};", name).unwrap();
        self.at_line_start = true;
    }

    /// Emit a typedef union for flags
    pub fn typedef_flags_union(&mut self, name: &str, flags: &[(&str, u8)]) {
        self.write_indent();
        writeln!(self.buffer, "typedef union {} {{", name).unwrap();
        self.indent();

        self.write_indent();
        self.buffer.push_str("uint8_t raw;\n");

        self.write_indent();
        self.buffer.push_str("struct {\n");
        self.indent();

        let mut sorted_flags = flags.to_vec();
        sorted_flags.sort_by_key(|(_, val)| *val);

        let mut next_bit = 0;
        for (flag_name, value) in sorted_flags {
            let bit_pos = value.trailing_zeros() as usize;

            if bit_pos > next_bit {
                self.write_indent();
                writeln!(self.buffer, "uint8_t : {};", bit_pos - next_bit).unwrap();
            }

            self.write_indent();
            writeln!(self.buffer, "uint8_t {} : 1;", flag_name.to_lowercase()).unwrap();
            next_bit = bit_pos + 1;
        }

        self.dedent();
        self.write_indent();
        self.buffer.push_str("};\n");

        self.dedent();
        self.write_indent();
        writeln!(self.buffer, "}} {};", name).unwrap();
        self.at_line_start = true;
    }

    /// Emit a function signature
    pub fn function_sig(
        &mut self,
        return_type: &str,
        name: &str,
        params: &[(&str, &str)],
        is_inline: bool,
        is_static: bool,
    ) {
        self.write_indent();

        if is_static {
            self.buffer.push_str("static ");
        }
        if is_inline {
            self.buffer.push_str("inline ");
        }

        write!(self.buffer, "{} {}(", return_type, name).unwrap();

        if params.is_empty() {
            self.buffer.push_str("void");
        } else {
            let params_str: String = params
                .iter()
                .map(|(ty, pname)| format!("{} {}", ty, pname))
                .collect::<Vec<_>>()
                .join(", ");

            // Check if we need to wrap (from current position)
            let current_line_len = self.buffer.len() - self.buffer.rfind('\n').unwrap_or(0);
            if current_line_len + params_str.len() > 80 {
                self.newline();
                self.indent();
                for (i, (ty, pname)) in params.iter().enumerate() {
                    self.write_indent();
                    write!(self.buffer, "{} {}", ty, pname).unwrap();
                    if i < params.len() - 1 {
                        self.buffer.push(',');
                    }
                    self.newline();
                }
                self.dedent();
                self.write_indent();
                self.buffer.push(')');
            } else {
                self.buffer.push_str(&params_str);
                self.buffer.push(')');
            }
        }

        // Don't add newline - caller may want to add "{" or ";"
    }

    /// Start a function body (adds " {" and increases indent)
    pub fn block_start(&mut self) {
        self.buffer.push_str(" {\n");
        self.indent();
        self.at_line_start = true;
    }

    /// End a function body
    pub fn block_end(&mut self) {
        self.dedent();
        self.write_indent();
        self.buffer.push_str("}\n");
        self.at_line_start = true;
    }

    /// Emit a return statement
    pub fn return_stmt(&mut self, expr: &str) {
        self.write_indent();
        writeln!(self.buffer, "return {};", expr).unwrap();
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
    fn test_struct() {
        let mut w = CWriter::new();
        w.typedef_struct_start("Point");
        w.struct_field("int", "x", None);
        w.struct_field("int", "y", Some("Y coordinate"));
        w.typedef_struct_end("Point");

        let output = w.finish();
        assert!(output.contains("typedef struct Point {"));
        assert!(output.contains("int x;"));
        assert!(output.contains("int y;  // Y coordinate"));
        assert!(output.contains("} Point;"));
    }

    #[test]
    fn test_flags_union() {
        let mut w = CWriter::new();
        w.typedef_flags_union("MyFlags", &[("FOO", 0x01), ("BAR", 0x02), ("BAZ", 0x04)]);

        let output = w.finish();
        assert!(output.contains("typedef union MyFlags {"));
        assert!(output.contains("uint8_t raw;"));
        assert!(output.contains("uint8_t foo : 1;"));
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
