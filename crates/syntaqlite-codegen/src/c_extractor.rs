//! C code extraction tailored for SQLite source code.
//!
//! Uses simple pattern matching specific to SQLite's coding conventions.
//! Not intended as a general-purpose C parser.

use std::fmt;

#[derive(Debug, Clone)]
pub struct CFunction {
    pub(crate) text: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub struct CStaticArray {
    pub(crate) text: String,
    pub(crate) name: String,
}

#[derive(Debug, Clone)]
pub struct CDefines {
    pub(crate) text: String,
}

impl CFunction {
    pub(crate) fn new(text: String, name: String) -> Self {
        Self { text, name }
    }
}

impl CStaticArray {
    pub(crate) fn new(text: String, name: String) -> Self {
        Self { text, name }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for CStaticArray {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl CDefines {
    pub fn new(text: String) -> Self {
        Self { text }
    }

    pub fn text(&self) -> &str {
        &self.text
    }
}

impl fmt::Display for CDefines {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
    }
}

impl fmt::Display for CFunction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.text)
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
        let pattern = format!("{}(", name);

        for (i, line) in self.lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
                continue;
            }
            if line.starts_with(' ') || line.starts_with('\t') {
                continue;
            }
            if !line.contains(&pattern) {
                continue;
            }

            if self.is_function_definition(i) {
                let end = self.find_closing_brace(i)?;
                let text = self.lines[i..=end].join("\n");
                return Ok(CFunction::new(text, name.to_string()));
            }
        }
        Err(format!("Could not find function definition for '{}'", name))
    }

    pub fn extract_static_array(&self, name: &str) -> Result<CStaticArray, String> {
        let pattern = format!("{}[]", name);

        for (i, line) in self.lines.iter().enumerate() {
            if !line.contains("static") || !line.contains(&pattern) {
                continue;
            }

            let end = self.find_array_end(i)?;
            let text = self.lines[i..=end].join("\n");
            return Ok(CStaticArray::new(text, name.to_string()));
        }
        Err(format!("Could not find static array '{}'", name))
    }

    pub fn extract_defines_with_prefix(&self, prefix: &str) -> Result<CDefines, String> {
        let mut start = None;
        let mut end = None;

        for (i, line) in self.lines.iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("#define") {
                let after_define = &trimmed[7..].trim_start();
                if after_define.starts_with(prefix) {
                    if start.is_none() {
                        start = Some(i);
                    }
                    end = Some(i);
                } else if start.is_some() {
                    break;
                }
            } else if start.is_some() && !trimmed.is_empty() && !trimmed.starts_with("/*") {
                break;
            }
        }

        let start = start.ok_or_else(|| format!("Could not find #defines with prefix '{}'", prefix))?;
        let end = end.unwrap();

        let text = self.lines[start..=end].join("\n");
        Ok(CDefines::new(text))
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
