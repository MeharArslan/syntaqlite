//! High-level C code transformer that applies transformations in-place.
//!
//! Instead of manually extracting, transforming, and reassembling pieces,
//! CTransformer handles the orchestration automatically.

use crate::c_extractor::CExtractor;

pub struct CTransformer {
    content: String,
}

impl CTransformer {
    pub fn new(content: &str) -> Self {
        Self {
            content: content.to_string(),
        }
    }

    /// Remove `static` keyword from a declaration (array, variable, or function)
    ///
    /// This works for array declarations like `static int foo[]`,
    /// variable declarations like `static int bar = 42`,
    /// and function declarations like `static int baz(...)`
    pub fn remove_static(mut self, name: &str) -> Self {
        let lines: Vec<String> = self.content.lines().map(|s| s.to_string()).collect();

        // Try to find the declaration line
        for line in &lines {
            let trimmed = line.trim_start();

            // Check if this line contains the declaration
            if trimmed.starts_with("static ") && line.contains(name) {
                // Additional checks to ensure this is the actual declaration:
                // - Array: contains "name["
                // - Variable: contains "name " or " name "
                // - Function: contains "name("
                let is_array = line.contains(&format!("{}[", name));
                let is_variable = line.contains(&format!("{} ", name))
                    || line.contains(&format!(" {} ", name));
                let is_function = line.contains(&format!("{}(", name));

                if is_array || is_variable || is_function {
                    let transformed = line.replacen("static ", "", 1);
                    self.content = self.content.replace(line, &transformed);
                    break;
                }
            }
        }
        self
    }

    /// Add `static` keyword to an array declaration
    pub fn add_array_static(mut self, name: &str) -> Self {
        let extractor = CExtractor::new(&self.content);
        if let Ok(array) = extractor.extract_static_array(name)
            && !array.text.trim_start().starts_with("static ")
        {
            // Find the declaration and add static
            let transformed =
                array
                    .text
                    .replacen(&format!("{}[", name), &format!("static {}[", name), 1);
            self.content = self.content.replace(&array.text, &transformed);
        }
        self
    }

    /// Add `const` keyword before a type declaration
    /// Example: `add_const("Keyword aKeywordTable")` transforms "Keyword aKeywordTable[]" to "const Keyword aKeywordTable[]"
    pub fn add_const(mut self, type_and_name: &str) -> Self {
        let replacement = format!("const {}", type_and_name);
        self.content = self.content.replace(type_and_name, &replacement);
        self
    }

    /// Add an include directive at the top of the file
    pub fn add_include(mut self, header: &str) -> Self {
        let include = format!("#include \"{}\"\n", header);
        self.content = include + &self.content;
        self
    }

    /// Replace all occurrences of a string throughout the content
    pub fn replace_all(mut self, from: &str, to: &str) -> Self {
        self.content = self.content.replace(from, to);
        self
    }

    /// Insert content after all include directives
    pub fn insert_after_includes(mut self, content: &str) -> Self {
        if let Some(pos) = self.content.rfind("#include") {
            if let Some(newline_pos) = self.content[pos..].find('\n') {
                let insert_pos = pos + newline_pos + 1;
                self.content.insert_str(insert_pos, &format!("\n{}\n", content));
            }
        }
        self
    }

    /// Rename a function
    pub fn rename_function(mut self, old_name: &str, new_name: &str) -> Self {
        let pattern = format!("{}(", old_name);
        let replacement = format!("{}(", new_name);
        self.transform_function(old_name, |text| text.replace(&pattern, &replacement));
        self
    }

    /// Remove a function completely
    pub fn remove_function(mut self, name: &str) -> Self {
        let extractor = CExtractor::new(&self.content);
        if let Ok(function) = extractor.extract_function(name) {
            self.content = self.content.replace(&function.text, "");
        }
        self
    }

    /// Remove lines matching a pattern
    pub fn remove_lines_matching(mut self, pattern: &str) -> Self {
        let lines: Vec<String> = self.content.lines().map(|s| s.to_string()).collect();
        let filtered: Vec<String> = lines
            .into_iter()
            .filter(|line| !line.contains(pattern))
            .collect();
        self.content = filtered.join("\n") + "\n";
        self
    }

    /// Add parameters to a function signature (appends after existing params)
    pub fn add_function_parameters(mut self, name: &str, additional_params: &str) -> Self {
        self.transform_function(name, |text| {
            let signature_pattern = format!("{}(", name);
            let Some(start_idx) = text.find(&signature_pattern) else {
                return text.to_string();
            };
            let params_start = start_idx + signature_pattern.len();
            let Some(brace_idx) = text[params_start..].find("){") else {
                return text.to_string();
            };
            let close_paren_idx = params_start + brace_idx;
            let existing_params = text[params_start..close_paren_idx].trim();

            let insertion = if existing_params.is_empty() {
                additional_params.to_string()
            } else {
                format!(", {}", additional_params)
            };

            let mut transformed = text.to_string();
            transformed.insert_str(close_paren_idx, &insertion);
            transformed
        });
        self
    }

    /// Replace text within a specific function's body
    pub fn replace_in_function(mut self, name: &str, from: &str, to: &str) -> Self {
        self.transform_function(name, |text| text.replace(from, to));
        self
    }

    /// Extract a function by name, apply `f` to its text, and replace it in content.
    fn transform_function(&mut self, name: &str, f: impl FnOnce(&str) -> String) {
        let extractor = CExtractor::new(&self.content);
        if let Ok(function) = extractor.extract_function(name) {
            let transformed = f(&function.text);
            self.content = self.content.replace(&function.text, &transformed);
        }
    }

    /// Finish transformation and return the result
    pub fn finish(self) -> String {
        self.content
    }
}
