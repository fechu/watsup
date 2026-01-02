use std::{
    fmt::Display,
    hash::{DefaultHasher, Hasher},
};

use chrono::{DateTime, Duration, Local, NaiveDateTime};
use serde::{Deserialize, Serialize};

use crate::{common::NonEmptyString, state::OngoingFrame};

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

    pub fn from(state: OngoingFrame) -> Self {
        Frame {
            project: state.project().clone(),
            id: generate_id(),
            start: *state.start(),
            end: None,
            tags: state.tags().into(),
            last_edit: chrono::Local::now(),
        }
    }

    pub fn set_end(mut self, end: chrono::DateTime<chrono::Local>) -> CompletedFrame {
        self.end = Some(end);
        CompletedFrame::from_frame(self).unwrap()
    }

    pub fn update_from(&mut self, edit: FrameEdit) {
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

    pub fn duration(&self) -> Duration {
        self.end() - self.frame().start()
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

impl Display for CompletedFrame {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let time_format = "%H:%M";
        write!(
            f,
            "{:.8}  {} to {}  {:>2}h {:>2}m {:>2}s  {}",
            self.frame().id(),
            self.frame().start().time().format(time_format),
            self.end().time().format(time_format),
            self.duration().num_hours(),
            self.duration().num_minutes() - self.duration().num_hours() * 60,
            self.duration().num_seconds() - self.duration().num_minutes() * 60,
            self.frame().project()
        )
    }
}

const EDIT_DATETIME_FORMAT: &str = "%Y-%m-%d %H:%M:%S";

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

impl From<&Frame> for FrameEdit {
    fn from(frame: &Frame) -> Self {
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

impl From<&OngoingFrame> for FrameEdit {
    fn from(ongoing_frame: &OngoingFrame) -> Self {
        FrameEdit {
            project: ongoing_frame.project().clone(),
            start: ongoing_frame
                .start()
                .format(EDIT_DATETIME_FORMAT)
                .to_string(),
            stop: None,
            tags: Vec::from(ongoing_frame.tags()),
        }
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

    /// Get all frames that fall between start and end time
    fn get_frames(
        &self,
        start: DateTime<Local>,
        end: DateTime<Local>,
    ) -> Result<Vec<CompletedFrame>, Self::FrameStoreError>;
}
