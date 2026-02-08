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
        self.split_by_function(name).map(|split| split.function)
    }

    /// Split source code into: before function, function, after function.
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

    pub fn extract_static_array(&self, name: &str) -> Result<CStaticArray, String> {
        let pattern = format!("{}[", name);

        for (i, line) in self.lines.iter().enumerate() {
            if !line.contains(&pattern) {
                continue;
            }

            if let Some(open_bracket_pos) = line.find(&pattern) {
                let after_name = &line[open_bracket_pos + pattern.len()..];

                if let Some(close_bracket_pos) = after_name.find(']') {
                    let after_brackets = &after_name[close_bracket_pos + 1..].trim_start();

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

    /// Extract `#define`(s) by name, including their enclosing `#ifdef`/`#endif`
    /// guards. When a macro has multiple conditional variants, adjacent blocks
    /// are merged into one contiguous region.
    pub fn extract_defines_with_ifdef_context(&self, names: &[&str]) -> Result<CDefines, String> {
        let mut ranges: Vec<(usize, usize)> = Vec::new();

        for name in names {
            let lines = self.find_define_lines(name);
            if lines.is_empty() {
                return Err(format!("Could not find #define for '{}'", name));
            }
            for idx in lines {
                let range = self.find_enclosing_ifdef(idx).unwrap_or((idx, idx));
                ranges.push(range);
            }
        }

        if ranges.is_empty() {
            return Err("Could not find any of the specified defines".to_string());
        }

        let merged = Self::merge_line_ranges(ranges);
        let text = self.extract_line_ranges(&merged);
        Ok(CDefines::new(text))
    }

    // --- Private helpers: function extraction ---

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

    fn should_skip_line_for_function(line: &str) -> bool {
        let trimmed = line.trim_start();
        if trimmed.starts_with("//") || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            return true;
        }
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

    // --- Private helpers: define extraction ---

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
        let name_end = after_define
            .find(|c: char| c.is_whitespace() || c == '(')
            .unwrap_or(after_define.len());

        Some(&after_define[..name_end])
    }

    fn find_define_lines(&self, name: &str) -> Vec<usize> {
        self.lines
            .iter()
            .enumerate()
            .filter_map(|(i, line)| {
                if let Some(n) = Self::parse_define_name(line)
                    && n == name
                {
                    Some(i)
                } else {
                    None
                }
            })
            .collect()
    }

    // --- Private helpers: ifdef context expansion ---

    fn is_if_directive(line: &str) -> bool {
        let t = line.trim();
        t.starts_with("#ifdef") || t.starts_with("#ifndef") || t.starts_with("#if ")
    }

    fn is_endif_directive(line: &str) -> bool {
        line.trim().starts_with("#endif")
    }

    /// Find the immediately enclosing `#ifdef`/`#endif` block for a line.
    fn find_enclosing_ifdef(&self, line_idx: usize) -> Option<(usize, usize)> {
        let mut endif_count: u32 = 0;
        let mut start = None;

        for i in (0..line_idx).rev() {
            let line = &self.lines[i];
            if Self::is_endif_directive(line) {
                endif_count += 1;
            } else if Self::is_if_directive(line) {
                if endif_count > 0 {
                    endif_count -= 1;
                } else {
                    start = Some(i);
                    break;
                }
            }
        }

        let start = start?;

        let mut depth: u32 = 0;
        for i in start..self.lines.len() {
            let line = &self.lines[i];
            if Self::is_if_directive(line) {
                depth += 1;
            } else if Self::is_endif_directive(line) {
                depth -= 1;
                if depth == 0 {
                    return Some((start, i));
                }
            }
        }

        None
    }

    /// Merge overlapping or adjacent line ranges (up to 1 blank line gap).
    fn merge_line_ranges(mut ranges: Vec<(usize, usize)>) -> Vec<(usize, usize)> {
        if ranges.is_empty() {
            return ranges;
        }
        ranges.sort_by_key(|&(start, _)| start);
        let mut merged = vec![ranges[0]];
        for &(start, end) in &ranges[1..] {
            let last = merged.last_mut().unwrap();
            if start <= last.1 + 2 {
                last.1 = last.1.max(end);
            } else {
                merged.push((start, end));
            }
        }
        merged
    }

    fn extract_line_ranges(&self, ranges: &[(usize, usize)]) -> String {
        ranges
            .iter()
            .map(|&(start, end)| self.lines[start..=end].join("\n"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ifdef_context_simple() {
        let src = "\
#ifdef FOO
#define BAR 1
#endif";
        let ext = CExtractor::new(src);
        let result = ext.extract_defines_with_ifdef_context(&["BAR"]).unwrap();
        assert_eq!(result.text, src);
    }

    #[test]
    fn ifdef_context_two_variants_merged() {
        let src = "\
#ifdef ASCII
#define IdChar(C) ascii_version
#endif
#ifdef EBCDIC
int table[] = {0,1,2};
#define IdChar(C) ebcdic_version
#endif";
        let ext = CExtractor::new(src);
        let result = ext.extract_defines_with_ifdef_context(&["IdChar"]).unwrap();
        assert_eq!(result.text, src);
    }

    #[test]
    fn ifdef_context_not_inside_ifdef() {
        let src = "\
#define PLAIN 42
int x = 0;";
        let ext = CExtractor::new(src);
        let result = ext.extract_defines_with_ifdef_context(&["PLAIN"]).unwrap();
        assert_eq!(result.text, "#define PLAIN 42");
    }

    #[test]
    fn ifdef_context_nested() {
        let src = "\
#ifdef OUTER
#ifdef INNER
#define NESTED 1
#endif
#endif";
        let ext = CExtractor::new(src);
        // Should expand to the immediate enclosing ifdef (INNER), not OUTER.
        let range = ext.find_enclosing_ifdef(2).unwrap();
        assert_eq!(range, (1, 3));
    }

    #[test]
    fn ifdef_context_skips_closed_blocks() {
        // The #ifdef A / #endif block is fully closed before our target,
        // so find_enclosing_ifdef should find #ifdef B, not #ifdef A.
        let src = "\
#ifdef A
int a;
#endif
#ifdef B
#define TARGET 1
#endif";
        let ext = CExtractor::new(src);
        let range = ext.find_enclosing_ifdef(4).unwrap();
        assert_eq!(range, (3, 5));
    }

    #[test]
    fn merge_adjacent_ranges() {
        // Adjacent (gap of 1 line) → merged
        let merged = CExtractor::merge_line_ranges(vec![(0, 2), (4, 6)]);
        assert_eq!(merged, vec![(0, 6)]);
    }

    #[test]
    fn merge_disjoint_ranges() {
        // Gap of 2+ lines → not merged
        let merged = CExtractor::merge_line_ranges(vec![(0, 2), (5, 7)]);
        assert_eq!(merged, vec![(0, 2), (5, 7)]);
    }

    #[test]
    fn find_define_lines_multiple() {
        let src = "\
#ifdef A
#define X 1
#endif
#ifdef B
#define X 2
#endif";
        let ext = CExtractor::new(src);
        assert_eq!(ext.find_define_lines("X"), vec![1, 4]);
    }
}
