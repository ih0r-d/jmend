use crate::{
    analysis::finding::Finding,
    error::AppError,
    project::{ProjectContext, detector},
};
use std::path::Path;

#[derive(Debug)]
pub struct CheckResult {
    pub project_context: ProjectContext,
    pub findings: Vec<Finding>,
}

pub fn run(start: &Path) -> Result<CheckResult, AppError> {
    let project = detector::detect(start)?;

    Ok(CheckResult {
        project_context: ProjectContext::new(project),
        findings: Vec::new(),
    })
}
