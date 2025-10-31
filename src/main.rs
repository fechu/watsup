use chrono::Local;
use clap::{CommandFactory, Parser, Subcommand};
mod frame;
use frame::Frame;
use simple_logger::SimpleLogger;

use crate::frame::Tag;

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
    },
    Stop,
}

fn main() {
    SimpleLogger::new().env().init().unwrap();

    let cli = Cli::parse();

    match &cli.command {
        Some(Commands::Start { project, tags }) => {
            let tags = tags
                .iter()
                .filter_map(|tag| Tag::new(tag.to_string()))
                .collect();
            let mut frame = Frame::new(project, tags);
            let frame = frame.set_end(Local::now() + chrono::Duration::minutes(1));

            log::info!("Frame started: {:?}", frame);
            log::info!("Frame started: {:?}", frame.as_watson_json());

            // Write the frame to file /tmp/frame.json
            let file_path = "/tmp/frame.json";
            let json = frame.as_watson_json();
            std::fs::write(file_path, json).expect("Failed to write frame to file");
        }
        Some(Commands::Stop) => {
            log::info!("Frame stopped");
        }
        None => Cli::command().print_help().unwrap(),
    }
}
