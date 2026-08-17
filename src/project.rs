pub mod detector;

use std::{collections::BTreeSet, fmt, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildTool {
    Maven,
    Gradle,
}

impl fmt::Display for BuildTool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Maven => formatter.write_str("Maven"),
            Self::Gradle => formatter.write_str("Gradle"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    pub build_tool: BuildTool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProjectCapability {
    SpringBoot,
    Quarkus,
    Micronaut,
    GraalVm,
    Polyglot,
    NativeImage,
    Jpms,
    Jni,
    Ffm,
    NativeLibraries,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectContext {
    pub project: Project,
    pub capabilities: BTreeSet<ProjectCapability>,
}

impl ProjectContext {
    pub fn new(project: Project) -> Self {
        Self {
            project,
            capabilities: BTreeSet::new(),
        }
    }
}
