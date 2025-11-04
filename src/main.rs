use clap::{CommandFactory, Parser, Subcommand};
mod common;
mod config;
mod frame;
mod watson;
use frame::CompletedFrameStore;
use frame::Frame;
use frame::WatsonState;
use simple_logger::SimpleLogger;

use crate::common::NonEmptyString;
use crate::config::Config;
use crate::frame::reset_state;

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
    /// List all projects
    Projects,
}

fn main() {
    SimpleLogger::new().env().init().unwrap();

    let cli = Cli::parse();
    let config = config::Config::default();

    let result = match &cli.command {
        Some(Commands::Start {
            project,
            tags,
            no_gap,
        }) => {
            if WatsonState::is_frame_ongoing() {
                Err(String::from("A frame is already ongoing"))
            } else {
                start_project(project, tags, no_gap, &config)
            }
        }
        Some(Commands::Stop) => match WatsonState::load(&config.get_state_path()) {
            None => Err(String::from("No project started")),
            Some(state) => {
                let mut frame = Frame::from(state);
                let completed_frame = frame.set_end(chrono::Local::now());
                let frame_project = completed_frame.frame().project().clone();
                let frame_start = completed_frame.frame().start().clone();
                let mut frame_store = CompletedFrameStore::default();
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
        Some(Commands::Projects) => {
            let frame_store = CompletedFrameStore::load(&config.get_frames_path()).unwrap();
            let projects = frame_store.get_projects();
            for project in projects {
                println!("{}", project);
            }
            Ok(())
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
}

fn start_project(
    project: &str,
    tags: &[String],
    no_gap: &bool,
    config: &Config,
) -> Result<(), String> {
    let project = NonEmptyString::new(&project.to_string()).ok_or("Invalid project name")?;
    let tags = tags
        .iter()
        .filter_map(|tag| NonEmptyString::new(tag))
        .collect();
    let start = match no_gap {
        true => {
            log::debug!("--no_gap given, finding last end time");
            let store = CompletedFrameStore::default();
            match store.get_last_frame() {
                Some(frame) => frame.end(),
                None => chrono::Local::now(),
            }
        }
        false => chrono::Local::now(),
    };
    let frame = Frame::new(project.clone(), None, Some(start), None, tags, None);
    log::debug!("Starting frame. frame={:?}", frame);

    // Write the frame to file
    let state = WatsonState::from(frame);
    let result = state
        .save(&config.get_state_path())
        .map_err(|e| e.to_string());
    println!("Project {} started", project);
    result
}
