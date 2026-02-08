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
