use super::{BuildTool, Project, gradle_runtime, maven_runtime};
use crate::{
    process::{CommandExecutor, CommandOutput, CommandPlatform},
    runtime::JdkInfo,
};
use std::{fmt, io, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildToolSource {
    Wrapper,
    System,
}

impl fmt::Display for BuildToolSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wrapper => formatter.write_str("wrapper"),
            Self::System => formatter.write_str("system"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildToolRuntime {
    pub tool: BuildTool,
    pub version: String,
    pub executable: PathBuf,
    pub source: BuildToolSource,
    pub jdk: Option<JdkInfo>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildToolCommand {
    pub tool: BuildTool,
    pub executable: PathBuf,
    pub source: BuildToolSource,
}

pub fn select_command(project: &Project) -> BuildToolCommand {
    select_command_for(project, CommandPlatform::current())
}

fn select_command_for(project: &Project, platform: CommandPlatform) -> BuildToolCommand {
    match &project.wrapper {
        Some(wrapper) if wrapper_matches_platform(&wrapper.executable, platform) => {
            BuildToolCommand {
                tool: project.build_tool,
                executable: wrapper.executable.clone(),
                source: BuildToolSource::Wrapper,
            }
        }
        Some(_) | None => BuildToolCommand {
            tool: project.build_tool,
            executable: PathBuf::from(project.build_tool.descriptor().system_command()),
            source: BuildToolSource::System,
        },
    }
}

fn wrapper_matches_platform(executable: &std::path::Path, platform: CommandPlatform) -> bool {
    let extension = executable.extension().and_then(|value| value.to_str());
    match platform {
        CommandPlatform::Unix => !matches!(extension, Some("cmd" | "bat")),
        CommandPlatform::Windows => matches!(extension, Some("cmd" | "bat")),
    }
}

pub fn collect(
    project: &Project,
    executor: &dyn CommandExecutor,
) -> Result<BuildToolRuntime, BuildToolRuntimeError> {
    let selected = select_command(project);
    let mut request = crate::process::CommandRequest::new(&selected.executable, &["--version"]);
    request.current_directory = Some(project.root.clone());
    let output = executor
        .execute(&request)
        .map_err(BuildToolRuntimeError::Execute)?;
    require_success(output).and_then(|text| match selected.tool {
        BuildTool::Maven => maven_runtime::parse(&text, selected),
        BuildTool::Gradle => gradle_runtime::parse(&text, selected),
        BuildTool::Sbt | BuildTool::Mill | BuildTool::Ant => {
            Err(BuildToolRuntimeError::Unsupported(selected.tool))
        }
    })
}

fn require_success(output: CommandOutput) -> Result<String, BuildToolRuntimeError> {
    if !output.success {
        return Err(BuildToolRuntimeError::CommandFailed {
            status: output.status,
            stderr: output.stderr.trim().to_string(),
        });
    }

    let mut text = output.stdout;
    if !output.stderr.is_empty() {
        text.push('\n');
        text.push_str(&output.stderr);
    }
    Ok(text)
}

#[derive(Debug)]
pub enum BuildToolRuntimeError {
    Execute(io::Error),
    CommandFailed { status: Option<i32>, stderr: String },
    VersionNotFound(BuildTool),
    Unsupported(BuildTool),
}

impl fmt::Display for BuildToolRuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Execute(error) => write!(formatter, "cannot execute build tool: {error}"),
            Self::CommandFailed { status, stderr } => write!(
                formatter,
                "build tool exited with status {}: {stderr}",
                status.map_or_else(|| "unknown".to_string(), |status| status.to_string())
            ),
            Self::VersionNotFound(tool) => write!(formatter, "{tool} version was not found"),
            Self::Unsupported(tool) => {
                write!(formatter, "{tool} runtime collection is unsupported")
            }
        }
    }
}

impl std::error::Error for BuildToolRuntimeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Execute(error) => Some(error),
            Self::CommandFailed { .. } | Self::VersionNotFound(_) | Self::Unsupported(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        process::{CommandExecutor, CommandOutput, CommandRequest},
        project::{BuildWrapper, ProjectModule},
    };
    use std::{cell::RefCell, io};

    struct FakeExecutor {
        output: Option<CommandOutput>,
        error_kind: Option<io::ErrorKind>,
        requests: RefCell<Vec<CommandRequest>>,
    }

    impl FakeExecutor {
        fn succeeds(stdout: &str) -> Self {
            Self {
                output: Some(CommandOutput {
                    success: true,
                    status: Some(0),
                    stdout: stdout.to_string(),
                    stderr: String::new(),
                }),
                error_kind: None,
                requests: RefCell::new(Vec::new()),
            }
        }

        fn exits(status: i32) -> Self {
            Self {
                output: Some(CommandOutput {
                    success: false,
                    status: Some(status),
                    stdout: String::new(),
                    stderr: "failed".to_string(),
                }),
                error_kind: None,
                requests: RefCell::new(Vec::new()),
            }
        }

        fn fails_to_start(kind: io::ErrorKind) -> Self {
            Self {
                output: None,
                error_kind: Some(kind),
                requests: RefCell::new(Vec::new()),
            }
        }
    }

    impl CommandExecutor for FakeExecutor {
        fn execute(&self, request: &CommandRequest) -> io::Result<CommandOutput> {
            self.requests.borrow_mut().push(request.clone());
            match self.error_kind {
                Some(kind) => Err(io::Error::new(kind, "fake execution failure")),
                None => Ok(self.output.clone().expect("fake output configured")),
            }
        }
    }

    fn project(tool: BuildTool, wrapper: Option<&str>) -> Project {
        Project {
            root: PathBuf::from("project"),
            build_tool: tool,
            wrapper: wrapper.map(|executable| BuildWrapper {
                executable: PathBuf::from(executable),
                metadata: None,
            }),
            root_module: ProjectModule::new(PathBuf::from("project")),
            modules: Vec::new(),
        }
    }

    #[test]
    fn maven_wrapper_is_preferred_and_executed_once() -> Result<(), BuildToolRuntimeError> {
        let project = project(BuildTool::Maven, Some("project/mvnw"));
        let executor = FakeExecutor::succeeds("Apache Maven 3.9.11");

        let runtime = collect(&project, &executor)?;

        assert_eq!(runtime.source, BuildToolSource::Wrapper);
        assert_eq!(runtime.executable, PathBuf::from("project/mvnw"));
        assert_eq!(executor.requests.borrow().len(), 1);
        Ok(())
    }

    #[test]
    fn gradle_wrapper_is_preferred() -> Result<(), BuildToolRuntimeError> {
        let project = project(BuildTool::Gradle, Some("project/gradlew"));
        let executor = FakeExecutor::succeeds("Gradle 8.14.3");

        let runtime = collect(&project, &executor)?;

        assert_eq!(runtime.source, BuildToolSource::Wrapper);
        assert_eq!(runtime.executable, PathBuf::from("project/gradlew"));
        Ok(())
    }

    #[test]
    fn system_commands_are_selected_without_wrappers() {
        assert_eq!(
            select_command(&project(BuildTool::Maven, None)).executable,
            PathBuf::from("mvn")
        );
        assert_eq!(
            select_command(&project(BuildTool::Gradle, None)).executable,
            PathBuf::from("gradle")
        );
        assert_eq!(
            select_command(&project(BuildTool::Maven, None)).source,
            BuildToolSource::System
        );
    }

    #[test]
    fn windows_wrapper_paths_are_preserved_for_execution_selection() {
        assert_eq!(
            select_command_for(
                &project(BuildTool::Maven, Some("project/mvnw.cmd")),
                CommandPlatform::Windows,
            )
            .executable,
            PathBuf::from("project/mvnw.cmd")
        );
        assert_eq!(
            select_command_for(
                &project(BuildTool::Gradle, Some("project/gradlew.bat")),
                CommandPlatform::Windows,
            )
            .executable,
            PathBuf::from("project/gradlew.bat")
        );
        assert_eq!(
            select_command_for(
                &project(BuildTool::Maven, Some("project/mvnw.cmd")),
                CommandPlatform::Unix,
            )
            .source,
            BuildToolSource::System
        );
    }

    #[test]
    fn unavailable_executable_is_a_collection_error() {
        let executor = FakeExecutor::fails_to_start(io::ErrorKind::NotFound);

        assert!(matches!(
            collect(&project(BuildTool::Maven, None), &executor),
            Err(BuildToolRuntimeError::Execute(error)) if error.kind() == io::ErrorKind::NotFound
        ));
    }

    #[test]
    fn nonzero_exit_is_a_collection_error() {
        let executor = FakeExecutor::exits(7);

        assert!(matches!(
            collect(&project(BuildTool::Gradle, None), &executor),
            Err(BuildToolRuntimeError::CommandFailed {
                status: Some(7),
                ..
            })
        ));
    }
}
