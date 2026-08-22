pub mod check;

use crate::{cli::Commands, error::AppError, output::console};
use std::env;

pub fn run(command: Commands) -> Result<(), AppError> {
    match command {
        Commands::Check => {
            let current_directory = env::current_dir().map_err(AppError::CurrentDirectory)?;
            let result = check::run(&current_directory)?;
            console::render_check(&result).map_err(AppError::Output)
        }
    }
}
