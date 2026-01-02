// The compatiblity layer to watson (https://github.com/jazzband/Watson/)
//

use std::{collections::HashSet, fmt::Display, fs::File, io::Read};

use chrono::{DateTime, Local, TimeZone};
use serde::{Deserialize, Serialize, ser::SerializeSeq};
use serde_json::json;

use crate::{
    common::NonEmptyString,
    config::Config,
    frame::{self, CompletedFrame, FrameStore},
    state::{OngoingFrame as WatsupOngoingFrame, StateStoreBackend},
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

#[derive(Serialize, Deserialize, Debug)]
pub struct OngoingFrame {
    project: NonEmptyString,
    start: i64,
    tags: Vec<NonEmptyString>,
}

impl From<OngoingFrame> for WatsupOngoingFrame {
    fn from(ongoing_frame: OngoingFrame) -> Self {
        let start = chrono::Local
            .timestamp_opt(ongoing_frame.start, 0)
            .single()
            .expect("Invalid timestamp for OngoingFrame::start");
        WatsupOngoingFrame::new(ongoing_frame.project, start, ongoing_frame.tags)
    }
}

impl From<&WatsupOngoingFrame> for OngoingFrame {
    fn from(value: &WatsupOngoingFrame) -> Self {
        OngoingFrame {
            project: value.project().clone(),
            start: value.start().timestamp(),
            tags: value.tags().to_vec(),
        }
    }
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
mod frame_serialization_tests {
    use super::*;
    use serde_json;

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
}

#[cfg(test)]
mod state_serializaton_tests {
    use crate::{common::NonEmptyString, watson::OngoingFrame};
    use chrono::Local;

    #[test]
    fn test_state_load_success() {
        // Create a temporary file with valid state data

        let ongoing_frame = OngoingFrame {
            project: NonEmptyString::new("Project").unwrap(),
            start: Local::now().timestamp(),
            tags: vec![],
        };

        let json = serde_json::to_string(&ongoing_frame).unwrap();

        let roundtrip_ongoing_frame: OngoingFrame = serde_json::from_str(&json).unwrap();

        assert_eq!(ongoing_frame.project, roundtrip_ongoing_frame.project);
        assert_eq!(ongoing_frame.start, roundtrip_ongoing_frame.start);
        assert_eq!(ongoing_frame.tags, roundtrip_ongoing_frame.tags);
    }
}

#[derive(Debug)]
pub enum StoreError {
    Serialization(serde_json::Error),
    IO(std::io::Error),
}

impl Display for StoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoreError::Serialization(e) => write!(f, "Serialization error: {}", e),
            StoreError::IO(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        StoreError::IO(error)
    }
}

impl From<serde_json::Error> for StoreError {
    fn from(error: serde_json::Error) -> Self {
        StoreError::Serialization(error)
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
        let frames = frames.into_iter().map(CompletedFrame::from).collect();
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
        let projects: HashSet<NonEmptyString> = self
            .load()?
            .iter()
            .map(|f| f.frame().project().clone())
            .collect();
        let mut projects: Vec<NonEmptyString> = projects.into_iter().collect();
        projects.sort();
        Ok(projects)
    }

    fn get_last_frame(&self) -> Option<frame::CompletedFrame> {
        match self.load() {
            Ok(frames) => frames.last().cloned(),
            Err(_) => None,
        }
    }

    fn get_frame(&self, frame_id: &str) -> Result<Option<CompletedFrame>, Self::FrameStoreError> {
        let frames = self.load()?;
        Ok(frames
            .iter()
            .find(|frame| frame.frame().id() == frame_id)
            .cloned())
    }

    fn get_frames(
        &self,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Result<Vec<CompletedFrame>, Self::FrameStoreError> {
        let frames = self.load()?;
        Ok(frames
            .into_iter()
            .filter(|f| *f.frame().start() >= start && f.end() <= end)
            .collect())
    }
}

impl StateStoreBackend for Store {
    type StateStoreBackendError = StoreError;

    fn get_state(&self) -> Result<Option<WatsupOngoingFrame>, Self::StateStoreBackendError> {
        let file_path = self.config.get_state_path();
        if !file_path.exists() {
            return Ok(None);
        }

        let mut file = File::open(&self.config.get_state_path())?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let ongoing_frame: OngoingFrame =
            serde_json::from_str(&contents).map_err(StoreError::from)?;
        Ok(Some(ongoing_frame.into()))
    }

    fn store_state(&self, state: &WatsupOngoingFrame) -> Result<(), Self::StateStoreBackendError> {
        let ongoing_frame = OngoingFrame::from(state);
        let mut file = File::create(&self.config.get_state_path())?;
        serde_json::to_writer(&mut file, &ongoing_frame)?;
        Ok(())
    }

    fn clear_state(&self) -> Result<bool, Self::StateStoreBackendError> {
        let state_path = &self.config.get_state_path();
        let exists = state_path.exists();
        std::fs::remove_file(state_path)?;
        Ok(exists)
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

    fn get_test_frame_with_project(project: NonEmptyString) -> Frame {
        Frame::new(project, None, None, None, vec![], None)
    }

    fn get_test_frame() -> Frame {
        get_test_frame_with_project("project name".try_into().unwrap())
    }

    fn get_test_ongoing_frame() -> WatsupOngoingFrame {
        let start = chrono::Local.with_ymd_and_hms(2020, 1, 1, 0, 0, 0).unwrap();
        WatsupOngoingFrame::new("project name".try_into().unwrap(), start, vec![])
    }

    fn get_completed_test_frame_with_project(project: NonEmptyString) -> CompletedFrame {
        get_test_frame_with_project(project).set_end(Local::now())
    }

    fn get_completed_test_frame() -> CompletedFrame {
        get_test_frame().set_end(Local::now())
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
    fn test_has_ongoing_frame_after_storing_one() {
        let test_config = get_test_config();
        let backend: &dyn StateStoreBackend<StateStoreBackendError = StoreError> =
            &Store::new(test_config.config);
        let ongoing_frame = get_test_ongoing_frame();

        backend.store_state(&ongoing_frame).unwrap();

        let fetched_ongoing_frame = backend.get_state().unwrap();
        assert!(fetched_ongoing_frame.is_some());
        let fetched_ongoing_frame = fetched_ongoing_frame.unwrap();

        assert_eq!(fetched_ongoing_frame.project(), ongoing_frame.project());
        assert_eq!(fetched_ongoing_frame.start(), ongoing_frame.start());
        assert_eq!(fetched_ongoing_frame.tags(), ongoing_frame.tags());
    }

    #[test]
    fn get_state_with_no_ongoing_frame() {
        let test_config = get_test_config();
        let backend: &dyn StateStoreBackend<StateStoreBackendError = StoreError> =
            &Store::new(test_config.config);
        assert!(backend.get_state().unwrap().is_none())
    }

    #[test]
    fn test_get_projects_has_none_by_default() {
        let test_config = get_test_config();
        let store = Store::new(test_config.config);

        let projects = store.get_projects().expect("Failed to fetch projects");

        assert_eq!(projects.len(), 0);
    }

    #[test]
    fn test_get_projects() {
        let test_config = get_test_config();
        let store = Store::new(test_config.config);
        let frame = get_completed_test_frame();

        let project = frame.frame().project().clone();
        store.save_frame(frame).expect("Failed to save frame");

        let projects = store.get_projects().expect("Failed to get projects");

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0], project);
    }

    #[test]
    fn test_get_projects_of_multiple_frames_returns_no_duplicates() {
        let test_config = get_test_config();
        let store = Store::new(test_config.config);
        let project = NonEmptyString::new("project").unwrap();
        let frame1 = get_completed_test_frame_with_project(project.clone());
        let frame2 = get_completed_test_frame_with_project(project.clone());

        store.save_frame(frame1).expect("Failed to save frame");
        store.save_frame(frame2).expect("Failed to save frame");

        let projects = store.get_projects().expect("Failed to get projects");

        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0], project);
    }
}
