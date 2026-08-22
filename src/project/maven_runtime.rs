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
            line.strip_prefix("Apache Maven ")
                .or_else(|| line.strip_prefix("Maven "))
                .and_then(first_value)
                .filter(|value| looks_like_version(value))
        })
        .ok_or(BuildToolRuntimeError::VersionNotFound(command.tool))?;
    let java = keyed_line(output, "Java version:").and_then(parse_java_line);

    Ok(BuildToolRuntime {
        tool: command.tool,
        version: version.to_string(),
        executable: command.executable,
        source: command.source,
        jdk: java,
    })
}

fn parse_java_line(value: &str) -> Option<JdkInfo> {
    let mut parts = value.split(',').map(str::trim);
    let version = parts.next().filter(|value| looks_like_version(value))?;
    let vendor = parts.find_map(|part| labeled_value(part, "vendor:"));
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

fn labeled_value(value: &str, label: &str) -> Option<String> {
    value
        .strip_prefix(label)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
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
            tool: BuildTool::Maven,
            executable: PathBuf::from("mvnw"),
            source: BuildToolSource::Wrapper,
        }
    }

    #[test]
    fn parses_realistic_maven_runtime_output() -> Result<(), BuildToolRuntimeError> {
        let runtime = parse(
            "Apache Maven 3.9.11 (abcdef)\nMaven home: /opt/maven\nJava version: 21.0.8, vendor: Oracle Corporation, runtime: /opt/jdk\nDefault locale: en_US",
            command(),
        )?;

        assert_eq!(runtime.version, "3.9.11");
        assert_eq!(
            runtime.jdk.as_ref().map(|jdk| jdk.version.as_str()),
            Some("21.0.8")
        );
        assert_eq!(
            runtime.jdk.and_then(|jdk| jdk.vendor).as_deref(),
            Some("Oracle Corporation")
        );
        Ok(())
    }

    #[test]
    fn parses_maven_output_with_spacing_and_missing_jdk() -> Result<(), BuildToolRuntimeError> {
        let runtime = parse(
            "  Maven 4.0.0-rc-4  \nMaven home: C:\\tools\\maven",
            command(),
        )?;

        assert_eq!(runtime.version, "4.0.0-rc-4");
        assert_eq!(runtime.jdk, None);
        Ok(())
    }

    #[test]
    fn malformed_output_without_version_is_rejected() {
        assert!(matches!(
            parse("Java version: 21", command()),
            Err(BuildToolRuntimeError::VersionNotFound(BuildTool::Maven))
        ));
        assert!(matches!(
            parse("Apache Maven unknown", command()),
            Err(BuildToolRuntimeError::VersionNotFound(BuildTool::Maven))
        ));
    }
}
