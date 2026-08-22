use super::build_runtime::{BuildToolCommand, BuildToolRuntime, BuildToolRuntimeError};
use crate::runtime::JdkInfo;

pub fn parse(
    output: &str,
    command: BuildToolCommand,
) -> Result<BuildToolRuntime, BuildToolRuntimeError> {
    let version = output
        .lines()
        .map(str::trim)
        .find_map(|line| {
            line.strip_prefix("Gradle ")
                .and_then(first_value)
                .filter(|value| looks_like_version(value))
        })
        .ok_or(BuildToolRuntimeError::VersionNotFound(command.tool))?;
    let java = keyed_line(output, "JVM:")
        .or_else(|| keyed_line(output, "Launcher JVM:"))
        .and_then(parse_jvm_line);

    Ok(BuildToolRuntime {
        tool: command.tool,
        version: version.to_string(),
        executable: command.executable,
        source: command.source,
        jdk: java,
    })
}

fn parse_jvm_line(value: &str) -> Option<JdkInfo> {
    let version = first_value(value)?;
    if !version.starts_with(|character: char| character.is_ascii_digit()) {
        return None;
    }
    let vendor = value
        .strip_prefix(version)
        .map(str::trim)
        .and_then(|rest| rest.strip_prefix('('))
        .and_then(|rest| rest.strip_suffix(')'))
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    Some(JdkInfo {
        version: version.to_string(),
        vendor,
        runtime: None,
    })
}

fn keyed_line<'a>(output: &'a str, key: &str) -> Option<&'a str> {
    output
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix(key).map(str::trim))
        .filter(|value| !value.is_empty())
}

fn first_value(value: &str) -> Option<&str> {
    value
        .split_whitespace()
        .next()
        .filter(|value| !value.is_empty())
}

fn looks_like_version(value: &str) -> bool {
    value.starts_with(|character: char| character.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{BuildTool, BuildToolSource};
    use std::path::PathBuf;

    fn command() -> BuildToolCommand {
        BuildToolCommand {
            tool: BuildTool::Gradle,
            executable: PathBuf::from("gradlew"),
            source: BuildToolSource::Wrapper,
        }
    }

    #[test]
    fn parses_legacy_gradle_runtime_output() -> Result<(), BuildToolRuntimeError> {
        let runtime = parse(
            "------------------------------------------------------------\nGradle 8.14.3\n------------------------------------------------------------\nJVM: 21.0.7 (Eclipse Adoptium 21.0.7+6-LTS)",
            command(),
        )?;

        assert_eq!(runtime.version, "8.14.3");
        assert_eq!(
            runtime.jdk.as_ref().map(|jdk| jdk.version.as_str()),
            Some("21.0.7")
        );
        assert_eq!(
            runtime.jdk.and_then(|jdk| jdk.vendor).as_deref(),
            Some("Eclipse Adoptium 21.0.7+6-LTS")
        );
        Ok(())
    }

    #[test]
    fn parses_modern_gradle_launcher_jvm() -> Result<(), BuildToolRuntimeError> {
        let runtime = parse(
            "Gradle 9.0.0\nLauncher JVM: 24.0.1 (Homebrew 24.0.1)\nDaemon JVM: /opt/jdk (no JDK specified)",
            command(),
        )?;

        assert_eq!(runtime.version, "9.0.0");
        assert_eq!(
            runtime.jdk.as_ref().map(|jdk| jdk.version.as_str()),
            Some("24.0.1")
        );
        assert_eq!(
            runtime.jdk.and_then(|jdk| jdk.vendor).as_deref(),
            Some("Homebrew 24.0.1")
        );
        Ok(())
    }

    #[test]
    fn missing_jvm_metadata_remains_optional() -> Result<(), BuildToolRuntimeError> {
        let runtime = parse("Gradle 8.10.2", command())?;
        assert_eq!(runtime.jdk, None);
        Ok(())
    }

    #[test]
    fn malformed_output_without_version_is_rejected() {
        assert!(matches!(
            parse("JVM: 21.0.7 (Vendor)", command()),
            Err(BuildToolRuntimeError::VersionNotFound(BuildTool::Gradle))
        ));
        assert!(matches!(
            parse("Gradle unknown", command()),
            Err(BuildToolRuntimeError::VersionNotFound(BuildTool::Gradle))
        ));
    }
}
