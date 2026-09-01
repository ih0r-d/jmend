use crate::project::detector::ProjectDetectionError;
use std::{error::Error, fmt, io};

#[derive(Debug)]
pub enum AppError {
    CurrentDirectory(io::Error),
    ProjectDetection(ProjectDetectionError),
    Artifact(crate::artifact::ArtifactError),
    Inspect(crate::commands::inspect::InspectError),
    Output(io::Error),
}

impl fmt::Display for AppError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CurrentDirectory(_) => {
                write!(formatter, "cannot determine the current directory")
            }
            Self::ProjectDetection(error) => error.fmt(formatter),
            Self::Artifact(error) => error.fmt(formatter),
            Self::Inspect(error) => error.fmt(formatter),
            Self::Output(_) => write!(formatter, "cannot write output"),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::CurrentDirectory(error) | Self::Output(error) => Some(error),
            Self::ProjectDetection(error) => Some(error),
            Self::Artifact(error) => Some(error),
            Self::Inspect(error) => Some(error),
        }
    }
}

impl From<ProjectDetectionError> for AppError {
    fn from(error: ProjectDetectionError) -> Self {
        Self::ProjectDetection(error)
    }
}

#[derive(Debug)]
pub struct AnalysisError {
    message: String,
}

impl AnalysisError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AnalysisError {}
