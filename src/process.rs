use std::{io, path::PathBuf, process::Command};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandPlatform {
    Unix,
    Windows,
}

impl CommandPlatform {
    pub const fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else {
            Self::Unix
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandRequest {
    pub executable: PathBuf,
    pub arguments: Vec<String>,
    pub current_directory: Option<PathBuf>,
    pub platform: CommandPlatform,
}

impl CommandRequest {
    pub fn new(executable: impl Into<PathBuf>, arguments: &[&str]) -> Self {
        Self {
            executable: executable.into(),
            arguments: arguments.iter().map(|value| (*value).to_string()).collect(),
            current_directory: None,
            platform: CommandPlatform::current(),
        }
    }
}

pub trait CommandExecutor {
    fn execute(&self, request: &CommandRequest) -> io::Result<CommandOutput>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommandOutput {
    pub success: bool,
    pub status: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Default)]
pub struct ProcessExecutor;

impl CommandExecutor for ProcessExecutor {
    fn execute(&self, request: &CommandRequest) -> io::Result<CommandOutput> {
        let mut command = match request.platform {
            CommandPlatform::Unix => Command::new(&request.executable),
            CommandPlatform::Windows => {
                let mut command = Command::new("cmd.exe");
                command.arg("/C").arg(&request.executable);
                command
            }
        };
        command.args(&request.arguments);
        if let Some(directory) = &request.current_directory {
            command.current_dir(directory);
        }
        command.output().map(|output| CommandOutput {
            success: output.status.success(),
            status: output.status.code(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_executable_is_reported_by_process_executor() {
        let request = CommandRequest {
            executable: PathBuf::from("jmend-command-that-does-not-exist"),
            arguments: Vec::new(),
            current_directory: None,
            platform: CommandPlatform::Unix,
        };

        let error = ProcessExecutor.execute(&request).err();

        assert!(error.is_some_and(|error| error.kind() == io::ErrorKind::NotFound));
    }
}
