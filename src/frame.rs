use std::{
    fmt::Display,
    hash::{DefaultHasher, Hasher},
};

use chrono::{DateTime, Local, TimeZone};
use chrono_humanize::HumanTime;

use crate::{
    common::NonEmptyString,
    watson::{self, State},
};

/// Generate a unique ID for the frame using a hash of the current time
fn generate_id() -> String {
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
            end,
            tags,
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

    pub fn update_from(&mut self, edit: watson::FrameEdit) {
        self.project = edit.project().clone();
        self.start = edit.start();
        self.end = edit.stop();
        self.tags = Vec::from(edit.tags());
        self.last_edit = chrono::Local::now();
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

    pub fn end(&self) -> &Option<DateTime<Local>> {
        &self.end
    }
}

impl Display for Frame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Project {} started {} ({})",
            self.project,
            HumanTime::from(self.start),
            self.start
        )
    }
}

/// Represents a completed frame.
/// A completed frame is guaranteed to have an end time.
#[derive(Debug, Clone)]
pub struct CompletedFrame(Frame);

impl CompletedFrame {
    pub fn from_frame(frame: Frame) -> Option<Self> {
        frame.end.map(|_| CompletedFrame(frame))
    }

    pub fn frame(&self) -> &Frame {
        &self.0
    }

    pub fn end(&self) -> DateTime<Local> {
        self.0.end.unwrap()
    }
}

impl Ord for CompletedFrame {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.start().cmp(other.0.start())
    }
}

impl PartialOrd for CompletedFrame {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Eq for CompletedFrame {}

impl PartialEq for CompletedFrame {
    fn eq(&self, other: &Self) -> bool {
        self.0.start() == other.0.start()
    }
}

pub trait FrameStore {
    type FrameStoreError;

    /// Save a frame to the store.
    /// If the frame already exists (identified by "id") it will be updated, otherwise inserted.
    /// Returns an error if the saving failed.
    fn save_frame(&self, frame: CompletedFrame) -> Result<(), Self::FrameStoreError>;

    /// Get all the projects of frames stored in this store.
    fn get_projects(&self) -> Result<Vec<NonEmptyString>, Self::FrameStoreError>;

    /// Get the last frame, ordered by completion datetime.
    fn get_last_frame(&self) -> Option<CompletedFrame>;

    /// Get a frame based on the id.
    /// Returns a CompletedFrame if one matching `frame_id` exists, otherwise None.
    fn get_frame(&self, frame_id: &str) -> Result<Option<CompletedFrame>, Self::FrameStoreError>;

    /// Save a frame that is currently ongoing to the store.
    /// Will fail if there already is an ongoing frame.
    fn save_ongoing_frame(&self, frame: Frame) -> Result<(), Self::FrameStoreError>;

    /// Clear an ongoing frame if there is one.
    /// Will return an error if there is no ongoing frame to clear from the store.
    fn clear_ongoing_frame(&self) -> Result<(), Self::FrameStoreError>;

    /// Get the ongoing frame if there is one.
    fn get_ongoing_frame(&self) -> Option<Frame>;

    fn has_ongoing_frame(&self) -> bool {
        self.get_ongoing_frame().is_some()
    }
}
