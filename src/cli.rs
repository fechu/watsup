use std::env;
use std::process::Command as ProcessCommand;
use std::{fmt::Display, fs::File};

use chrono::{DateTime, Duration, Local};
use chrono_humanize::HumanTime;
use clap::{Parser, Subcommand};
use log::info;

use crate::{
    common::NonEmptyString,
    frame::{CompletedFrame, Frame, FrameEdit, FrameStore, ProjectName},
    log::FrameLog,
    state::{
        self, Ongoing, StateStore, StateStoreBackend, StateStoreVariant, Stopped, get_state_store,
    },
};

#[derive(Parser)]
#[command(version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
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
    Stop {
        /// The date at which to stop the tracking
        #[arg(long, value_parser = crate::cli_args::parse_datetime_now)]
        at: Option<DateTime<Local>>,
    },
    /// Cancel the current frame
    Cancel,
    /// Edit a frame
    Edit {
        /// The id of the frame to edit.
        /// If none provided, and a frame is ongoing, then frame is the currently ongoing frame.
        /// If none provided, and no frame is ongoing, then frame is the last completed frame.
        #[clap(verbatim_doc_comment)]
        id: Option<String>,
    },
    /// List all projects
    Projects,
    /// Show the status of the currently tracked project
    Status,
    /// Show the log of work between provided start and end date
    Log {
        /// Include the currently ongoing frame (if there is one) in the log
        #[arg(short, long)]
        current: bool,
        /// The date and time from which to show the frames. Defaults to the beginning of the current week.
        #[arg(short, long, value_parser = crate::cli_args::parse_beginning_of_day)]
        from: Option<DateTime<Local>>,
        /// The date and time until which to show the frames. Defaults to now.
        #[arg(short, long, value_parser = crate::cli_args::parse_end_of_day)]
        to: Option<DateTime<Local>>,
    },
}

#[derive(Debug, Clone)]
/// Any kind of error the CLI ever produces
/// Errors from the FrameStore and the StateStore are wrapped in the respective errors.
pub enum CliError<E1, E2> {
    OngoingProject(ProjectName),
    InvalidProjectName,
    FrameStoreError(E1),
    StateStoreError(E2),
    NoOngoingRecording,
    EditorNotSet,
    EditorError(String),
    TempFileError(String),
    SerializationError(String),
    InvalidFrame(Option<String>),
    FutureStopDate,
}

impl<E1: Display, E2: Display> Display for CliError<E1, E2> {
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
            CliError::StateStoreError(details) => {
                write!(f, "State store error. details={}", details)
            }
            CliError::NoOngoingRecording => {
                write!(f, "No project started")
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
            CliError::FutureStopDate => {
                write!(f, "End date cannot be in the future")
            }
        }
    }
}

/// The class responsible for executing commands
pub struct CommandExecutor<T: FrameStore + StateStoreBackend> {
    /// The place where frames are stored
    store: T,
}

impl<T: FrameStore + StateStoreBackend> CommandExecutor<T> {
    pub fn new(store: T) -> Self {
        Self { store }
    }

    pub fn execute_command(
        &mut self,
        command: &Command,
    ) -> Result<(), CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        info!("Executing command: {:?}", command);
        let state_store = get_state_store(&self.store).map_err(CliError::StateStoreError)?;
        match command {
            Command::Start {
                project,
                tags,
                no_gap,
            } => match state_store {
                StateStoreVariant::Ongoing(state_store) => Err(CliError::OngoingProject(
                    state_store
                        .get_ongoing()
                        .map_err(CliError::StateStoreError)?
                        .project()
                        .clone(),
                )),
                StateStoreVariant::Stopped(state_store) => {
                    self.start(state_store, project, tags, no_gap)
                }
            },
            Command::Stop { at } => match state_store {
                StateStoreVariant::Ongoing(state_store) => {
                    let stop_datetime = at.unwrap_or(Local::now());
                    if stop_datetime > Local::now() {
                        return Err(CliError::FutureStopDate);
                    }
                    self.stop(&stop_datetime, state_store)
                }
                StateStoreVariant::Stopped(_) => Err(CliError::NoOngoingRecording),
            },
            Command::Cancel => match state_store {
                StateStoreVariant::Ongoing(state_store) => {
                    let ongoing_frame = state_store
                        .get_ongoing()
                        .map_err(CliError::StateStoreError)?;
                    println!(
                        "Canceling the timer for project {}",
                        ongoing_frame.project()
                    );
                    state_store.cancel().map_err(CliError::StateStoreError)
                }
                StateStoreVariant::Stopped(_) => Err(CliError::NoOngoingRecording),
            },
            Command::Edit { id } => {
                if let Some(id) = id {
                    self.edit(id)
                } else if let StateStoreVariant::Ongoing(state_store) = state_store {
                    self.edit_ongoing(&state_store)
                } else if let Some(f) = self.store.get_last_frame() {
                    self.edit(f.frame().id())
                } else {
                    Err(CliError::InvalidFrame(None))
                }
            }
            Command::Projects => self.list_projects(),
            Command::Status => match state_store {
                StateStoreVariant::Ongoing(state_store) => self.status(state_store),
                StateStoreVariant::Stopped(_) => Err(CliError::NoOngoingRecording),
            },
            Command::Log {
                current: include_current,
                from,
                to,
            } => {
                let from = from.unwrap_or(Local::now() - Duration::days(7));
                let to = to.unwrap_or(Local::now());
                self.show_log(from, to, *include_current, state_store)
            }
        }
    }

    fn start(
        &self,
        state_store: StateStore<T, Stopped>,
        project: &String,
        tags: &[String],
        no_gap: &bool,
    ) -> Result<(), CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        let project = ProjectName::from(
            NonEmptyString::new(&project.to_string()).ok_or(CliError::InvalidProjectName)?,
        );
        let tags = tags
            .iter()
            .filter_map(|tag| NonEmptyString::new(tag))
            .collect();
        let start = match no_gap {
            true => {
                log::debug!("--no_gap given, finding last end time");
                match self.store.get_last_frame() {
                    Some(frame) => frame.end(),
                    None => {
                        log::info!("--no_gap given, but no previous frame. Ignoring --no_gap");
                        chrono::Local::now()
                    }
                }
            }
            false => chrono::Local::now(),
        };

        let ongoing_frame = state_store
            .start(project.clone(), start, tags)
            .map_err(CliError::StateStoreError)?
            .frame;
        log::debug!("Starting frame. frame={:?}", ongoing_frame);
        println!("Project {} started", project);
        Ok(())
    }

    fn stop(
        &self,
        at: &DateTime<Local>,
        state_store: StateStore<T, Ongoing>,
    ) -> Result<(), CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        let completed_frame = state_store
            .stop(at)
            .map_err(CliError::StateStoreError)?
            .frame;
        println!(
            "Stopping project {} at {}, started {}",
            completed_frame.frame().project(),
            completed_frame.end(),
            completed_frame.frame().start()
        );
        self.store
            .save_frame(&completed_frame)
            .map_err(CliError::FrameStoreError)?;
        Ok(())
    }

    fn edit_frame_in_editor(
        frame_edit: &FrameEdit,
    ) -> Result<FrameEdit, CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        let editor = env::var_os("EDITOR").ok_or(CliError::EditorNotSet)?;
        let tmp_file_path = std::env::temp_dir().join("watsup.tmp");
        let tmp_file_write =
            File::create(&tmp_file_path).map_err(|e| CliError::TempFileError(e.to_string()))?;
        serde_json::to_writer_pretty(tmp_file_write, &frame_edit)
            .map_err(|e| CliError::SerializationError(e.to_string()))?;
        log::debug!(
            "Starting editor for editing frame. editor={:?} frame_edit={:?}",
            editor,
            frame_edit
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

        match exit_status.success() {
            true => Ok(updated_frame_edit),
            false => Err(CliError::EditorError(format!(
                "Editor exist status: {}",
                exit_status
            ))),
        }
    }

    fn edit(
        &mut self,
        frame_id: &str,
    ) -> Result<(), CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        let frame = self
            .store
            .get_frame(frame_id)
            .map_err(CliError::FrameStoreError)?
            .ok_or(CliError::InvalidFrame(Some(frame_id.into())))?;

        let updated_frame_edit = Self::edit_frame_in_editor(&FrameEdit::from(frame.frame()))?;

        let mut frame = frame.frame().clone();
        frame.update_from(updated_frame_edit);
        log::debug!(
            "Updated frame successfully. Writing updates to disk. frame={:?}",
            frame
        );
        self.store
            .save_frame(&CompletedFrame::from_frame(frame).unwrap())
            .map_err(CliError::FrameStoreError)
    }

    fn edit_ongoing(
        &self,
        state_store: &StateStore<T, state::Ongoing>,
    ) -> Result<(), CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        let mut ongoing_frame = state_store
            .get_ongoing()
            .map_err(CliError::StateStoreError)?;

        let frame_edit = FrameEdit::from(&ongoing_frame);
        let frame_edit = Self::edit_frame_in_editor(&frame_edit)?;

        ongoing_frame.update_from(frame_edit);
        state_store
            .update_ongoing(ongoing_frame)
            .map_err(CliError::StateStoreError)
    }

    fn list_projects(&self) -> Result<(), CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        let projects = self
            .store
            .get_projects()
            .map_err(CliError::FrameStoreError)?;
        for project in projects {
            println!("{}", project);
        }
        Ok(())
    }

    fn status(
        &self,
        state_store: StateStore<T, Ongoing>,
    ) -> Result<(), CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        let ongoing_frame = state_store
            .get_ongoing()
            .map_err(CliError::StateStoreError)?;
        let frame = Frame::from(ongoing_frame);
        let completed_frame = frame.set_end(Local::now());
        println!(
            "Project {} started {} ({})",
            completed_frame.frame().project(),
            HumanTime::from(*completed_frame.frame().start()),
            completed_frame.frame().start()
        );
        Ok(())
    }

    fn show_log(
        &self,
        from: DateTime<Local>,
        to: DateTime<Local>,
        include_current: bool,
        state_store: StateStoreVariant<T>,
    ) -> Result<(), CliError<T::FrameStoreError, T::StateStoreBackendError>> {
        let mut frames = self
            .store
            .get_frames(from, to)
            .map_err(CliError::FrameStoreError)?;

        if include_current && let StateStoreVariant::Ongoing(state_store) = state_store {
            let ongoing_frame = state_store
                .get_ongoing()
                .map_err(CliError::StateStoreError)?;
            let frame = Frame::from(ongoing_frame).set_end(Local::now());
            frames.push(frame);
        }

        let log = FrameLog::new(&frames);
        print!("{}", log);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stores::in_memory_store::InMemoryStore;

    #[test]
    fn test_start_project() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let command = Command::Start {
            project: "test_project".to_string(),
            tags: vec![],
            no_gap: false,
        };

        let result = executor.execute_command(&command);
        assert!(result.is_ok());
        assert!(executor.store.get().unwrap().is_some());
    }

    #[test]
    fn test_start_project_twice_returns_error() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let command = Command::Start {
            project: "test_project".to_string(),
            tags: vec![],
            no_gap: false,
        };

        executor.execute_command(&command).unwrap();
        let result = executor.execute_command(&command);

        assert!(result.is_err());
        match result {
            Err(CliError::OngoingProject(_)) => {}
            _ => panic!("Expected OngoingProject error"),
        }
    }

    #[test]
    fn test_stop_without_start_returns_error() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let command = Command::Stop { at: None };

        let result = executor.execute_command(&command);
        assert!(result.is_err());
        match result {
            Err(CliError::NoOngoingRecording) => {}
            _ => panic!("Expected NoOngoingRecording error"),
        }
    }

    #[test]
    fn test_start_and_stop_project() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let start_command = Command::Start {
            project: "test project".to_string(),
            tags: vec![],
            no_gap: false,
        };

        executor.execute_command(&start_command).unwrap();

        let stop_command = Command::Stop { at: None };
        let result = executor.execute_command(&stop_command);

        assert!(result.is_ok());
    }

    #[test]
    fn test_cancel_without_start_returns_error() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let command = Command::Cancel;

        let result = executor.execute_command(&command);
        assert!(result.is_err());
        match result {
            Err(CliError::NoOngoingRecording) => {}
            _ => panic!("Expected NoOngoingRecording error"),
        }
    }

    #[test]
    fn test_start_and_cancel_project() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let start_command = Command::Start {
            project: "test project".to_string(),
            tags: vec![],
            no_gap: false,
        };

        executor.execute_command(&start_command).unwrap();

        let cancel_command = Command::Cancel;
        let result = executor.execute_command(&cancel_command);

        assert!(result.is_ok());
        assert!(executor.store.get().unwrap().is_none())
    }

    #[test]
    fn test_list_projects_empty() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let command = Command::Projects;
        let result = executor.execute_command(&command);

        assert!(result.is_ok());
    }

    #[test]
    fn test_stop_with_future_date_returns_error() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let start_command = Command::Start {
            project: "test project".to_string(),
            tags: vec![],
            no_gap: false,
        };

        executor.execute_command(&start_command).unwrap();

        let future_time = Local::now() + Duration::hours(1);
        let stop_command = Command::Stop {
            at: Some(future_time),
        };

        let result = executor.execute_command(&stop_command);
        assert!(result.is_err());
        match result {
            Err(CliError::FutureStopDate) => {}
            _ => panic!("Expected FutureStopDate error"),
        }
    }

    #[test]
    fn test_start_with_tags() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        let command = Command::Start {
            project: "test project".to_string(),
            tags: vec!["tag1".to_string(), "tag2".to_string()],
            no_gap: false,
        };

        let result = executor.execute_command(&command);
        assert!(result.is_ok());
    }

    #[test]
    fn test_start_with_no_gap() {
        let store = InMemoryStore::new();
        let mut executor = CommandExecutor::new(store);

        // First, create and stop a frame
        let start1 = Command::Start {
            project: "project1".to_string(),
            tags: vec![],
            no_gap: false,
        };
        executor.execute_command(&start1).unwrap();

        let stop1 = Command::Stop { at: None };
        executor.execute_command(&stop1).unwrap();

        // Now start a new frame with no_gap
        let start2 = Command::Start {
            project: "project2".to_string(),
            tags: vec![],
            no_gap: true,
        };

        let result = executor.execute_command(&start2);
        assert!(result.is_ok());
    }
}
