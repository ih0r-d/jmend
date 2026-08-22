pub mod build_tool;
pub mod detector;
pub mod jvm_framework;
pub mod jvm_language;
pub mod jvm_target;
pub mod maven;
pub mod project_module;

use crate::runtime::JdkStatus;
use std::{collections::BTreeSet, path::PathBuf};

pub use build_tool::{BuildTool, BuildWrapper};
pub use jvm_framework::{JvmFramework, JvmFrameworkKind};
pub use jvm_language::{JvmLanguage, JvmLanguageKind};
pub use jvm_target::JvmTarget;
pub use project_module::ProjectModule;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub root: PathBuf,
    pub build_tool: BuildTool,
    pub wrapper: Option<BuildWrapper>,
    pub root_module: ProjectModule,
    /// Child build units declared by the root or nested aggregators.
    ///
    /// The root build unit is stored separately in `root_module` and is not
    /// included in this collection or its presentation count.
    pub modules: Vec<ProjectModule>,
}

impl Project {
    /// Iterates over every build unit, beginning with the root build unit.
    pub fn build_units(&self) -> impl Iterator<Item = &ProjectModule> {
        std::iter::once(&self.root_module).chain(&self.modules)
    }
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
    pub jdk: JdkStatus,
    pub capabilities: BTreeSet<ProjectCapability>,
}

impl ProjectContext {
    pub fn new(project: Project, jdk: JdkStatus) -> Self {
        Self {
            project,
            jdk,
            capabilities: BTreeSet::new(),
        }
    }
}
