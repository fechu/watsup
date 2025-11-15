use clap::Parser;
use simple_logger::SimpleLogger;

mod cli;
mod common;
mod config;
mod frame;
mod watson;

use cli::CommandExecutor;
use frame::CompletedFrameStore;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().env().init().unwrap();

    let cli = cli::Cli::parse();
    let config = config::Config::default();
    let frame_store = CompletedFrameStore::load(&config.get_frames_path())?;

    let mut command_executor = CommandExecutor::new(frame_store, config);
    match command_executor.execute_command(&cli.command) {
        Err(error) => println!("Error: {}", error),
        Ok(_) => {}
    }

    Ok(())
}
