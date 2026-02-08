use crate::c_extractor::CFunction;

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
