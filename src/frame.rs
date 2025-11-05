use std::{
    hash::{DefaultHasher, Hasher},
    path::PathBuf,
};

use chrono::{DateTime, Local, TimeZone};
use serde_json::json;

use crate::{
    common::NonEmptyString,
    config::Config,
    watson::{self, State},
};

fn generate_id() -> String {
    // Generate a unique ID for the frame using a hash of the current time.
    let mut hasher = DefaultHasher::new();
    hasher.write(chrono::Local::now().to_string().as_bytes());
    format!("{:x}", hasher.finish())
}

#[derive(Debug, Clone)]
/// Represents a frame associated with a specific project.
///
/// The `Frame` struct is used to encapsulate project-related data
/// and provide methods to interact with it.
pub struct Frame {
    /// The project the frame is associated with.
    project: NonEmptyString,

    /// The frame's unique identifier.
    id: String,

    /// The start time of the frame.
    start: chrono::DateTime<chrono::Local>,

    /// The end time of the frame.
    end: Option<chrono::DateTime<chrono::Local>>,

    /// The tags associated with the frame.
    tags: Vec<NonEmptyString>,

    /// The last time the frame was edited.
    last_edit: chrono::DateTime<chrono::Local>,
}

impl Frame {
    pub fn new(
        project: NonEmptyString,
        id: Option<String>,
        start: Option<chrono::DateTime<Local>>,
        end: Option<chrono::DateTime<Local>>,
        tags: Vec<NonEmptyString>,
        last_edit: Option<chrono::DateTime<Local>>,
    ) -> Self {
        Frame {
            project,
            id: id.unwrap_or(generate_id()),
            start: start.unwrap_or(chrono::Local::now()),
            end: end,
            tags: tags,
            last_edit: last_edit.unwrap_or(chrono::Local::now()),
        }
    }

    pub fn from(state: State) -> Self {
        Frame {
            project: state.project().clone(),
            id: generate_id(),
            start: chrono::Local.timestamp_opt(state.start(), 0).unwrap(),
            end: None,
            tags: state.tags().into(),
            last_edit: chrono::Local::now(),
        }
    }

    pub fn set_end(&mut self, end: chrono::DateTime<chrono::Local>) -> CompletedFrame {
        self.end = Some(end);
        CompletedFrame::from_frame(self.clone()).unwrap()
    }

    pub fn project(&self) -> &NonEmptyString {
        &self.project
    }

    pub(crate) fn start(&self) -> &chrono::DateTime<chrono::Local> {
        &self.start
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn tags(&self) -> &[NonEmptyString] {
        &self.tags
    }

    pub fn last_edit(&self) -> DateTime<Local> {
        self.last_edit
    }
}

/// Represents a completed frame.
/// A completed frame is guaranteed to have an end time.
#[derive(Debug, Clone)]
pub struct CompletedFrame(Frame);

impl CompletedFrame {
    pub fn from_frame(frame: Frame) -> Option<Self> {
        match frame.end {
            Some(_) => Some(CompletedFrame(frame)),
            None => None,
        }
    }

    pub fn frame(&self) -> &Frame {
        &self.0
    }

    pub fn end(&self) -> DateTime<Local> {
        self.0.end.unwrap()
    }
}

pub struct CompletedFrameStore {
    frames: Vec<CompletedFrame>,
}

impl CompletedFrameStore {
    /**
     * Load a CompletedFrameStore from a file
     */
    pub fn load(path: &PathBuf) -> Result<Self, String> {
        if !path.exists() {
            return Ok(CompletedFrameStore { frames: Vec::new() });
        }

        let json = std::fs::read_to_string(path).unwrap();
        let frames: Vec<watson::Frame> = serde_json::from_str(&json).unwrap();
        let frames = frames
            .into_iter()
            .map(|frame| CompletedFrame::from(frame))
            .collect();
        Ok(CompletedFrameStore { frames })
    }

    pub fn add_frame(&mut self, frame: CompletedFrame) {
        self.frames.push(frame);
    }

    pub fn save(&self, store_path: &PathBuf) -> Result<(), std::io::Error> {
        let json_array = json!(
            self.frames
                .iter()
                .map(|frame| frame.clone().into())
                .collect::<Vec<watson::Frame>>()
        );
        let json = serde_json::to_string_pretty(&json_array).unwrap();
        std::fs::write(store_path, json)?;
        Ok(())
    }

    pub fn get_projects(&self) -> Vec<NonEmptyString> {
        self.frames.iter().map(|f| f.0.project.clone()).collect()
    }

    pub fn get_last_frame(&self) -> Option<&CompletedFrame> {
        self.frames.last()
    }
}

impl Default for CompletedFrameStore {
    /**
     * Creates a new CompletedFrameStore instance with default configuration.
     */
    fn default() -> Self {
        let config = Config::default();
        CompletedFrameStore::load(&config.get_frames_path()).expect("Failed to read frames store")
    }
}
