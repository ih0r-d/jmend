use clap::Parser;
use jdoctor_tool::{cli::Cli, commands, error::AppError};
use std::{io, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("jdoctor: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), AppError> {
    let cli = Cli::parse();
    let stdout = io::stdout();
    commands::run(cli.command, &mut stdout.lock())
}
