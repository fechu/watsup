use std::{env, fs::OpenOptions, io, path::PathBuf};

use ::log::{info, warn};
use clap::Parser;
use simplelog::{Config, WriteLogger};

mod cli;
mod common;
mod config;
mod frame;
mod log;
mod watson;

use cli::CommandExecutor;

fn setup_logging() -> Result<(), io::Error> {
    let home = PathBuf::from(env::var("HOME").unwrap());
    let config_folder = home.join(".config/watsup");
    std::fs::create_dir_all(&config_folder)?;
    let log_file_path = config_folder.join("log.txt");
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(log_file_path)?;
    let _ = WriteLogger::init(::log::LevelFilter::Trace, Config::default(), file);
    info!("Initialized logging");
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    setup_logging()?;

    let cli = cli::Cli::parse();
    let config = config::Config::default();
    let frame_store = watson::Store::new(config);

    let mut command_executor = CommandExecutor::new(frame_store);
    if let Err(error) = command_executor.execute_command(&cli.command) {
        warn!("Command execution error: {:?}", error);
        println!("Error: {}", error);
    }

    Ok(())
}
