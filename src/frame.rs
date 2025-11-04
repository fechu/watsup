use std::{
    fs::File,
    hash::{DefaultHasher, Hasher},
    io::{Read, Write},
    path::PathBuf,
};

use chrono::{DateTime, Local, TimeZone};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::{common::NonEmptyString, config::Config};

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

fn generate_id() -> String {
    // Generate a unique ID for the frame using a hash of the current time.
    let mut hasher = DefaultHasher::new();
    hasher.write(chrono::Local::now().to_string().as_bytes());
    format!("{:x}", hasher.finish())
}

impl Frame {
    pub fn new(
        name: NonEmptyString,
        tags: Vec<NonEmptyString>,
        start: Option<chrono::DateTime<Local>>,
    ) -> Self {
        Frame {
            project: name,
            id: generate_id(),
            start: start.unwrap_or(chrono::Local::now()),
            end: None,
            tags: tags,
            last_edit: chrono::Local::now(),
        }
    }

    pub fn from(state: WatsonState) -> Self {
        Frame {
            project: state.project,
            id: generate_id(),
            start: chrono::Local.timestamp_opt(state.start, 0).unwrap(),
            end: None,
            tags: state.tags,
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

    pub fn project(&self) -> &NonEmptyString {
        &self.project
    }

    pub(crate) fn start(&self) -> &chrono::DateTime<chrono::Local> {
        &self.start
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
    tags: Vec<NonEmptyString>,
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

    pub fn frame(&self) -> &Frame {
        &self.0
    }

    pub fn end(&self) -> DateTime<Local> {
        self.0.end.unwrap()
    }

    fn as_watson_json(&self) -> Value {
        json!([
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
        ])
    }

    fn from_watson_json(json: &Value) -> Result<CompletedFrame, String> {
        let array = json
            .as_array()
            .ok_or("Encountered unexpected watson frame format")?;

        let start = array[0].as_i64().ok_or("Invalid start time")?;
        let end = array[1].as_i64().ok_or("Invalid end time")?;
        let project = NonEmptyString::new(array[2].as_str().ok_or("Invalid project name")?)
            .expect("Invalid project name");
        let id = array[3].as_str().ok_or("Invalid frame ID")?;
        let tags = array[4]
            .as_array()
            .ok_or("Invalid tags")?
            .iter()
            .filter_map(|s| NonEmptyString::new(&s.to_string()))
            .collect::<Vec<_>>();
        let last_edit = array[5].as_i64().ok_or("Invalid last edit time")?;

        let start_time = Local
            .timestamp_opt(start, 0)
            .earliest()
            .ok_or("Failed to parse start time")?;

        let end_time = Local
            .timestamp_opt(end, 0)
            .latest()
            .ok_or("Failed to parse end time")?;

        let last_edit = Local
            .timestamp_opt(last_edit, 0)
            .earliest()
            .ok_or("Failed to parse last edit")?;

        Ok(CompletedFrame {
            0: Frame {
                start: start_time,
                end: Some(end_time),
                project: project,
                id: id.to_string(),
                tags,
                last_edit: last_edit,
            },
        })
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
        let json = std::fs::read_to_string(path).unwrap();

        let frames: Value = serde_json::from_str(&json).unwrap();
        match frames {
            Value::Array(frames) => {
                let f = frames
                    .iter()
                    .filter_map(|frame| CompletedFrame::from_watson_json(frame).ok())
                    .collect();
                Ok(Self { frames: f })
            }
            _ => Err(String::from("Expected an array of frames in file")),
        }
    }

    pub fn add_frame(&mut self, frame: CompletedFrame) {
        self.frames.push(frame);
    }

    pub fn save(&self, store_path: &PathBuf) -> Result<(), std::io::Error> {
        let json_array = json!(
            self.frames
                .iter()
                .map(|frame| frame.as_watson_json())
                .collect::<Vec<_>>()
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

pub fn reset_state(path: &PathBuf) {
    let mut file = File::create(path).expect("Cannot write state file");
    file.write(b"{}").expect("Cannot write state file");
}

#[derive(Serialize, Deserialize)]
pub struct WatsonState {
    project: NonEmptyString,
    start: i64,
    tags: Vec<NonEmptyString>,
}

impl WatsonState {
    pub fn from(frame: Frame) -> Self {
        Self {
            project: frame.project,
            start: frame.start.timestamp(),
            tags: frame.tags,
        }
    }

    pub fn save(&self, path: &PathBuf) -> Result<(), std::io::Error> {
        let mut file = File::create(path)?;
        serde_json::to_writer(&mut file, self)?;
        Ok(())
    }

    pub fn load(path: &PathBuf) -> Option<WatsonState> {
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

    pub fn is_frame_ongoing() -> bool {
        let state = Self::load_default();
        state.is_some()
    }

    fn load_default() -> Option<Self> {
        let default_state_file = Config::default().get_state_path();
        Self::load(&default_state_file)
    }
}
