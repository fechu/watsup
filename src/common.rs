use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    fmt::{self, Display},
    hash::Hash,
};

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

impl PartialEq for NonEmptyString {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Eq for NonEmptyString {}

impl From<NonEmptyString> for String {
    fn from(value: NonEmptyString) -> Self {
        value.0
    }
}

impl TryFrom<&str> for NonEmptyString {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        NonEmptyString::new(value).ok_or("String is empty")
    }
}

impl Hash for NonEmptyString {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.hash(state);
    }
}

impl PartialOrd for NonEmptyString {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NonEmptyString {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.cmp(&other.0)
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
