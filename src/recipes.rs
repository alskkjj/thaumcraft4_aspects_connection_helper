use std::hash::{Hash, };

#[derive(Clone, Eq, PartialEq, PartialOrd, Ord)]
pub struct ElementHandle(String);

impl ElementHandle {
    pub fn get_name(&self) -> String {
        self.0.clone()
    }
}

impl std::fmt::Display for ElementHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Aspect: {}", &self.0)
    }
}
impl std::fmt::Debug for ElementHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "EH({})", &self.0)
    }
}

impl From<String> for ElementHandle {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for ElementHandle {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

impl Hash for ElementHandle {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state)
    }
}

pub struct Element {
    pub(crate) name: String,
    pub(crate) belongs_to_mod: Option<String>,
    pub(crate) base_value: f64,
}

impl Element {
    pub fn pretty_print(&self) -> String {
        format!("{},{},{}", self.name, self.belongs_to_mod.clone().unwrap_or("<>".to_string()), self.base_value)
    }
}

