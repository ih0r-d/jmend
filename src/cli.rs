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
