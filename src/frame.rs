use std::hash::{DefaultHasher, Hasher};

use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Clone)]
/// Represents a frame associated with a specific project.
///
/// The `Frame` struct is used to encapsulate project-related data
/// and provide methods to interact with it.
pub struct Frame {
    /// The project the frame is associated with.
    project: String,

    /// The frame's unique identifier.
    id: String,

    /// The start time of the frame.
    start: chrono::DateTime<chrono::Local>,

    /// The end time of the frame.
    end: Option<chrono::DateTime<chrono::Local>>,

    /// The tags associated with the frame.
    tags: Vec<Tag>,

    /// The last time the frame was edited.
    last_edit: chrono::DateTime<chrono::Local>,
}

impl Frame {
    pub fn new(name: &str, tags: Vec<Tag>) -> Self {
        // Generate a unique ID for the frame using a hash of the current time.
        let mut hasher = DefaultHasher::new();
        hasher.write(chrono::Local::now().to_string().as_bytes());
        let id = format!("{:x}", hasher.finish());

        Frame {
            project: name.to_string(),
            id: id,
            start: chrono::Local::now(),
            end: None,
            tags: tags,
            last_edit: chrono::Local::now(),
        }
    }

    // pub fn edit_frame(&self) -> FrameEdit {
    //     FrameEdit {
    //         project: self.project.clone(),
    //         start: self.start.timestamp(),
    //         end: match self.end {
    //             Some(end) => Some(end.timestamp()),
    //             None => None,
    //         },
    //         tags: self.tags.clone(),
    //     }
    // }

    pub fn set_end(&mut self, end: chrono::DateTime<chrono::Local>) -> CompletedFrame {
        self.end = Some(end);
        CompletedFrame::from_frame(self.clone()).unwrap()
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Tag(String);

impl Tag {
    pub fn new(t: String) -> Option<Self> {
        if t.is_empty() { None } else { Some(Self(t)) }
    }

    pub fn to_string(&self) -> String {
        self.0.clone()
    }
}

#[derive(Serialize, Deserialize)]
/// Structure that represents a frame during editing.
pub struct FrameEdit {
    /// Name of the project
    project: String,
    /// Start time as unix timestamp
    start: i64,
    /// End time as unix timestamp
    end: Option<i64>,
    /// Tags associated with the frame
    tags: Vec<Tag>,
}

#[derive(Debug)]
pub struct CompletedFrame(Frame);

impl CompletedFrame {
    pub fn from_frame(frame: Frame) -> Option<Self> {
        match frame.end {
            Some(_) => Some(CompletedFrame(frame)),
            None => None,
        }
    }

    pub fn as_watson_json(&self) -> String {
        let json = json!([
            self.0.start.timestamp(),
            self.0
                .end
                .expect("CompletedFrame needs to have an end time")
                .timestamp(),
            self.0.project,
            self.0.id,
            self.0
                .tags
                .iter()
                .map(|tag| tag.to_string())
                .collect::<Vec<_>>(),
            self.0.last_edit.timestamp(),
        ]);
        serde_json::to_string_pretty(&json).unwrap()
    }
}
