use std::env;
use std::fs::File;
use std::process::Command;

use clap::{CommandFactory, Parser, Subcommand};
mod common;
mod config;
mod frame;
mod watson;
use frame::CompletedFrameStore;
use frame::Frame;
use simple_logger::SimpleLogger;
use watson::State;

use crate::common::NonEmptyString;
use crate::config::Config;
use crate::frame::CompletedFrame;
use crate::watson::FrameEdit;
use crate::watson::reset_state;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().env().init().unwrap();

    let cli = Cli::parse();
    let config = config::Config::default();
    let mut frame_store = CompletedFrameStore::load(&config.get_frames_path())?;

    let result = match &cli.command {
        Some(Commands::Start {
            project,
            tags,
            no_gap,
        }) => {
            if State::is_frame_ongoing() {
                Err(String::from("A frame is already ongoing"))
            } else {
                start_project(project, tags, no_gap, &config, &frame_store)
            }
        }
        Some(Commands::Stop) => match State::load(&config.get_state_path()) {
            None => Err(String::from("No project started")),
            Some(state) => {
                let mut frame = Frame::from(state);
                let completed_frame = frame.set_end(chrono::Local::now());
                let frame_project = completed_frame.frame().project().clone();
                let frame_start = completed_frame.frame().start().clone();
                frame_store.add_frame(completed_frame);
                match frame_store.save(&config.get_frames_path()) {
                    Err(e) => Err(e.to_string()),
                    Ok(_) => {
                        reset_state(&config.get_state_path());
                        println!(
                            "Stopping project {}, started {}",
                            frame_project, frame_start
                        );
                        Ok(())
                    }
                }
            }
        },
        Some(Commands::Cancel) => match State::load(&config.get_state_path()) {
            None => Err(String::from("No project started")),
            Some(state) => {
                println!("Canceling the timer for project {}", state.project());
                reset_state(&config.get_state_path());
                Ok(())
            }
        },
        Some(Commands::Projects) => {
            let projects = frame_store.get_projects();
            for project in projects {
                println!("{}", project);
            }
            Ok(())
        }
        Some(Commands::Edit) => {
            // First see if we have a current frame
            let mut frame =
                State::load(&config.get_state_path()).and_then(|s| Some(Frame::from(s)));
            // TODO: Make this nicer. This bool should not be necessary!
            let is_ongoing = frame.is_some();
            if frame.is_none() {
                // If no ongoing frame, take the last frame
                frame = frame_store
                    .get_last_frame()
                    .and_then(|f| Some(f.frame().clone()))
            }

            if let Some(mut f) = frame {
                let editor = env::var_os("EDITOR")
                    .ok_or("Cannot launch editor. $EDITOR environment variable not set")?;

                let tmp_file_path = std::env::temp_dir().join("watsup.tmp");
                let tmp_file_write = File::create(&tmp_file_path)?;
                let frame_edit = watson::FrameEdit::from(&f);
                serde_json::to_writer_pretty(tmp_file_write, &frame_edit)?;
                log::debug!(
                    "Starting editor for editing frame. editor={:?} frame_id={}",
                    editor,
                    f.id()
                );
                let exit_status = Command::new(editor).arg(&tmp_file_path).status()?;
                let tmp_file_read = File::open(&tmp_file_path)?;
                let updated_frame_edit: FrameEdit = serde_json::from_reader(tmp_file_read)?;
                log::debug!("Editor exited. exit_status={:?}", exit_status);
                if exit_status.success() {
                    // TODO: Save the updated frame
                    f.update_from(updated_frame_edit);
                    log::debug!(
                        "Updated frame successfully. Writing updates to disk. frame={:?}",
                        f
                    );

                    if is_ongoing {
                        State::from(f).save(&config.get_state_path())?;
                        log::debug!("Updated ongoing state")
                    } else {
                        let completed_frame = CompletedFrame::from_frame(f).unwrap();
                        frame_store.insert_or_update_frame(completed_frame.clone());
                        frame_store.save(&config.get_frames_path())?;
                        log::debug!("Updated completed frame store. frame={:?}", completed_frame)
                    }

                    Ok(())
                } else {
                    Err(String::from(
                        "There was a problem with the editor. Aborting editing...",
                    ))
                }
            } else {
                Err(String::from("No frame to edit"))
            }
        }
        None => Cli::command().print_help().map_err(|e| e.to_string()),
    };

    match result {
        Err(err) => {
            println!("Error: {}", err);
            std::process::exit(1)
        }
        _ => {}
    };
    Ok(())
}

fn start_project(
    project: &str,
    tags: &[String],
    no_gap: &bool,
    config: &Config,
    frame_store: &CompletedFrameStore,
) -> Result<(), String> {
    let project = NonEmptyString::new(&project.to_string()).ok_or("Invalid project name")?;
    let tags = tags
        .iter()
        .filter_map(|tag| NonEmptyString::new(tag))
        .collect();
    let start = match no_gap {
        true => {
            log::debug!("--no_gap given, finding last end time");
            match frame_store.get_last_frame() {
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
        .save(&config.get_state_path())
        .map_err(|e| e.to_string());
    println!("Project {} started", project);
    result
}
