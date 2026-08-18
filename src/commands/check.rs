use crate::runtime::jdk::JdkDetectionError;
use crate::runtime::JdkStatus;
use crate::{
    analysis::finding::Finding,
    error::AppError,
    project::{detector, ProjectContext},
};
use std::path::Path;

#[derive(Debug)]
pub struct CheckResult {
    pub project_context: ProjectContext,
    pub jdk: JdkStatus,
    pub findings: Vec<Finding>,
}

pub fn run(start: &Path) -> Result<CheckResult, AppError> {
    let project = detector::detect(start)?;
    let jdk = match crate::runtime::jdk::detect() {
        Ok(info) => JdkStatus::Detected(info),
        Err(JdkDetectionError::VersionNotFound) => JdkStatus::NotFound,
        Err(error) => JdkStatus::Unavailable(error.to_string()),
    };

    Ok(CheckResult {
        project_context: ProjectContext::new(project),
        jdk,
        findings: Vec::new(),
    })
}
