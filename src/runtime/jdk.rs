use std::fmt;
use super::JdkInfo;
use std::io::Error;
use std::process::{Command, Output};

#[derive(Debug)]
pub enum JdkDetectionError {
    Command(Error),
    CommandFailed { status: i32, stderr: String },
    VersionNotFound,
}

impl fmt::Display for JdkDetectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Command(error) => {
                write!(formatter, "failed to execute java command: {error}")
            }
            Self::CommandFailed { status, stderr } => {
                write!(
                    formatter,
                    "java command failed with exit code {status}: {stderr}"
                )
            }
            Self::VersionNotFound => formatter.write_str("JDK version could not be detected"),
        }
    }
}

impl std::error::Error for JdkDetectionError {}

pub fn detect() -> Result<JdkInfo, JdkDetectionError> {
    let output = Command::new("java")
        .arg("-version")
        .output()
        .map_err(JdkDetectionError::Command)?;

    parse_output(output)
}

fn parse_output(output: Output) -> Result<JdkInfo, JdkDetectionError> {
    if !output.status.success() {
        return Err(JdkDetectionError::CommandFailed {
            status: output.status.code().unwrap_or(-1),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        });
    }

    let stderr = String::from_utf8_lossy(&output.stderr);

    let first_line = stderr
        .lines()
        .next()
        .ok_or(JdkDetectionError::VersionNotFound)?;

    let version = extract_version(first_line).ok_or(JdkDetectionError::VersionNotFound)?;

    Ok(JdkInfo {
        version,
        vendor: None,
        runtime: None,
    })
}

fn extract_version(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')? + start;

    Some(line[start..end].to_string())
}
