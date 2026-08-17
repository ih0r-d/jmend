use crate::{
    error::AppError,
    output::console,
    project::{ProjectContext, detector},
};
use std::{io::Write, path::Path};

pub fn run(root: &Path, output: &mut dyn Write) -> Result<(), AppError> {
    let project = detector::detect(root)?;
    let context = ProjectContext::new(project);
    console::write_project(output, &context).map_err(AppError::Output)
}
