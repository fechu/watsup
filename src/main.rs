use clap::{CommandFactory, Parser, Subcommand};
mod config;
mod frame;
use frame::Frame;
use frame::WatsonState;
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
    let config = config::Config::default();

    match &cli.command {
        Some(Commands::Start { project, tags }) => {
            let tags = tags
                .iter()
                .filter_map(|tag| Tag::new(tag.to_string()))
                .collect();
            let frame = Frame::new(project, tags);

            // Write the frame to file
            let state = WatsonState::from(frame);
            state
                .save(&config.get_state_path())
                .expect("Could not write state")
        }
        Some(Commands::Stop) => {
            log::info!("Frame stopped");
        }
        None => Cli::command().print_help().unwrap(),
    }
}
