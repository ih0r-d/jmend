use clap::{CommandFactory, Parser};
use jmend::{cli::Cli, commands, error::AppError, output::console};
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("jmend: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), AppError> {
    let cli = Cli::parse();
    if cli.command.is_none() {
        let help = Cli::command().render_help().to_string();
        return console::render_root(&help).map_err(AppError::Output);
    }

    commands::run(cli.command.expect("subcommand checked above"))
}
