use super::{JvmFramework, JvmLanguage, JvmTarget};
use crate::artifact::ArtifactEvidence;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectModule {
    pub name: Option<String>,
    pub path: PathBuf,
    pub build_file: Option<PathBuf>,
    pub languages: Vec<JvmLanguage>,
    pub frameworks: Vec<JvmFramework>,
    pub jvm_targets: Vec<JvmTarget>,
    /// Compiled artifacts discovered for this build unit. Zero, one, or many are valid.
    pub artifacts: Vec<ArtifactEvidence>,
}

impl ProjectModule {
    pub fn new(path: PathBuf) -> Self {
        Self {
            name: None,
            path,
            build_file: None,
            languages: Vec::new(),
            frameworks: Vec::new(),
            jvm_targets: Vec::new(),
            artifacts: Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        project::{BuildTool, JvmLanguageKind, JvmTarget, Project, ProjectContext},
        runtime::{JdkInfo, JdkStatus},
    };

    #[test]
    fn module_target_is_independent_from_runtime_jdk() {
        let mut module = ProjectModule::new(PathBuf::from("project/module"));
        module
            .jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, "21"));
        let context = ProjectContext::new(
            Project {
                root: PathBuf::from("project"),
                build_tool: BuildTool::Maven,
                wrapper: None,
                root_module: ProjectModule::new(PathBuf::from("project")),
                modules: vec![module],
            },
            JdkStatus::Detected(JdkInfo {
                version: "17".to_string(),
                vendor: None,
                runtime: None,
            }),
        );

        assert_eq!(context.project.modules[0].jvm_targets[0].version, "21");
        assert!(matches!(
            context.jdk,
            JdkStatus::Detected(JdkInfo { ref version, .. }) if version == "17"
        ));
    }

    #[test]
    fn declared_target_and_actual_bytecode_are_independent_evidence() {
        let mut module = ProjectModule::new(PathBuf::from("project"));
        module
            .jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, "17"));
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/java25/fixtures/StaticEvidence.class");
        module
            .artifacts
            .push(crate::artifact::inspect(&path).unwrap());
        assert_eq!(module.jvm_targets[0].version, "17");
        assert_eq!(
            module.artifacts[0].classes[0].version.java,
            Some(crate::artifact::JavaRelease::Standard(25))
        );
    }
}
