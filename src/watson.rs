// The compatiblity layer to watson (https://github.com/jazzband/Watson/)
//

use std::{
    fmt::Display,
    fs::File,
    io::{Read, Write},
    path::PathBuf,
};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use serde::{Deserialize, Serialize, ser::SerializeSeq};
use serde_json::json;

use crate::{
    common::NonEmptyString,
    config::Config,
    frame::{self, CompletedFrame, FrameStore},
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

#[derive(Serialize, Deserialize, Debug)]
/// Frame representation used for editing a frame
pub struct FrameEdit {
    project: NonEmptyString,
    start: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    stop: Option<String>,
    tags: Vec<NonEmptyString>,
}

impl FrameEdit {
    pub fn project(&self) -> &NonEmptyString {
        &self.project
    }

    pub fn start(&self) -> DateTime<Local> {
        let naive = NaiveDateTime::parse_from_str(&self.start, EDIT_DATETIME_FORMAT).unwrap();
        naive.and_local_timezone(Local).single().unwrap()
    }

    pub fn stop(&self) -> Option<DateTime<Local>> {
        self.stop
            .clone()
            .map(|s| NaiveDateTime::parse_from_str(&s, EDIT_DATETIME_FORMAT).unwrap())
            .map(|d| d.and_local_timezone(Local).unwrap())
    }

    pub fn tags(&self) -> &[NonEmptyString] {
        &self.tags
    }
}

const EDIT_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

impl From<&frame::Frame> for FrameEdit {
    fn from(frame: &frame::Frame) -> Self {
        FrameEdit {
            project: frame.project().clone(),
            start: frame.start().format(EDIT_DATETIME_FORMAT).to_string(),
            stop: frame
                .end()
                .and_then(|e| Some(e.format(EDIT_DATETIME_FORMAT).to_string())),
            tags: Vec::from(frame.tags()),
        }
    }
}

#[derive(Debug)]
pub enum StoreError {
    OngoingFrameError,
    SerializationError(serde_json::Error),
    IoError(std::io::Error),
}

impl Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::OngoingFrameError => write!(f, "Ongoing frame error"),
            StoreError::SerializationError(e) => write!(f, "Serialization error: {}", e),
            StoreError::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        StoreError::IoError(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        StoreError::SerializationError(error)
    }
}

pub struct Store {
    config: Config,
}

impl Store {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    /// Load the frames from the json file stored in the location from the config.
    fn load(&self) -> Result<Vec<CompletedFrame>, StoreError> {
        let frames_file_path = self.config.get_frames_path();
        if !frames_file_path.exists() {
            return Ok(Vec::new());
        }

        let json = std::fs::read_to_string(frames_file_path)?;
        let frames: Vec<Frame> = serde_json::from_str(&json)?;
        let frames = frames
            .into_iter()
            .map(|frame| CompletedFrame::from(frame))
            .collect();
        Ok(frames)
    }

    fn save(&self, frames: Vec<CompletedFrame>) -> Result<(), StoreError> {
        let json_array = json!(
            frames
                .iter()
                .map(|frame| frame.clone().into())
                .collect::<Vec<Frame>>()
        );
        log::debug!("Writing to frames store. frame_count={}", frames.len());
        let json = serde_json::to_string_pretty(&json_array)?;
        std::fs::write(self.config.get_frames_path(), json)?;
        Ok(())
    }
}

impl FrameStore for Store {
    type FrameStoreError = StoreError;

    fn save_frame(
        &self,
        completed_frame: frame::CompletedFrame,
    ) -> Result<(), Self::FrameStoreError> {
        let mut frames = self.load()?;
        frames.retain(|f| f.frame().id() != completed_frame.frame().id());
        frames.push(completed_frame);
        frames.sort();
        self.save(frames)
    }

    fn get_projects(&self) -> Result<Vec<NonEmptyString>, Self::FrameStoreError> {
        let projects = self
            .load()?
            .iter()
            .map(|f| f.frame().project().clone())
            .collect();
        Ok(projects)
    }

    fn get_last_frame(&self) -> Option<frame::CompletedFrame> {
        match self.load() {
            Ok(frames) => frames.last().and_then(|f| Some(f.clone())),
            Err(_) => None,
        }
    }

    fn save_ongoing_frame(&self, frame: frame::Frame) -> Result<(), Self::FrameStoreError> {
        if self.has_ongoing_frame() {
            return Err(StoreError::OngoingFrameError);
        }

        let state = State::from(frame);
        state
            .save(&self.config.get_state_path())
            .map_err(StoreError::IoError)
    }

    fn clear_ongoing_frame(&self) -> Result<(), Self::FrameStoreError> {
        let mut file = File::create(self.config.get_state_path()).map_err(StoreError::IoError)?;
        file.write_all(b"{}").map_err(StoreError::IoError)
    }

    fn get_ongoing_frame(&self) -> Option<frame::Frame> {
        let state = State::load(&self.config.get_state_path());
        let frame = state.and_then(|s| Some(frame::Frame::from(s)));
        frame
    }
}

#[cfg(test)]
mod store_tests {
    use super::*;
    use frame::Frame;
    use tempfile::TempDir;

    struct TestConfig {
        // Warning ignored as we need to keep ownership of tmp_dir because otherwise the tmp dir is removed again.
        #[allow(dead_code)]
        tmp_dir: TempDir,
        config: Config,
    }

    fn get_test_config() -> TestConfig {
        let tmp_dir = tempfile::TempDir::new().expect("Failed to create tmp dir");

        TestConfig {
            config: Config::new(tmp_dir.path().into()),
            tmp_dir: tmp_dir,
        }
    }

    fn get_test_frame() -> Frame {
        Frame::new(
            NonEmptyString::new("project name").unwrap(),
            None,
            None,
            None,
            vec![],
            None,
        )
    }

    fn get_completed_test_frame() -> CompletedFrame {
        let mut frame = get_test_frame();
        frame.set_end(chrono::Local::now());
        CompletedFrame::from_frame(frame).unwrap()
    }

    #[test]
    fn test_get_last_frame_with_no_frames_returns_none() {
        let test_config = get_test_config();
        let store = Store::new(test_config.config);
        assert!(store.get_last_frame().is_none());
    }

    #[test]
    fn test_get_last_frame() {
        let test_config = get_test_config();
        let store = Store::new(test_config.config);
        let test_frame = get_completed_test_frame();
        assert!(store.get_last_frame().is_none());
        store
            .save_frame(test_frame)
            .expect("Saving test frame failed");

        assert!(store.get_last_frame().is_some());
    }

    #[test]
    fn test_has_no_ongoing_frame_by_default() {
        let test_config = get_test_config();
        let store = Store::new(test_config.config);
        assert!(store.get_ongoing_frame().is_none());
    }

    #[test]
    fn test_has_ongoing_frame_after_storing_one() {
        let test_config = get_test_config();
        let store = Store::new(test_config.config);
        let frame = get_test_frame();
        assert!(store.get_ongoing_frame().is_none());
        store
            .save_ongoing_frame(frame)
            .expect("Failed to save ongoing frame");
        assert!(store.get_ongoing_frame().is_some());
        assert!(store.has_ongoing_frame())
    }

    #[test]
    fn test_clear_ongoing_frame() {
        let test_config = get_test_config();
        let store = Store::new(test_config.config);
        let frame = get_test_frame();
        store
            .save_ongoing_frame(frame)
            .expect("Failed to save ongoing frame");
        assert!(store.get_ongoing_frame().is_some());
        assert!(store.has_ongoing_frame());
        store
            .clear_ongoing_frame()
            .expect("Failed to clear ongoing frame");
        assert!(store.get_ongoing_frame().is_none());
        assert!(!store.has_ongoing_frame());
    }
}
