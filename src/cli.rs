use clap::{Parser, Subcommand};

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
    Check,
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
        for command in ["why", "inspect", "explain", "compare"] {
            assert!(!help.contains(command));
        }
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
