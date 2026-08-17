use super::{BuildTool, Project};
use std::{
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

const MAVEN_BUILD_FILE: &str = "pom.xml";
const GRADLE_BUILD_FILES: [&str; 2] = ["build.gradle", "build.gradle.kts"];

pub fn detect(root: &Path) -> Result<Project, ProjectDetectionError> {
    let has_maven = is_file(&root.join(MAVEN_BUILD_FILE))?;
    let has_gradle = GRADLE_BUILD_FILES
        .iter()
        .try_fold(false, |found, name| Ok(found || is_file(&root.join(name))?))?;

    let build_tool = match (has_maven, has_gradle) {
        (true, false) => BuildTool::Maven,
        (false, true) => BuildTool::Gradle,
        (false, false) => return Err(ProjectDetectionError::Unsupported(root.to_path_buf())),
        (true, true) => return Err(ProjectDetectionError::Ambiguous(root.to_path_buf())),
    };

    Ok(Project {
        root: root.to_path_buf(),
        build_tool,
    })
}

fn is_file(path: &Path) -> Result<bool, ProjectDetectionError> {
    match fs::metadata(path) {
        Ok(metadata) => Ok(metadata.is_file()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(source) => Err(ProjectDetectionError::ReadBuildFile {
            path: path.to_path_buf(),
            source,
        }),
    }
}

#[derive(Debug)]
pub enum ProjectDetectionError {
    Unsupported(PathBuf),
    Ambiguous(PathBuf),
    ReadBuildFile { path: PathBuf, source: io::Error },
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
            Self::ReadBuildFile { path, .. } => {
                write!(formatter, "cannot inspect build file {}", path.display())
            }
        }
    }
}

impl Error for ProjectDetectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadBuildFile { source, .. } => Some(source),
            Self::Unsupported(_) | Self::Ambiguous(_) => None,
        }
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
            let path = std::env::temp_dir().join(format!(
                "jdoctor-detector-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }

        fn create_file(&self, name: &str) -> io::Result<()> {
            File::create(self.0.join(name)).map(drop)
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn detects_maven_project() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file(MAVEN_BUILD_FILE)?;

        let project = detect(&directory.0)?;

        assert_eq!(project.build_tool, BuildTool::Maven);
        assert_eq!(project.root, directory.0);
        Ok(())
    }

    #[test]
    fn detects_gradle_groovy_project() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("build.gradle")?;

        let project = detect(&directory.0)?;

        assert_eq!(project.build_tool, BuildTool::Gradle);
        Ok(())
    }

    #[test]
    fn detects_gradle_kotlin_project() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.create_file("build.gradle.kts")?;

        let project = detect(&directory.0)?;

        assert_eq!(project.build_tool, BuildTool::Gradle);
        Ok(())
    }

    #[test]
    fn rejects_unsupported_directory() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;

        let error = detect(&directory.0)
            .err()
            .ok_or("detection unexpectedly succeeded")?;

        assert!(matches!(error, ProjectDetectionError::Unsupported(_)));
        Ok(())
    }
}
