pub mod check;

use crate::{cli::Commands, error::AppError};
use std::{env, io::Write};

pub fn run(command: Option<Commands>, output: &mut dyn Write) -> Result<(), AppError> {
    match command {
        Some(Commands::Check) => {
            let current_directory = env::current_dir().map_err(AppError::CurrentDirectory)?;
            check::run(&current_directory, output)
        }
        None => placeholder(output, "interactive mode"),
        Some(Commands::Why { .. }) => placeholder(output, "'why'"),
        Some(Commands::Inspect { .. }) => placeholder(output, "'inspect'"),
        Some(Commands::Explain { .. }) => placeholder(output, "'explain'"),
        Some(Commands::Compare { .. }) => placeholder(output, "'compare'"),
    }
}

fn placeholder(output: &mut dyn Write, command: &str) -> Result<(), AppError> {
    writeln!(output, "jdoctor: {command} is not implemented yet").map_err(AppError::Output)
}
