use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "jmend",
    version,
    about = "JVM ecosystem diagnostics with GraalVM and Native Image awareness"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Collect the current project fingerprint and run available checks.
    Check {
        /// Project path; defaults to the current directory.
        project: Option<PathBuf>,
    },
    /// Inspect JVM project or artifact evidence.
    Inspect {
        /// JVM project directory, module directory, JAR, or class file.
        path: PathBuf,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn root_and_check_help_use_clap_help_flow() {
        let root_help = Cli::try_parse_from(["jmend", "--help"]).err();
        let check_help = Cli::try_parse_from(["jmend", "check", "--help"]).err();
        let help_command = Cli::try_parse_from(["jmend", "help"]).err();

        assert_eq!(
            root_help.as_ref().map(clap::Error::kind),
            Some(ErrorKind::DisplayHelp)
        );
        assert_eq!(
            check_help.as_ref().map(clap::Error::kind),
            Some(ErrorKind::DisplayHelp)
        );
        assert_eq!(
            help_command.as_ref().map(clap::Error::kind),
            Some(ErrorKind::DisplayHelp)
        );
        assert!(root_help.is_some_and(|help| help.to_string().contains("check")));
    }

    #[test]
    fn root_help_exposes_only_implemented_commands() {
        let help = Cli::try_parse_from(["jmend", "--help"])
            .err()
            .expect("help should use clap display flow")
            .to_string();

        assert!(help.contains("check"));
        for command in ["why", "explain", "compare"] {
            assert!(!help.contains(command));
        }
        assert!(help.contains("inspect"));
    }

    #[test]
    fn version_is_concise_and_does_not_include_the_banner() {
        let version = Cli::try_parse_from(["jmend", "--version"]).err();

        assert_eq!(
            version.as_ref().map(clap::Error::kind),
            Some(ErrorKind::DisplayVersion)
        );
        assert!(version.is_some_and(|version| {
            let output = version.to_string();
            output.trim() == format!("jmend {}", env!("CARGO_PKG_VERSION"))
                && !output.contains("_ __  __")
        }));
    }

    #[test]
    fn check_path_and_standalone_inspect_are_real_commands() {
        let check = Cli::try_parse_from(["jmend", "check", "project"]).unwrap();
        assert!(
            matches!(check.command,Some(Commands::Check{project:Some(path)}) if path==*"project")
        );
        let inspect = Cli::try_parse_from(["jmend", "inspect", "app.jar"]).unwrap();
        assert!(matches!(inspect.command,Some(Commands::Inspect{path}) if path==*"app.jar"));
    }

    #[test]
    fn inspect_help_describes_the_unified_path_contract() {
        let help = Cli::try_parse_from(["jmend", "inspect", "--help"])
            .err()
            .expect("inspect help should use clap display flow")
            .to_string();
        assert!(help.contains("Inspect JVM project or artifact evidence"));
        assert!(help.contains("JVM project directory, module directory, JAR, or class file"));
        assert!(help.contains("<PATH>"));
    }
}
