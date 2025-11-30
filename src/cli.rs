use std::env;
use std::fmt::Display;
use std::fs::File;
use std::process::Command as ProcessCommand;

use chrono::DateTime;
use chrono::Duration;
use chrono::Local;
use chrono::TimeZone;
use clap::{Parser, Subcommand};
use log::info;

use crate::common::NonEmptyString;
use crate::frame::CompletedFrame;
use crate::frame::Frame;
use crate::frame::FrameStore;
use crate::log::FrameLog;
use crate::watson;
use crate::watson::FrameEdit;

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
    Stop,
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
        #[arg(short, long, value_parser = parse_from_datetime)]
        from: Option<DateTime<Local>>,
        /// The date and time until which to show the frames. Defaults to now.
        #[arg(short, long, value_parser = parse_to_datetime)]
        to: Option<DateTime<Local>>,
    },
}

/// Variants for parsing a date, time or datetime argument from the command line.
/// See `parse_datetime` for usage
enum DateTimeArgument {
    DateTime(chrono::NaiveDateTime),
    Date(chrono::NaiveDate),
    Time(chrono::NaiveTime),
}

/// Parse a datetime string into a `chrono::DateTime<Local>`
///
/// Accepts formats "YYYY-MM-DD HH:MM" or "HH:MM"
fn parse_datetime(arg: &str) -> Result<DateTimeArgument, String> {
    let arg = arg.trim();
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(arg, "%Y-%m-%d %H:%M") {
        Ok(DateTimeArgument::DateTime(dt))
    } else if let Ok(date) = chrono::NaiveDate::parse_from_str(arg, "%Y-%m-%d") {
        Ok(DateTimeArgument::Date(date))
    } else if let Ok(time) = chrono::NaiveTime::parse_from_str(arg, "%H:%M") {
        Ok(DateTimeArgument::Time(time))
    } else {
        Err("Invalid datetime expected format YYYY-MM-DD HH:MM or HH:MM".to_string())
    }
}

/// Parse a start date
/// By default if the time is not provided, the time will be set to 00:00 to include frames
/// from the very beginning of the day
fn parse_from_datetime(arg: &str) -> Result<chrono::DateTime<Local>, String> {
    match parse_datetime(arg)? {
        DateTimeArgument::DateTime(dt) => Ok(Local.from_local_datetime(&dt).unwrap()),
        DateTimeArgument::Date(date) => {
            let time = chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap();
            Ok(Local.from_local_datetime(&date.and_time(time)).unwrap())
        }
        DateTimeArgument::Time(time) => {
            let date = Local::now();
            Ok(date.with_time(time).unwrap())
        }
    }
}

/// Parse an end date
/// By default if the time is not provided, the time will be set to 23:59 to include frames
/// from the very end of the day
fn parse_to_datetime(arg: &str) -> Result<chrono::DateTime<Local>, String> {
    match parse_datetime(arg)? {
        DateTimeArgument::DateTime(dt) => Ok(Local.from_local_datetime(&dt).unwrap()),
        DateTimeArgument::Date(date) => {
            let time = chrono::NaiveTime::from_hms_opt(23, 59, 59).unwrap();
            Ok(Local.from_local_datetime(&date.and_time(time)).unwrap())
        }
        DateTimeArgument::Time(time) => {
            let date = Local::now();
            Ok(date.with_time(time).unwrap())
        }
    }
}

#[derive(Debug, Clone)]
pub enum CliError<E> {
    OngoingProject(NonEmptyString),
    InvalidProjectName,
    FrameStoreError(E),
    NoOngoingRecording,
    EditorNotSet,
    EditorError(String),
    TempFileError(String),
    SerializationError(String),
    InvalidFrame(Option<String>),
}

impl<E: Display> Display for CliError<E> {
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
        }
    }
}

/// The class responsible for executing commands
pub struct CommandExecutor<T: FrameStore> {
    /// The place where frames are stored
    frame_store: T,
}

impl<T: FrameStore> CommandExecutor<T> {
    pub fn new(frame_store: T) -> Self {
        Self { frame_store }
    }

    pub fn execute_command(
        &mut self,
        command: &Command,
    ) -> Result<(), CliError<T::FrameStoreError>> {
        info!("Executing command: {:?}", command);
        match command {
            Command::Start {
                project,
                tags,
                no_gap,
            } => self.start(project, tags, no_gap),
            Command::Stop => self.stop(),
            Command::Cancel => self.cancel(),
            Command::Edit { id } => {
                if let Some(id) = id {
                    self.edit(id)
                } else if self.frame_store.has_ongoing_frame() {
                    self.edit_ongoing()
                } else if let Some(f) = self.frame_store.get_last_frame() {
                    self.edit(f.frame().id())
                } else {
                    Err(CliError::InvalidFrame(None))
                }
            }
            Command::Projects => self.list_projects(),
            Command::Status => self.status(),
            Command::Log {
                current: include_current,
                from,
                to,
            } => {
                let from = from.unwrap_or(Local::now() - Duration::days(7));
                let to = to.unwrap_or(Local::now());
                self.show_log(from, to, *include_current)
            }
        }
    }

    fn start(
        &self,
        project: &String,
        tags: &[String],
        no_gap: &bool,
    ) -> Result<(), CliError<T::FrameStoreError>> {
        if let Some(ongoing_project_name) = self
            .frame_store
            .get_ongoing_frame()
            .map(|f| f.project().clone())
        {
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
            let result = self
                .frame_store
                .save_ongoing_frame(frame)
                .map_err(CliError::FrameStoreError);
            println!("Project {} started", project);
            result
        }
    }

    fn stop(&mut self) -> Result<(), CliError<T::FrameStoreError>> {
        match &self.frame_store.get_ongoing_frame() {
            None => Err(CliError::NoOngoingRecording),
            Some(frame) => {
                let mut frame = frame.clone();
                let completed_frame = frame.set_end(chrono::Local::now());
                let frame_project = completed_frame.frame().project().clone();
                let frame_start = *completed_frame.frame().start();
                match self.frame_store.save_frame(completed_frame) {
                    Err(e) => Err(CliError::FrameStoreError(e)),
                    Ok(_) => {
                        let result = self
                            .frame_store
                            .clear_ongoing_frame()
                            .map_err(CliError::FrameStoreError);
                        println!(
                            "Stopping project {}, started {}",
                            frame_project, frame_start
                        );
                        result
                    }
                }
            }
        }
    }

    fn cancel(&self) -> Result<(), CliError<T::FrameStoreError>> {
        match &self.frame_store.get_ongoing_frame() {
            None => Err(CliError::NoOngoingRecording),
            Some(state) => {
                println!("Canceling the timer for project {}", state.project());
                self.frame_store
                    .clear_ongoing_frame()
                    .map_err(CliError::FrameStoreError)
            }
        }
    }

    fn edit_frame_in_editor(
        frame_edit: &watson::FrameEdit,
    ) -> Result<FrameEdit, CliError<T::FrameStoreError>> {
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

    fn edit(&mut self, frame_id: &str) -> Result<(), CliError<T::FrameStoreError>> {
        let frame = self
            .frame_store
            .get_frame(frame_id)
            .map_err(CliError::FrameStoreError)?
            .ok_or(CliError::InvalidFrame(Some(frame_id.into())))?;

        let updated_frame_edit =
            Self::edit_frame_in_editor(&watson::FrameEdit::from(frame.frame()))?;

        let mut frame = frame.frame().clone();
        frame.update_from(updated_frame_edit);
        log::debug!(
            "Updated frame successfully. Writing updates to disk. frame={:?}",
            frame
        );
        self.frame_store
            .save_frame(CompletedFrame::from_frame(frame).unwrap())
            .map_err(CliError::FrameStoreError)
    }

    fn edit_ongoing(&mut self) -> Result<(), CliError<T::FrameStoreError>> {
        let mut ongoing_frame = self
            .frame_store
            .get_ongoing_frame()
            .ok_or(CliError::InvalidFrame(None))?;

        let frame_edit = watson::FrameEdit::from(&ongoing_frame);
        let frame_edit = Self::edit_frame_in_editor(&frame_edit)?;

        ongoing_frame.update_from(frame_edit);
        self.frame_store
            .save_ongoing_frame(ongoing_frame)
            .map_err(CliError::FrameStoreError)
    }

    fn list_projects(&self) -> Result<(), CliError<T::FrameStoreError>> {
        let projects = self
            .frame_store
            .get_projects()
            .map_err(CliError::FrameStoreError)?;
        for project in projects {
            println!("{}", project);
        }
        Ok(())
    }

    fn status(&self) -> Result<(), CliError<<T as FrameStore>::FrameStoreError>> {
        match self.frame_store.get_ongoing_frame() {
            None => Err(CliError::NoOngoingRecording),
            Some(frame) => {
                println!("{}", frame);
                Ok(())
            }
        }
    }

    fn show_log(
        &self,
        from: DateTime<Local>,
        to: DateTime<Local>,
        include_current: bool,
    ) -> Result<(), CliError<<T as FrameStore>::FrameStoreError>> {
        let mut frames = self
            .frame_store
            .get_frames(from, to)
            .map_err(CliError::FrameStoreError)?;

        if include_current && let Some(ongoing_frame) = self.frame_store.get_ongoing_frame() {
            let frame = ongoing_frame.clone().set_end(Local::now());
            frames.push(frame);
        }

        let log = FrameLog::new(&frames);
        print!("{}", log);
        Ok(())
    }
}
