pub mod check;
pub mod inspect;

use crate::{cli::Commands, error::AppError, output::console};
use std::env;

pub fn run(command: Commands) -> Result<(), AppError> {
    match command {
        Commands::Check { project } => {
            let start = match project {
                Some(path) => path,
                None => env::current_dir().map_err(AppError::CurrentDirectory)?,
            };
            let result = check::run(&start)?;
            console::render_check(&result).map_err(AppError::Output)
        }
        Commands::Inspect { path } => {
            let result = inspect::run(&path)?;
            console::render_inspection(&result).map_err(AppError::Output)
        }
    }
}
