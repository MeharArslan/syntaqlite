use crate::c_extractor::{CFunction, CStaticArray};

pub trait ChangeNameTransform {
    fn change_name(self, new_name: &str) -> Self;
}

impl ChangeNameTransform for CFunction {
    fn change_name(mut self, new_name: &str) -> Self {
        let pattern = format!("{}(", self.name);
        let replacement = format!("{}(", new_name);
        self.text = self.text.replace(&pattern, &replacement);
        self.name = new_name.to_string();
        self
    }
}

pub trait AddStaticTransform {
    fn add_static(self) -> Self;
}

impl AddStaticTransform for CStaticArray {
    fn add_static(mut self) -> Self {
        if !self.text.starts_with("static ") {
            self.text = format!("static {}", self.text);
        }
        self
    }
}

pub trait AddParametersTransform {
    fn add_parameters(self, additional_params: &str) -> Self;
}

impl AddParametersTransform for CFunction {
    fn add_parameters(mut self, additional_params: &str) -> Self {
        // Find the closing parenthesis of the parameter list
        // We look for the pattern: name(...){
        let signature_pattern = format!("{}(", self.name);

        if let Some(start_idx) = self.text.find(&signature_pattern) {
            let params_start = start_idx + signature_pattern.len();

            // Find the matching closing paren before the opening brace
            if let Some(brace_idx) = self.text[params_start..].find("){") {
                let close_paren_idx = params_start + brace_idx;

                // Insert additional parameters before the closing paren
                // Check if there are existing parameters (non-empty between parens)
                let existing_params = self.text[params_start..close_paren_idx].trim();

                let insertion = if existing_params.is_empty() {
                    // No existing parameters
                    additional_params.to_string()
                } else {
                    // Has existing parameters, add comma
                    format!(", {}", additional_params)
                };

                self.text.insert_str(close_paren_idx, &insertion);
            }
        }

        self
    }
}
