use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "jmend",
    version,
    about = "JVM and GraalVM diagnostics, compatibility checks, and project analysis"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Detect the current project and run available checks.
    Check,

    /// Explain why a dependency is present.
    Why { dependency: String },

    /// Inspect a JAR file.
    Inspect { jar: PathBuf },

    /// Explain a JVM diagnostic input.
    Explain { input: PathBuf },

    /// Compare two JVM artifacts.
    Compare { source: PathBuf, target: PathBuf },
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    #[test]
    fn root_and_check_help_use_clap_help_flow() {
        let root_help = Cli::try_parse_from(["jmend", "--help"]).err();
        let check_help = Cli::try_parse_from(["jmend", "check", "--help"]).err();

        assert_eq!(
            root_help.as_ref().map(clap::Error::kind),
            Some(ErrorKind::DisplayHelp)
        );
        assert_eq!(
            check_help.as_ref().map(clap::Error::kind),
            Some(ErrorKind::DisplayHelp)
        );
        assert!(root_help.is_some_and(|help| help.to_string().contains("check")));
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
}
