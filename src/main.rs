use clap::Parser;
use simple_logger::SimpleLogger;

mod cli;
mod common;
mod config;
mod frame;
mod log;
mod watson;

use cli::CommandExecutor;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SimpleLogger::new().env().init().unwrap();

    let cli = cli::Cli::parse();
    let config = config::Config::default();
    let frame_store = watson::Store::new(config);

    let mut command_executor = CommandExecutor::new(frame_store);
    if let Err(error) = command_executor.execute_command(&cli.command) {
        println!("Error: {}", error);
    }

    Ok(())
}
