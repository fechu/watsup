use std::env;
use std::fmt::Display;
use std::fs::File;
use std::process::Command as ProcessCommand;

use clap::{Parser, Subcommand};

use crate::common::NonEmptyString;
use crate::config::Config;
use crate::frame::CompletedFrame;
use crate::frame::CompletedFrameStore;
use crate::frame::Frame;
use crate::watson;
use crate::watson::FrameEdit;
use crate::watson::State;
use crate::watson::reset_state;

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Start a new frame to record time for a project
    Start {
        /// The name of the project to track the time for
        project: String,
        /// Tags to associate with the frame
        tags: Vec<String>,
        /// Set the start time of the frame to the end time of the previous frame
        #[arg(short, long)]
        no_gap: bool,
    },
    /// Stop the current frame
    Stop,
    /// Cancel the current frame
    Cancel,
    /// Edit a frame
    Edit,
    /// List all projects
    Projects,
}

#[derive(Debug, Clone)]
pub enum CliError {
    OngoingProject(NonEmptyString),
    InvalidProjectName,
    FrameStoreError(String),
    NoOngoingRecording,
    EditorNotSet,
    EditorError(String),
    TempFileError(String),
    SerializationError(String),
    InvalidFrame(Option<String>),
}

impl Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::OngoingProject(project) => {
                write!(f, "Project {} already started", project)
            }
            CliError::InvalidProjectName => {
                write!(f, "Invalid project name")
            }
            CliError::FrameStoreError(details) => {
                write!(f, "Failed to store frame. details={}", details)
            }
            CliError::NoOngoingRecording => {
                write!(f, "No ongoing recording")
            }
            CliError::EditorNotSet => {
                write!(f, "Editor not set via EDITOR env variable")
            }
            CliError::EditorError(details) => {
                write!(f, "Editor error: {}", details)
            }
            CliError::TempFileError(details) => {
                write!(f, "Temp file error: {}", details)
            }
            CliError::SerializationError(details) => {
                write!(f, "Serialization error: {}", details)
            }
            CliError::InvalidFrame(details) => {
                write!(
                    f,
                    "Invalid frame: {}",
                    details.clone().unwrap_or(String::from("No Details"))
                )
            }
        }
    }
}

/// The class responsible for executing commands
pub struct CommandExecutor {
    /// The place where frames are stored
    frame_store: CompletedFrameStore,

    /// TODO: Get rid of the config in this class, shouldn't be needed if we have a good FrameStore trait
    config: Config,
}

impl CommandExecutor {
    pub fn new(frame_store: CompletedFrameStore, config: Config) -> Self {
        Self {
            frame_store: frame_store,
            config: config,
        }
    }

    pub fn execute_command(&mut self, command: &Command) -> Result<(), CliError> {
        match command {
            Command::Start {
                project,
                tags,
                no_gap,
            } => self.start(project, tags, no_gap),
            Command::Stop => self.stop(),
            Command::Cancel => self.cancel(),
            Command::Edit => self.edit(),
            Command::Projects => self.list_projects(),
        }
    }

    fn start(&self, project: &String, tags: &Vec<String>, no_gap: &bool) -> Result<(), CliError> {
        if let Some(ongoing_project_name) = State::ongoing_project_name() {
            Err(CliError::OngoingProject(ongoing_project_name))
        } else {
            let project =
                NonEmptyString::new(&project.to_string()).ok_or(CliError::InvalidProjectName)?;
            let tags = tags
                .iter()
                .filter_map(|tag| NonEmptyString::new(tag))
                .collect();
            let start = match no_gap {
                true => {
                    log::debug!("--no_gap given, finding last end time");
                    match self.frame_store.get_last_frame() {
                        Some(frame) => frame.end(),
                        None => chrono::Local::now(),
                    }
                }
                false => chrono::Local::now(),
            };
            let frame = Frame::new(project.clone(), None, Some(start), None, tags, None);
            log::debug!("Starting frame. frame={:?}", frame);

            // Write the frame to file
            let state = State::from(frame);
            let result = state
                .save(&self.config.get_state_path())
                .map_err(|e| CliError::FrameStoreError(e.to_string()));
            println!("Project {} started", project);
            result
        }
    }

    fn stop(&mut self) -> Result<(), CliError> {
        match State::load(&self.config.get_state_path()) {
            None => Err(CliError::NoOngoingRecording),
            Some(state) => {
                let mut frame = Frame::from(state);
                let completed_frame = frame.set_end(chrono::Local::now());
                let frame_project = completed_frame.frame().project().clone();
                let frame_start = completed_frame.frame().start().clone();
                self.frame_store.add_frame(completed_frame);
                match self.frame_store.save(&self.config.get_frames_path()) {
                    Err(e) => Err(CliError::FrameStoreError(e.to_string())),
                    Ok(_) => {
                        reset_state(&self.config.get_state_path());
                        println!(
                            "Stopping project {}, started {}",
                            frame_project, frame_start
                        );
                        Ok(())
                    }
                }
            }
        }
    }

    fn cancel(&self) -> Result<(), CliError> {
        match State::load(&self.config.get_state_path()) {
            None => Err(CliError::NoOngoingRecording),
            Some(state) => {
                println!("Canceling the timer for project {}", state.project());
                reset_state(&self.config.get_state_path());
                Ok(())
            }
        }
    }

    fn edit(&mut self) -> Result<(), CliError> {
        // First see if we have a current frame
        let mut frame =
            State::load(&self.config.get_state_path()).and_then(|s| Some(Frame::from(s)));
        // TODO: Make this nicer. This bool should not be necessary!
        let is_ongoing = frame.is_some();
        if frame.is_none() {
            // If no ongoing frame, take the last frame
            frame = self
                .frame_store
                .get_last_frame()
                .and_then(|f| Some(f.frame().clone()))
        }

        if let Some(mut f) = frame {
            let editor = env::var_os("EDITOR").ok_or(CliError::EditorNotSet)?;

            let tmp_file_path = std::env::temp_dir().join("watsup.tmp");
            let tmp_file_write =
                File::create(&tmp_file_path).map_err(|e| CliError::TempFileError(e.to_string()))?;
            let frame_edit = watson::FrameEdit::from(&f);
            serde_json::to_writer_pretty(tmp_file_write, &frame_edit)
                .map_err(|e| CliError::SerializationError(e.to_string()))?;
            log::debug!(
                "Starting editor for editing frame. editor={:?} frame_id={}",
                editor,
                f.id()
            );
            let exit_status = ProcessCommand::new(editor)
                .arg(&tmp_file_path)
                .status()
                .map_err(|e| CliError::EditorError(e.to_string()))?;
            let tmp_file_read =
                File::open(&tmp_file_path).map_err(|e| CliError::TempFileError(e.to_string()))?;
            let updated_frame_edit: FrameEdit = serde_json::from_reader(tmp_file_read)
                .map_err(|e| CliError::SerializationError(e.to_string()))?;
            log::debug!("Editor exited. exit_status={:?}", exit_status);
            if exit_status.success() {
                // TODO: Save the updated frame
                f.update_from(updated_frame_edit);
                log::debug!(
                    "Updated frame successfully. Writing updates to disk. frame={:?}",
                    f
                );

                if is_ongoing {
                    State::from(f)
                        .save(&self.config.get_state_path())
                        .map_err(|e| CliError::FrameStoreError(e.to_string()))?;
                    log::debug!("Updated ongoing state")
                } else {
                    let completed_frame = CompletedFrame::from_frame(f).unwrap();
                    self.frame_store
                        .insert_or_update_frame(completed_frame.clone());
                    self.frame_store
                        .save(&self.config.get_frames_path())
                        .map_err(|e| CliError::FrameStoreError(e.to_string()))?;
                    log::debug!("Updated completed frame store. frame={:?}", completed_frame)
                }

                Ok(())
            } else {
                Err(CliError::EditorError(format!(
                    "Editor exist status: {}",
                    exit_status.to_string()
                )))
            }
        } else {
            Err(CliError::InvalidFrame(None))
        }
    }

    fn list_projects(&self) -> Result<(), CliError> {
        let projects = self.frame_store.get_projects();
        for project in projects {
            println!("{}", project);
        }
        Ok(())
    }
}
