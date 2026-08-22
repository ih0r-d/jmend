use super::JdkInfo;
use crate::process::{CommandExecutor, CommandOutput, CommandRequest, ProcessExecutor};
use std::fmt;
use std::io::Error;

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
    detect_with(&ProcessExecutor)
}

pub fn detect_with(executor: &dyn CommandExecutor) -> Result<JdkInfo, JdkDetectionError> {
    let request = CommandRequest::new("java", &["-XshowSettings:properties", "-version"]);
    let output = executor
        .execute(&request)
        .map_err(JdkDetectionError::Command)?;
    parse_output(output)
}

fn parse_output(output: CommandOutput) -> Result<JdkInfo, JdkDetectionError> {
    if !output.success {
        return Err(JdkDetectionError::CommandFailed {
            status: output.status.unwrap_or(-1),
            stderr: output.stderr,
        });
    }

    parse_metadata(&format!("{}\n{}", output.stdout, output.stderr))
}

fn parse_metadata(output: &str) -> Result<JdkInfo, JdkDetectionError> {
    let version =
        property(output, "java.version").or_else(|| output.lines().find_map(extract_version));
    let version = version.ok_or(JdkDetectionError::VersionNotFound)?;

    Ok(JdkInfo {
        version,
        vendor: property(output, "java.vendor"),
        runtime: property(output, "java.runtime.name"),
    })
}

fn property(output: &str, name: &str) -> Option<String> {
    output.lines().find_map(|line| {
        let (key, value) = line.trim().split_once('=')?;
        (key.trim() == name)
            .then(|| value.trim())
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    })
}

fn extract_version(line: &str) -> Option<String> {
    let start = line.find('"')? + 1;
    let end = line[start..].find('"')? + start;

    Some(line[start..end].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_jdk_properties_with_arbitrary_vendor() -> Result<(), JdkDetectionError> {
        let info = parse_metadata(
            "Property settings:\n    java.home = /opt/custom-jdk\n    java.runtime.name = Example Runtime\n    java.vendor = Example Custom JDK Vendor\n    java.version = 21.0.2\n",
        )?;

        assert_eq!(info.version, "21.0.2");
        assert_eq!(info.vendor.as_deref(), Some("Example Custom JDK Vendor"));
        assert_eq!(info.runtime.as_deref(), Some("Example Runtime"));
        Ok(())
    }

    #[test]
    fn missing_vendor_remains_optional_and_is_not_inferred_from_java_home()
    -> Result<(), JdkDetectionError> {
        let info = parse_metadata(
            "java.home = /Library/Java/JavaVirtualMachines/SomeVendorName.jdk\njava.version = 21.0.2\n",
        )?;

        assert_eq!(info.version, "21.0.2");
        assert_eq!(info.vendor, None);
        Ok(())
    }

    #[test]
    fn quoted_version_output_remains_a_fallback() -> Result<(), JdkDetectionError> {
        let info = parse_metadata("openjdk version \"21.0.2\" 2024-01-16")?;

        assert_eq!(info.version, "21.0.2");
        assert_eq!(info.vendor, None);
        Ok(())
    }
}
