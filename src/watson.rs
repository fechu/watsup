// The compatiblity layer to watson (https://github.com/jazzband/Watson/)
//

use std::{fs::File, io::Read, path::PathBuf};

use chrono::TimeZone;
use serde::{Deserialize, Serialize, ser::SerializeSeq};

use crate::{
    common::NonEmptyString,
    config::Config,
    frame::{self},
};

#[derive(Clone)]
pub struct Frame {
    start_timestamp: i64,
    end_timestamp: i64,
    project: NonEmptyString,
    id: String,
    tags: Vec<NonEmptyString>,
    last_edit_timestamp: i64,
}

impl From<frame::CompletedFrame> for Frame {
    fn from(completed_frame: frame::CompletedFrame) -> Self {
        Self {
            start_timestamp: completed_frame.frame().start().timestamp(),
            end_timestamp: completed_frame.end().timestamp(),
            project: completed_frame.frame().project().clone(),
            id: completed_frame.frame().id().into(),
            tags: completed_frame.frame().tags().into(),
            last_edit_timestamp: completed_frame.frame().last_edit().timestamp(),
        }
    }
}

impl From<Frame> for frame::CompletedFrame {
    fn from(value: Frame) -> Self {
        Self::from_frame(frame::Frame::new(
            value.project,
            Some(value.id),
            chrono::Local
                .timestamp_opt(value.start_timestamp, 0)
                .earliest(),
            chrono::Local.timestamp_opt(value.end_timestamp, 0).latest(),
            value.tags,
            chrono::Local
                .timestamp_opt(value.last_edit_timestamp, 0)
                .latest(),
        ))
        .unwrap()
    }
}

impl From<&frame::CompletedFrame> for Frame {
    fn from(completed_frame: &frame::CompletedFrame) -> Self {
        Self::from(completed_frame.clone())
    }
}

// Custom serialization implemention to match the format of watson
impl Serialize for Frame {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut seq = serializer.serialize_seq(Some(6))?;
        seq.serialize_element(&self.start_timestamp)?;
        seq.serialize_element(&self.end_timestamp)?;
        seq.serialize_element(&self.project)?;
        seq.serialize_element(&self.id)?;
        seq.serialize_element(&self.tags)?;
        seq.serialize_element(&self.last_edit_timestamp)?;
        seq.end()
    }
}

// Custom deserialization implemention to match the format of watson
impl<'de> Deserialize<'de> for Frame {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let seq = <[serde_json::Value; 6]>::deserialize(deserializer)?;
        let mut iter = seq.into_iter();

        let start_timestamp = iter
            .next()
            .and_then(|v| v.as_i64())
            .ok_or_else(|| serde::de::Error::custom("Invalid start_timestamp"))?;
        let end_timestamp = iter
            .next()
            .and_then(|v| v.as_i64())
            .ok_or_else(|| serde::de::Error::custom("Invalid end_timestamp"))?;
        let project = iter
            .next()
            .ok_or_else(|| serde::de::Error::custom("Missing project"))?;
        let project: NonEmptyString =
            serde_json::from_value(project).map_err(serde::de::Error::custom)?;
        let id = iter
            .next()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .ok_or_else(|| serde::de::Error::custom("Invalid id"))?;
        let tags = iter
            .next()
            .ok_or_else(|| serde::de::Error::custom("Missing tags"))?;
        let tags: Vec<NonEmptyString> =
            serde_json::from_value(tags).map_err(serde::de::Error::custom)?;
        let last_edit_timestamp = iter
            .next()
            .and_then(|v| v.as_i64())
            .ok_or_else(|| serde::de::Error::custom("Invalid last_edit_timestamp"))?;

        Ok(Frame {
            start_timestamp,
            end_timestamp,
            project,
            id,
            tags,
            last_edit_timestamp,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;
    use std::io::Write;
    use tempfile::NamedTempFile;

    fn make_test_frame() -> Frame {
        Frame {
            start_timestamp: 1620000000,
            end_timestamp: 1620003600,
            project: NonEmptyString::new("test_project").unwrap(),
            id: "abc123".to_string(),
            tags: vec![
                NonEmptyString::new("tag1").unwrap(),
                NonEmptyString::new("tag2").unwrap(),
            ],
            last_edit_timestamp: 1620004000,
        }
    }

    #[test]
    fn test_frame_serialization() {
        let frame = make_test_frame();
        let serialized = serde_json::to_string(&frame).unwrap();
        // Should be a JSON array of 6 elements
        let v: serde_json::Value = serde_json::from_str(&serialized).unwrap();
        assert!(v.is_array());
        let arr = v.as_array().unwrap();
        assert_eq!(arr.len(), 6);
        assert_eq!(arr[0], 1620000000);
        assert_eq!(arr[1], 1620003600);
        assert_eq!(arr[2], "test_project");
        assert_eq!(arr[3], "abc123");
        assert_eq!(arr[4], serde_json::json!(["tag1", "tag2"]));
        assert_eq!(arr[5], 1620004000);
    }

    #[test]
    fn test_frame_deserialization() {
        let json = r#"
            [
                1620000000,
                1620003600,
                "test_project",
                "abc123",
                ["tag1", "tag2"],
                1620004000
            ]
        "#;
        let frame: Frame = serde_json::from_str(json).unwrap();
        assert_eq!(frame.start_timestamp, 1620000000);
        assert_eq!(frame.end_timestamp, 1620003600);
        assert_eq!(frame.project.to_string(), "test_project");
        assert_eq!(frame.id, "abc123");
        assert_eq!(frame.tags.len(), 2);
        assert_eq!(frame.tags[0].to_string(), "tag1");
        assert_eq!(frame.tags[1].to_string(), "tag2");
        assert_eq!(frame.last_edit_timestamp, 1620004000);
    }

    #[test]
    fn test_state_load_success() {
        // Create a temporary file with valid state data
        let mut temp_file = NamedTempFile::new().unwrap();
        let state_data = r#"
        {
            "project": "test_project",
            "start": 1620000000,
            "tags": ["tag1", "tag2"]
        }
        "#;
        temp_file.write_all(state_data.as_bytes()).unwrap();

        // Load the state from the temporary file
        let path = temp_file.path().to_path_buf();
        let loaded_state = State::load(&path);

        // Verify the loaded state
        assert!(loaded_state.is_some());
        let state = loaded_state.unwrap();
        assert_eq!(state.project().to_string(), "test_project");
        assert_eq!(state.start(), 1620000000);
        assert_eq!(state.tags().len(), 2);
        assert_eq!(state.tags()[0].to_string(), "tag1");
        assert_eq!(state.tags()[1].to_string(), "tag2");
    }

    #[test]
    fn test_state_load_file_not_found() {
        // Provide a non-existent file path
        let path = PathBuf::from("non_existent_file.json");
        let loaded_state = State::load(&path);

        // Verify that the state is None
        assert!(loaded_state.is_none());
    }

    #[test]
    fn test_state_load_invalid_data() {
        // Create a temporary file with invalid state data
        let mut temp_file = NamedTempFile::new().unwrap();
        let invalid_data = r#"
        {
            "invalid_key": "invalid_value"
        }
        "#;
        temp_file.write_all(invalid_data.as_bytes()).unwrap();

        // Load the state from the temporary file
        let path = temp_file.path().to_path_buf();
        let loaded_state = State::load(&path);

        // Verify that the state is None
        assert!(loaded_state.is_none());
    }
}

#[derive(Serialize, Deserialize)]
pub struct State {
    project: NonEmptyString,
    start: i64,
    tags: Vec<NonEmptyString>,
}

impl State {
    pub fn from(frame: frame::Frame) -> Self {
        Self {
            project: frame.project().clone(),
            start: frame.start().timestamp(),
            tags: frame.tags().into(),
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let mut file = File::create(path)?;
        serde_json::to_writer(&mut file, self)?;
        Ok(())
    }

    pub fn load(path: &PathBuf) -> Option<State> {
        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return None,
        };
        let mut contents = String::new();
        match file.read_to_string(&mut contents) {
            Ok(_) => (),
            Err(_) => return None,
        };
        match serde_json::from_str(&contents) {
            Ok(state) => Some(state),
            Err(_) => None,
        }
    }

    fn load_default() -> Option<Self> {
        let default_state_file = Config::default().get_state_path();
        Self::load(&default_state_file)
    }

    pub fn is_frame_ongoing() -> bool {
        let state = Self::load_default();
        state.is_some()
    }

    pub fn project(&self) -> &NonEmptyString {
        &self.project
    }

    pub fn start(&self) -> i64 {
        self.start
    }

    pub fn tags(&self) -> &[NonEmptyString] {
        &self.tags
    }
}
