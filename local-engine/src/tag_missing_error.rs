use std::{error::Error, fmt::Display};

#[derive(Debug, Default)]
pub struct TagMissingError {
    description: String,
}

impl TagMissingError {
    pub fn new(description: String) -> Self {
        Self { description }
    }

    pub fn description(&self) -> &str {
        self.description.as_ref()
    }
}

impl Display for TagMissingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.description())
    }
}

impl Error for TagMissingError {}
