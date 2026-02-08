//! C code extraction tailored for SQLite source code.
//!
//! Uses simple pattern matching specific to SQLite's coding conventions.
//! Not intended as a general-purpose C parser.

use std::fmt;

#[derive(Debug, Clone)]
pub struct CFunction {
    pub text: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct CStaticArray {
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct CDefines {
    pub text: String,
}

impl CFunction {
    pub(crate) fn new(text: String, name: String) -> Self {
        Self { text, name }
    }
}

/// Result of splitting source code by a function
pub struct SplitByFunction {
    pub before: String,
    pub function: CFunction,
    pub after: String,
}

impl CStaticArray {
    pub(crate) fn new(text: String) -> Self {
        Self { text }
    }
}

impl CDefines {
    pub(crate) fn new(text: String) -> Self {
        Self { text }
    }
}

impl fmt::Display for CFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl fmt::Display for CStaticArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

impl fmt::Display for CDefines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.text)
    }
}

pub struct CExtractor {
    lines: Vec<String>,
}

impl CExtractor {
    pub fn new(content: &str) -> Self {
        Self {
            lines: content.lines().map(|s| s.to_string()).collect(),
        }
    }

    pub fn extract_function(&self, name: &str) -> Result<CFunction, String> {
        // Reuse split_by_function and just return the function
        self.split_by_function(name).map(|split| split.function)
    }

    /// Split source code into: before function, function, after function
    pub fn split_by_function(&self, name: &str) -> Result<SplitByFunction, String> {
        let (start, end) = self.find_function_bounds(name)?;

        let before = self.lines[..start].join("\n");
        let text = self.lines[start..=end].join("\n");
        let after = if end + 1 < self.lines.len() {
            self.lines[end + 1..].join("\n")
        } else {
            String::new()
        };

        Ok(SplitByFunction {
            before,
            function: CFunction::new(text, name.to_string()),
            after,
        })
    }

    /// Find the start and end line indices of a function
    fn find_function_bounds(&self, name: &str) -> Result<(usize, usize), String> {
        let pattern = format!("{}(", name);

        for (i, line) in self.lines.iter().enumerate() {
            if Self::should_skip_line_for_function(line) || !line.contains(&pattern) {
                continue;
            }

            if self.is_function_definition(i) {
                let end = self.find_closing_brace(i)?;
                return Ok((i, end));
            }
        }
        Err(format!("Could not find function definition for '{}'", name))
    }

    pub fn extract_static_array(&self, name: &str) -> Result<CStaticArray, String> {
        let pattern = format!("{}[", name);

        for (i, line) in self.lines.iter().enumerate() {
            if !line.contains(&pattern) {
                continue;
            }

            // Check if this looks like an array declaration (not just a mention)
            // Array declarations should have ']' followed eventually by '=' or '{'
            if let Some(open_bracket_pos) = line.find(&pattern) {
                let after_name = &line[open_bracket_pos + pattern.len()..];

                // Find the closing bracket
                if let Some(close_bracket_pos) = after_name.find(']') {
                    let after_brackets = &after_name[close_bracket_pos + 1..].trim_start();

                    // Check if it's followed by '=' or '{' (array initialization)
                    if after_brackets.starts_with('=') || after_brackets.starts_with('{') {
                        let end = self.find_array_end(i)?;
                        let text = self.lines[i..=end].join("\n");
                        return Ok(CStaticArray::new(text));
                    }
                }
            }
        }
        Err(format!("Could not find array '{}'", name))
    }

    pub fn extract_specific_defines(&self, names: &[&str]) -> Result<CDefines, String> {
        let mut lines = Vec::new();

        for name in names {
            for line in &self.lines {
                if let Some(define_name) = Self::parse_define_name(line)
                    && define_name == *name
                {
                    lines.push(line.clone());
                    break;
                }
            }
        }

        if lines.is_empty() {
            return Err("Could not find any of the specified defines".to_string());
        }

        Ok(CDefines::new(lines.join("\n")))
    }

    /// Parses a #define line and returns the macro name if found
    fn parse_define_name(line: &str) -> Option<&str> {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("#") {
            return None;
        }

        let after_hash = trimmed[1..].trim_start();
        if !after_hash.starts_with("define") {
            return None;
        }

        let after_define = after_hash[6..].trim_start();
        // Extract the macro name (up to whitespace or parenthesis)
        let name_end = after_define
            .find(|c: char| c.is_whitespace() || c == '(')
            .unwrap_or(after_define.len());

        Some(&after_define[..name_end])
    }

    /// Check if a line should be skipped when looking for function definitions
    fn should_skip_line_for_function(line: &str) -> bool {
        let trimmed = line.trim_start();
        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            return true;
        }
        // Skip indented lines (not at column 0)
        line.starts_with(' ') || line.starts_with('\t')
    }

    fn is_function_definition(&self, start: usize) -> bool {
        for i in start..self.lines.len().min(start + 10) {
            let line = &self.lines[i];
            if line.contains('{') {
                return true;
            }
            if line.contains(';') {
                return false;
            }
        }
        false
    }

    fn find_closing_brace(&self, start: usize) -> Result<usize, String> {
        let mut brace_count = 0;
        for i in start..self.lines.len() {
            for ch in self.lines[i].chars() {
                match ch {
                    '{' => brace_count += 1,
                    '}' => {
                        brace_count -= 1;
                        if brace_count == 0 {
                            return Ok(i);
                        }
                    }
                    _ => {}
                }
            }
        }
        Err("Could not find closing brace".to_string())
    }

    fn find_array_end(&self, start: usize) -> Result<usize, String> {
        for i in start..self.lines.len() {
            if self.lines[i].contains("};") {
                return Ok(i);
            }
        }
        Err("Could not find array end".to_string())
    }
}
