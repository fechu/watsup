use serde::{Deserialize, Serialize};
use std::fmt::{self, Display};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NonEmptyString(String);

impl NonEmptyString {
    pub fn new(t: &str) -> Option<Self> {
        if t.is_empty() {
            None
        } else {
            Some(Self(t.to_string()))
        }
    }
}

impl From<NonEmptyString> for String {
    fn from(value: NonEmptyString) -> Self {
        value.0
    }
}

impl Display for NonEmptyString {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
#[cfg(test)]
mod tests {
    use super::NonEmptyString;

    #[test]
    fn test_non_empty_string_new() {
        assert!(NonEmptyString::new("Hello").is_some());
        assert!(NonEmptyString::new("").is_none());
    }

    #[test]
    fn test_non_empty_string_to_string() {
        let non_empty = NonEmptyString::new("Hello").unwrap();
        assert_eq!(non_empty.to_string(), "Hello");
    }
}
