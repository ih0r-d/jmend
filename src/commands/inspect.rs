use crate::{artifact::ArtifactEvidence, error::AppError, project::Project};
use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub enum Inspection {
    Project(Project),
    Artifact(ArtifactEvidence),
}

pub fn run(path: &Path) -> Result<Inspection, AppError> {
    let metadata = fs::metadata(path).map_err(|source| {
        AppError::Inspect(InspectError::Path {
            path: path.to_path_buf(),
            source,
        })
    })?;
    if metadata.is_file() {
        return crate::artifact::inspect(path)
            .map(Inspection::Artifact)
            .map_err(AppError::Artifact);
    }
    if metadata.is_dir() {
        let root = fs::canonicalize(path).map_err(|source| {
            AppError::Inspect(InspectError::Path {
                path: path.to_path_buf(),
                source,
            })
        })?;
        let mut project = crate::project::detector::detect_at(&root)?;
        crate::artifact::discovery::collect(&mut project);
        return Ok(Inspection::Project(project));
    }
    Err(AppError::Inspect(InspectError::Unsupported(
        path.to_path_buf(),
    )))
}

#[derive(Debug)]
pub enum InspectError {
    Path { path: PathBuf, source: io::Error },
    Unsupported(PathBuf),
}

impl std::fmt::Display for InspectError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Path { path, source } if source.kind() == io::ErrorKind::NotFound => {
                write!(formatter, "inspection path not found: {}", path.display())
            }
            Self::Path { path, .. } => write!(formatter, "cannot inspect path {}", path.display()),
            Self::Unsupported(path) => {
                write!(
                    formatter,
                    "unsupported inspection input: {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for InspectError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Path { source, .. } => Some(source),
            Self::Unsupported(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT: AtomicU64 = AtomicU64::new(0);

    struct TempProject(PathBuf);

    impl TempProject {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "jmend-inspect-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempProject {
        fn drop(&mut self) {
            fs::remove_dir_all(&self.0).ok();
        }
    }

    fn pom(artifact: &str, modules: &[&str]) -> String {
        let modules = modules
            .iter()
            .map(|module| format!("<module>{module}</module>"))
            .collect::<String>();
        format!(
            "<project><modelVersion>4.0.0</modelVersion><groupId>test</groupId><artifactId>{artifact}</artifactId><version>1</version><modules>{modules}</modules></project>"
        )
    }
    #[test]
    fn inspects_class_without_project_detection() {
        for release in [8, 17, 21, 25] {
            let fixture = if release == 8 {
                "tests/fixtures/java8/fixtures/LegacyDependency.class".to_string()
            } else {
                format!("tests/fixtures/java{release}/fixtures/StaticEvidence.class")
            };
            let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(fixture);
            let Inspection::Artifact(evidence) = run(&path).unwrap() else {
                panic!("expected artifact inspection")
            };
            assert_eq!(
                evidence.classes[0].version.java,
                Some(crate::artifact::JavaRelease::Standard(release))
            );
        }
    }
    #[test]
    fn unsupported_input_fails_cleanly() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(run(&path).is_err());
    }

    #[test]
    fn directory_inspection_reuses_project_and_artifact_discovery() {
        let temp = TempProject::new();
        fs::write(
            temp.0.join("pom.xml"),
            pom("root", &["api", "core", "service"]),
        )
        .unwrap();
        for module in ["api", "core", "service"] {
            fs::create_dir(temp.0.join(module)).unwrap();
            fs::write(temp.0.join(module).join("pom.xml"), pom(module, &[])).unwrap();
        }
        fs::create_dir_all(temp.0.join("target/classes")).unwrap();
        fs::create_dir_all(temp.0.join("api/target/classes")).unwrap();
        fs::create_dir_all(temp.0.join("core/target/classes")).unwrap();
        let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/java17/fixtures/StaticEvidence.class");
        fs::copy(&fixture, temp.0.join("target/classes/Root.class")).unwrap();
        fs::copy(&fixture, temp.0.join("api/target/classes/Api.class")).unwrap();
        fs::copy(&fixture, temp.0.join("core/target/classes/Core.class")).unwrap();
        fs::copy(&fixture, temp.0.join("core/target/classes/CoreTests.class")).unwrap();

        let Inspection::Project(project) = run(&temp.0).unwrap() else {
            panic!("expected project inspection")
        };
        assert_eq!(project.root_module.artifacts.len(), 1);
        assert_eq!(project.modules[0].artifacts.len(), 1);
        assert_eq!(project.modules[1].artifacts.len(), 2);
        assert!(project.modules[2].artifacts.is_empty());
    }

    #[test]
    fn child_module_path_does_not_include_siblings() {
        let temp = TempProject::new();
        fs::write(temp.0.join("pom.xml"), pom("root", &["api", "core"])).unwrap();
        for module in ["api", "core"] {
            fs::create_dir(temp.0.join(module)).unwrap();
            fs::write(temp.0.join(module).join("pom.xml"), pom(module, &[])).unwrap();
        }

        let Inspection::Project(project) = run(&temp.0.join("core")).unwrap() else {
            panic!("expected module project inspection")
        };
        assert_eq!(project.root, fs::canonicalize(temp.0.join("core")).unwrap());
        assert_eq!(project.root_module.name.as_deref(), Some("core"));
        assert!(project.modules.is_empty());
    }

    #[test]
    fn missing_unsupported_and_empty_inputs_fail_cleanly() {
        let temp = TempProject::new();
        let missing = temp.0.join("missing");
        assert!(matches!(
            run(&missing),
            Err(AppError::Inspect(InspectError::Path { .. }))
        ));

        let text = temp.0.join("file.txt");
        fs::write(&text, "text").unwrap();
        assert!(matches!(run(&text), Err(AppError::Artifact(_))));

        let empty = temp.0.join("empty");
        fs::create_dir(&empty).unwrap();
        assert!(matches!(run(&empty), Err(AppError::ProjectDetection(_))));

        for name in ["corrupt.jar", "corrupt.class"] {
            let path = temp.0.join(name);
            fs::write(&path, "corrupt").unwrap();
            assert!(matches!(run(&path), Err(AppError::Artifact(_))));
        }
    }
}
