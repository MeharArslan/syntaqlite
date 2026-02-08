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

    /// Remove `static` keyword from an array declaration
    pub fn remove_array_static(mut self, name: &str) -> Self {
        let extractor = CExtractor::new(&self.content);
        if let Ok(array) = extractor.extract_static_array(name) {
            if array.text.trim_start().starts_with("static ") {
                let transformed = array.text.replacen("static ", "", 1);
                self.content = self.content.replace(&array.text, &transformed);
            }
        }
        self
    }

    /// Add `static` keyword to an array declaration
    pub fn add_array_static(mut self, name: &str) -> Self {
        let extractor = CExtractor::new(&self.content);
        if let Ok(array) = extractor.extract_static_array(name) {
            if !array.text.trim_start().starts_with("static ") {
                // Find the declaration and add static
                let transformed = array.text.replacen(
                    &format!("{}[", name),
                    &format!("static {}[", name),
                    1
                );
                self.content = self.content.replace(&array.text, &transformed);
            }
        }
        self
    }

    /// Rename a function
    pub fn rename_function(mut self, old_name: &str, new_name: &str) -> Self {
        let extractor = CExtractor::new(&self.content);
        if let Ok(function) = extractor.extract_function(old_name) {
            let pattern = format!("{}(", old_name);
            let replacement = format!("{}(", new_name);
            let transformed = function.text.replace(&pattern, &replacement);
            self.content = self.content.replace(&function.text, &transformed);
        }
        self
    }

    /// Add parameters to a function signature (appends after existing params)
    pub fn add_function_parameters(mut self, name: &str, additional_params: &str) -> Self {
        let extractor = CExtractor::new(&self.content);
        if let Ok(function) = extractor.extract_function(name) {
            let signature_pattern = format!("{}(", name);

            if let Some(start_idx) = function.text.find(&signature_pattern) {
                let params_start = start_idx + signature_pattern.len();

                if let Some(brace_idx) = function.text[params_start..].find("){") {
                    let close_paren_idx = params_start + brace_idx;
                    let existing_params = function.text[params_start..close_paren_idx].trim();

                    let insertion = if existing_params.is_empty() {
                        additional_params.to_string()
                    } else {
                        format!(", {}", additional_params)
                    };

                    let mut transformed = function.text.clone();
                    transformed.insert_str(close_paren_idx, &insertion);
                    self.content = self.content.replace(&function.text, &transformed);
                }
            }
        }
        self
    }

    /// Replace text within a specific function's body
    pub fn replace_in_function(mut self, name: &str, from: &str, to: &str) -> Self {
        let extractor = CExtractor::new(&self.content);
        if let Ok(function) = extractor.extract_function(name) {
            let transformed = function.text.replace(from, to);
            self.content = self.content.replace(&function.text, &transformed);
        }
        self
    }

    /// Finish transformation and return the result
    pub fn finish(self) -> String {
        self.content
    }
}
