use super::{BuildTool, BuildWrapper, Project, ProjectModule, maven};
use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

pub fn detect(start: &Path) -> Result<Project, ProjectDetectionError> {
    for candidate in start.ancestors() {
        if let Some(build_tool) = detect_build_tool(candidate)? {
            let (root_module, modules) = match build_tool {
                BuildTool::Maven => {
                    let model = maven::inspect_project(candidate)?;
                    (model.root_module, model.modules)
                }
                BuildTool::Gradle | BuildTool::Sbt | BuildTool::Mill | BuildTool::Ant => {
                    (root_module(candidate, build_tool)?, Vec::new())
                }
            };
            return Ok(Project {
                root: candidate.to_path_buf(),
                build_tool,
                wrapper: detect_wrapper(candidate, build_tool)?,
                root_module,
                modules,
            });
        }
    }

    Err(ProjectDetectionError::Unsupported(start.to_path_buf()))
}

fn root_module(root: &Path, build_tool: BuildTool) -> Result<ProjectModule, ProjectDetectionError> {
    let mut module = ProjectModule::new(root.to_path_buf());
    module.build_file = build_tool
        .descriptor()
        .build_files()
        .iter()
        .map(|name| root.join(name))
        .find_map(|path| match is_file(&path) {
            Ok(true) => Some(Ok(path)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .transpose()?;
    Ok(module)
}

fn detect_build_tool(root: &Path) -> Result<Option<BuildTool>, ProjectDetectionError> {
    let mut detected = None;

    for build_tool in BuildTool::DETECTABLE {
        if contains_any_file(root, build_tool.descriptor().build_files())? {
            if detected.is_some() {
                return Err(ProjectDetectionError::Ambiguous(root.to_path_buf()));
            }
            detected = Some(build_tool);
        }
    }

    Ok(detected)
}

fn detect_wrapper(
    root: &Path,
    build_tool: BuildTool,
) -> Result<Option<BuildWrapper>, ProjectDetectionError> {
    let descriptor = build_tool.descriptor();
    let executable = build_tool
        .wrapper_executables_for(crate::process::CommandPlatform::current())
        .iter()
        .map(|name| root.join(name))
        .find_map(|path| match is_file(&path) {
            Ok(true) => Some(Ok(path)),
            Ok(false) => None,
            Err(error) => Some(Err(error)),
        })
        .transpose()?;

    let Some(executable) = executable else {
        return Ok(None);
    };

    let metadata_path = descriptor
        .wrapper_metadata()
        .iter()
        .fold(root.to_path_buf(), |path, component| path.join(component));
    let metadata = is_file(&metadata_path)?.then_some(metadata_path);

    Ok(Some(BuildWrapper {
        executable,
        metadata,
    }))
}

fn contains_any_file(root: &Path, names: &[&str]) -> Result<bool, ProjectDetectionError> {
    names
        .iter()
        .try_fold(false, |found, name| Ok(found || is_file(&root.join(name))?))
}

fn is_file(path: &Path) -> Result<bool, ProjectDetectionError> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(ProjectDetectionError::InspectPath {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[derive(Debug)]
pub enum ProjectDetectionError {
    Unsupported(PathBuf),
    Ambiguous(PathBuf),
    InspectPath { path: PathBuf, source: io::Error },
    MavenModel(maven::MavenModelError),
}

impl fmt::Display for ProjectDetectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unsupported(root) => write!(
                formatter,
                "no supported JVM project found in {} (expected pom.xml, build.gradle, or build.gradle.kts)",
                root.display()
            ),
            Self::Ambiguous(root) => write!(
                formatter,
                "both Maven and Gradle build files found in {}; select a single project root",
                root.display()
            ),
            Self::InspectPath { path, .. } => {
                write!(formatter, "cannot inspect project path {}", path.display())
            }
            Self::MavenModel(error) => error.fmt(formatter),
        }
    }
}

impl Error for ProjectDetectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::InspectPath { source, .. } => Some(source),
            Self::MavenModel(error) => Some(error),
            Self::Unsupported(_) | Self::Ambiguous(_) => None,
        }
    }
}

impl From<maven::MavenModelError> for ProjectDetectionError {
    fn from(error: maven::MavenModelError) -> Self {
        Self::MavenModel(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs::{self, File},
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> io::Result<Self> {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir()
                .join(format!("jmend-detector-{}-{sequence}", std::process::id()));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }

        fn create_file(&self, name: &str) -> io::Result<()> {
            let path = self.0.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            if name.ends_with("pom.xml") {
                fs::write(path, "<project><artifactId>fixture</artifactId></project>")
            } else {
                File::create(path).map(drop)
            }
        }

        fn write_file(&self, name: &str, content: &str) -> io::Result<()> {
            let path = self.0.join(name);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn detects_maven_project_with_unix_wrapper() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("pom.xml")?;
        directory.create_file("mvnw")?;
        directory.create_file(".mvn/wrapper/maven-wrapper.properties")?;

        let project = detect(&directory.0)?;

        assert_eq!(project.build_tool, BuildTool::Maven);
        assert_eq!(project.root, directory.0);
        assert_eq!(
            project.wrapper,
            Some(BuildWrapper {
                executable: directory.0.join("mvnw"),
                metadata: Some(
                    directory
                        .0
                        .join(".mvn")
                        .join("wrapper")
                        .join("maven-wrapper.properties")
                ),
            })
        );
        Ok(())
    }

    #[test]
    fn detects_maven_project_with_windows_wrapper() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("pom.xml")?;
        directory.create_file("mvnw.cmd")?;

        let project = detect(&directory.0)?;

        assert_eq!(
            project.wrapper.as_ref().map(|wrapper| &wrapper.executable),
            Some(&directory.0.join("mvnw.cmd"))
        );
        assert_eq!(project.wrapper.and_then(|wrapper| wrapper.metadata), None);
        Ok(())
    }

    #[test]
    fn detects_maven_project_without_wrapper() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("pom.xml")?;

        let project = detect(&directory.0)?;

        assert_eq!(project.build_tool, BuildTool::Maven);
        assert_eq!(project.wrapper, None);
        let canonical_root = fs::canonicalize(&directory.0)?;
        assert_eq!(project.root_module.path, canonical_root);
        assert_eq!(
            project.root_module.build_file,
            Some(project.root_module.path.join("pom.xml"))
        );
        assert_eq!(project.root_module.name.as_deref(), Some("fixture"));
        assert!(project.modules.is_empty());
        Ok(())
    }

    #[test]
    fn preserves_single_module_root_target_without_creating_child_module()
    -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write_file(
            "pom.xml",
            "<project><artifactId>application</artifactId><properties><maven.compiler.release>21</maven.compiler.release></properties></project>",
        )?;

        let project = detect(&directory.0)?;

        assert_eq!(project.root_module.jvm_targets[0].version, "21");
        assert!(project.modules.is_empty());
        Ok(())
    }

    #[test]
    fn populates_declared_maven_modules() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write_file(
            "pom.xml",
            "<project><artifactId>root</artifactId><modules><module>api</module></modules></project>",
        )?;
        directory.write_file(
            "api/pom.xml",
            "<project><artifactId>public-api</artifactId><properties><maven.compiler.release>21</maven.compiler.release></properties></project>",
        )?;

        let project = detect(&directory.0)?;

        assert_eq!(project.modules.len(), 1);
        assert_eq!(project.modules[0].name.as_deref(), Some("public-api"));
        assert_eq!(project.modules[0].jvm_targets.len(), 1);
        assert_eq!(project.modules[0].jvm_targets[0].version, "21");
        Ok(())
    }

    #[test]
    fn discovers_maven_project_upward() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("pom.xml")?;
        let nested = directory.0.join("src").join("main").join("java");
        fs::create_dir_all(&nested)?;

        let project = detect(&nested)?;

        assert_eq!(project.build_tool, BuildTool::Maven);
        assert_eq!(project.root, directory.0);
        Ok(())
    }

    #[test]
    fn detects_gradle_groovy_project_with_unix_wrapper() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("build.gradle")?;
        directory.create_file("gradlew")?;
        directory.create_file("gradle/wrapper/gradle-wrapper.properties")?;

        let project = detect(&directory.0)?;

        assert_eq!(project.build_tool, BuildTool::Gradle);
        assert_eq!(project.root, directory.0);
        assert_eq!(
            project.wrapper,
            Some(BuildWrapper {
                executable: directory.0.join("gradlew"),
                metadata: Some(
                    directory
                        .0
                        .join("gradle")
                        .join("wrapper")
                        .join("gradle-wrapper.properties")
                ),
            })
        );
        Ok(())
    }

    #[test]
    fn detects_gradle_kotlin_project_with_windows_wrapper() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("build.gradle.kts")?;
        directory.create_file("gradlew.bat")?;

        let project = detect(&directory.0)?;

        assert_eq!(project.build_tool, BuildTool::Gradle);
        assert_eq!(
            project.wrapper.as_ref().map(|wrapper| &wrapper.executable),
            Some(&directory.0.join("gradlew.bat"))
        );
        Ok(())
    }

    #[test]
    fn detects_gradle_project_without_wrapper() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("build.gradle")?;

        let project = detect(&directory.0)?;

        assert_eq!(project.build_tool, BuildTool::Gradle);
        assert_eq!(project.wrapper, None);
        Ok(())
    }

    #[test]
    fn wrapper_without_build_file_is_not_a_project() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("mvnw")?;
        directory.create_file("gradlew.bat")?;

        let error = detect(&directory.0)
            .err()
            .ok_or("wrapper-only directory was detected as a project")?;

        assert!(matches!(error, ProjectDetectionError::Unsupported(_)));
        Ok(())
    }

    #[test]
    fn selects_nearest_project_root() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("pom.xml")?;
        let nested_project = directory.0.join("nested");
        fs::create_dir_all(&nested_project)?;
        File::create(nested_project.join("build.gradle.kts")).map(drop)?;
        let start = nested_project.join("src").join("main");
        fs::create_dir_all(&start)?;

        let project = detect(&start)?;

        assert_eq!(project.root, nested_project);
        assert_eq!(project.build_tool, BuildTool::Gradle);
        Ok(())
    }

    #[test]
    fn rejects_unsupported_directory_hierarchy() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let nested = directory.0.join("one").join("two").join("three");
        fs::create_dir_all(&nested)?;

        let error = detect(&nested)
            .err()
            .ok_or("detection unexpectedly succeeded")?;

        assert!(matches!(
            error,
            ProjectDetectionError::Unsupported(path) if path == nested
        ));
        Ok(())
    }

    #[test]
    fn stops_after_exhausting_parent_directories() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        let sibling_project = directory.0.join("project");
        let start = directory.0.join("unrelated").join("nested");
        fs::create_dir_all(&sibling_project)?;
        fs::create_dir_all(&start)?;
        File::create(sibling_project.join("pom.xml")).map(drop)?;

        let error = detect(&start)
            .err()
            .ok_or("detection unexpectedly searched outside the ancestor chain")?;

        assert!(matches!(error, ProjectDetectionError::Unsupported(_)));
        Ok(())
    }
}
