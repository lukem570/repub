//! Repub CLI entry point.

use std::process::ExitCode;

use clap::Parser;
use repub_cli::{Cli, run};

fn main() -> ExitCode {
    // Initialize logging
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("warn")
    ).init();

    let cli = Cli::parse();
    
    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("Error: {e}");
            if let Some(source) = e.source() {
                eprintln!("Caused by: {source}");
            }
            ExitCode::FAILURE
        }
    }
}
