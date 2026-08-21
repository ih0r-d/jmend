use super::ProjectModule;
use quick_xml::{Reader, escape::unescape, events::Event};
use std::{
    collections::HashSet,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

const POM_FILE: &str = "pom.xml";

pub fn discover_modules(root: &Path) -> Result<Vec<ProjectModule>, MavenModelError> {
    let root = canonicalize(root)?;
    let root_pom = root.join(POM_FILE);
    let model = read_pom(&root_pom)?;
    let mut state = DiscoveryState {
        modules: Vec::new(),
        discovered: HashSet::new(),
        active: HashSet::from([root]),
    };

    discover_declared(&root_pom, &model.modules, &mut state)?;
    Ok(state.modules)
}

struct DiscoveryState {
    modules: Vec<ProjectModule>,
    discovered: HashSet<PathBuf>,
    active: HashSet<PathBuf>,
}

fn discover_declared(
    declaring_pom: &Path,
    declarations: &[String],
    state: &mut DiscoveryState,
) -> Result<(), MavenModelError> {
    let declaring_directory = declaring_pom
        .parent()
        .ok_or_else(|| MavenModelError::ParsePom {
            path: declaring_pom.to_path_buf(),
            message: "POM has no parent directory".to_string(),
        })?;

    for declaration in declarations {
        let module_path = declaring_directory.join(Path::new(declaration));
        require_directory(&module_path, declaring_pom)?;
        let module_path = canonicalize(&module_path)?;

        if state.active.contains(&module_path) {
            return Err(MavenModelError::Cycle { path: module_path });
        }
        if !state.discovered.insert(module_path.clone()) {
            continue;
        }

        let pom_path = module_path.join(POM_FILE);
        require_pom(&pom_path, declaring_pom)?;
        let model = read_pom(&pom_path)?;

        let mut module = ProjectModule::new(module_path.clone());
        module.name = model.artifact_id;
        module.build_file = Some(pom_path.clone());
        state.modules.push(module);

        state.active.insert(module_path.clone());
        let result = discover_declared(&pom_path, &model.modules, state);
        state.active.remove(&module_path);
        result?;
    }

    Ok(())
}

fn require_directory(path: &Path, declaring_pom: &Path) -> Result<(), MavenModelError> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(MavenModelError::MissingModuleDirectory {
            path: path.to_path_buf(),
            declared_by: declaring_pom.to_path_buf(),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(MavenModelError::MissingModuleDirectory {
                path: path.to_path_buf(),
                declared_by: declaring_pom.to_path_buf(),
            })
        }
        Err(source) => Err(MavenModelError::ReadPath {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn require_pom(path: &Path, declaring_pom: &Path) -> Result<(), MavenModelError> {
    match fs::metadata(path) {
        Ok(metadata) if metadata.is_file() => Ok(()),
        Ok(_) => Err(MavenModelError::MissingModulePom {
            path: path.to_path_buf(),
            declared_by: declaring_pom.to_path_buf(),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            Err(MavenModelError::MissingModulePom {
                path: path.to_path_buf(),
                declared_by: declaring_pom.to_path_buf(),
            })
        }
        Err(source) => Err(MavenModelError::ReadPath {
            path: path.to_path_buf(),
            source,
        }),
    }
}

fn canonicalize(path: &Path) -> Result<PathBuf, MavenModelError> {
    fs::canonicalize(path).map_err(|source| MavenModelError::ReadPath {
        path: path.to_path_buf(),
        source,
    })
}

#[derive(Debug, Default)]
struct PomStructure {
    artifact_id: Option<String>,
    modules: Vec<String>,
}

#[derive(Clone, Copy)]
enum Capture {
    ArtifactId,
    Module,
}

fn read_pom(path: &Path) -> Result<PomStructure, MavenModelError> {
    let xml = fs::read(path).map_err(|source| MavenModelError::ReadPath {
        path: path.to_path_buf(),
        source,
    })?;
    let mut reader = Reader::from_reader(xml.as_slice());
    reader.config_mut().trim_text(true);
    let mut buffer = Vec::new();
    let mut depth = 0_u32;
    let mut modules_depth = None;
    let mut capture = None;
    let mut captured = String::new();
    let mut structure = PomStructure::default();
    let mut saw_project = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                depth += 1;
                let name = element.local_name();
                if name.as_ref() == b"project" && depth == 1 {
                    saw_project = true;
                } else if name.as_ref() == b"modules" && depth == 2 {
                    modules_depth = Some(depth);
                } else if name.as_ref() == b"artifactId" && depth == 2 {
                    capture = Some(Capture::ArtifactId);
                    captured.clear();
                } else if name.as_ref() == b"module" && modules_depth == Some(2) && depth == 3 {
                    capture = Some(Capture::Module);
                    captured.clear();
                }
            }
            Ok(Event::Text(text)) if capture.is_some() => {
                let decoded = text.decode().map_err(|error| MavenModelError::ParsePom {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?;
                captured.push_str(&unescape(&decoded).map_err(|error| {
                    MavenModelError::ParsePom {
                        path: path.to_path_buf(),
                        message: error.to_string(),
                    }
                })?);
            }
            Ok(Event::CData(text)) if capture.is_some() => {
                captured.push_str(&text.decode().map_err(|error| MavenModelError::ParsePom {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                })?);
            }
            Ok(Event::Empty(element))
                if element.local_name().as_ref() == b"module"
                    && modules_depth == Some(2)
                    && depth == 2 =>
            {
                return Err(MavenModelError::ParsePom {
                    path: path.to_path_buf(),
                    message: "empty Maven module declaration".to_string(),
                });
            }
            Ok(Event::End(element)) => {
                let name = element.local_name();
                if name.as_ref() == b"artifactId"
                    && depth == 2
                    && matches!(capture, Some(Capture::ArtifactId))
                {
                    structure.artifact_id = non_empty(&captured);
                    capture = None;
                } else if name.as_ref() == b"module"
                    && depth == 3
                    && matches!(capture, Some(Capture::Module))
                {
                    let module = non_empty(&captured).ok_or_else(|| MavenModelError::ParsePom {
                        path: path.to_path_buf(),
                        message: "empty Maven module declaration".to_string(),
                    })?;
                    structure.modules.push(module);
                    capture = None;
                } else if name.as_ref() == b"modules" && depth == 2 {
                    modules_depth = None;
                }
                depth = depth.saturating_sub(1);
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(error) => {
                return Err(MavenModelError::ParsePom {
                    path: path.to_path_buf(),
                    message: error.to_string(),
                });
            }
        }
        buffer.clear();
    }

    if depth != 0 {
        return Err(MavenModelError::ParsePom {
            path: path.to_path_buf(),
            message: "unexpected end of Maven POM".to_string(),
        });
    }
    if !saw_project {
        return Err(MavenModelError::ParsePom {
            path: path.to_path_buf(),
            message: "missing Maven project root element".to_string(),
        });
    }

    Ok(structure)
}

fn non_empty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[derive(Debug)]
pub enum MavenModelError {
    ReadPath { path: PathBuf, source: io::Error },
    ParsePom { path: PathBuf, message: String },
    MissingModuleDirectory { path: PathBuf, declared_by: PathBuf },
    MissingModulePom { path: PathBuf, declared_by: PathBuf },
    Cycle { path: PathBuf },
}

impl fmt::Display for MavenModelError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ReadPath { path, .. } => {
                write!(
                    formatter,
                    "cannot read Maven project path {}",
                    path.display()
                )
            }
            Self::ParsePom { path, message } => {
                write!(
                    formatter,
                    "cannot parse Maven POM {}: {message}",
                    path.display()
                )
            }
            Self::MissingModuleDirectory { path, declared_by } => write!(
                formatter,
                "Maven module directory {} declared by {} is missing or is not a directory",
                path.display(),
                declared_by.display()
            ),
            Self::MissingModulePom { path, declared_by } => write!(
                formatter,
                "Maven module POM {} declared by {} is missing or is not a file",
                path.display(),
                declared_by.display()
            ),
            Self::Cycle { path } => {
                write!(
                    formatter,
                    "cyclic Maven module declaration at {}",
                    path.display()
                )
            }
        }
    }
}

impl Error for MavenModelError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::ReadPath { source, .. } => Some(source),
            Self::ParsePom { .. }
            | Self::MissingModuleDirectory { .. }
            | Self::MissingModulePom { .. }
            | Self::Cycle { .. } => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    struct TestDirectory(PathBuf);

    impl TestDirectory {
        fn new() -> io::Result<Self> {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path =
                std::env::temp_dir().join(format!("jmend-maven-{}-{sequence}", std::process::id()));
            fs::create_dir(&path)?;
            Ok(Self(path))
        }

        fn write(&self, relative: &str, content: &str) -> io::Result<()> {
            let path = self.0.join(relative);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(path, content)
        }

        fn create_directory(&self, relative: &str) -> io::Result<()> {
            fs::create_dir_all(self.0.join(relative))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn pom(artifact_id: &str, modules: &[&str]) -> String {
        let modules = modules
            .iter()
            .map(|module| format!("<module>{module}</module>"))
            .collect::<String>();
        let modules = (!modules.is_empty()).then(|| format!("<modules>{modules}</modules>"));
        format!(
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"><modelVersion>4.0.0</modelVersion><artifactId>{artifact_id}</artifactId>{}</project>",
            modules.unwrap_or_default()
        )
    }

    #[test]
    fn project_without_declared_modules_is_empty() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &[]))?;

        assert!(discover_modules(&directory.0)?.is_empty());
        Ok(())
    }

    #[test]
    fn discovers_one_declared_module_and_artifact_id() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["api"]))?;
        directory.write("api/pom.xml", &pom("public-api", &[]))?;

        let modules = discover_modules(&directory.0)?;

        assert_eq!(modules.len(), 1);
        assert_eq!(modules[0].name.as_deref(), Some("public-api"));
        assert_eq!(modules[0].path, fs::canonicalize(directory.0.join("api"))?);
        assert_eq!(modules[0].build_file, Some(modules[0].path.join(POM_FILE)));
        assert!(modules[0].languages.is_empty());
        assert!(modules[0].frameworks.is_empty());
        assert_eq!(modules[0].target_runtime, None);
        Ok(())
    }

    #[test]
    fn discovers_multiple_and_nested_path_modules() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["services/api", "library"]))?;
        directory.write("services/api/pom.xml", &pom("api", &[]))?;
        directory.write("library/pom.xml", &pom("library", &[]))?;

        let modules = discover_modules(&directory.0)?;

        assert_eq!(modules.len(), 2);
        assert_eq!(
            modules[0].path,
            fs::canonicalize(directory.0.join("services/api"))?
        );
        Ok(())
    }

    #[test]
    fn discovers_nested_aggregator_and_children() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["api"]))?;
        directory.write("api/pom.xml", &pom("api", &["model", "annotations"]))?;
        directory.write("api/model/pom.xml", &pom("model", &[]))?;
        directory.write("api/annotations/pom.xml", &pom("annotations", &[]))?;

        let modules = discover_modules(&directory.0)?;

        assert_eq!(modules.len(), 3);
        assert_eq!(
            modules
                .iter()
                .filter_map(|module| module.name.as_deref())
                .collect::<Vec<_>>(),
            ["api", "model", "annotations"]
        );
        let root = fs::canonicalize(&directory.0)?;
        assert!(modules.iter().all(|module| module.path != root));
        Ok(())
    }

    #[test]
    fn ignores_undeclared_nested_pom_and_profile_modules() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(
            POM_FILE,
            "<project><artifactId>root</artifactId><profiles><profile><modules><module>profile-only</module></modules></profile></profiles></project>",
        )?;
        directory.write("unrelated/pom.xml", &pom("unrelated", &[]))?;
        directory.write("profile-only/pom.xml", &pom("profile-only", &[]))?;

        assert!(discover_modules(&directory.0)?.is_empty());
        Ok(())
    }

    #[test]
    fn duplicate_declaration_is_represented_once() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["api", "api"]))?;
        directory.write("api/pom.xml", &pom("api", &[]))?;

        assert_eq!(discover_modules(&directory.0)?.len(), 1);
        Ok(())
    }

    #[test]
    fn module_reachable_from_two_aggregators_is_represented_once() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["first", "second"]))?;
        directory.write("first/pom.xml", &pom("first", &["../shared"]))?;
        directory.write("second/pom.xml", &pom("second", &["../shared"]))?;
        directory.write("shared/pom.xml", &pom("shared", &[]))?;

        let modules = discover_modules(&directory.0)?;

        assert_eq!(modules.len(), 3);
        assert_eq!(
            modules
                .iter()
                .filter(|module| module.name.as_deref() == Some("shared"))
                .count(),
            1
        );
        Ok(())
    }

    #[test]
    fn missing_module_directory_is_an_error() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["missing"]))?;

        let error = discover_modules(&directory.0)
            .err()
            .ok_or("missing module directory was accepted")?;

        let expected = fs::canonicalize(&directory.0)?.join("missing");
        assert!(
            matches!(error, MavenModelError::MissingModuleDirectory { path, .. } if path == expected)
        );
        Ok(())
    }

    #[test]
    fn missing_module_pom_is_an_error() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["api"]))?;
        directory.create_directory("api")?;

        let error = discover_modules(&directory.0)
            .err()
            .ok_or("missing module POM was accepted")?;

        let expected = fs::canonicalize(directory.0.join("api"))?.join(POM_FILE);
        assert!(matches!(
            error,
            MavenModelError::MissingModulePom { path, .. } if path == expected
        ));
        Ok(())
    }

    #[test]
    fn malformed_root_pom_is_an_error() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, "<project><modules>")?;

        let expected = fs::canonicalize(&directory.0)?.join(POM_FILE);
        assert!(matches!(
            discover_modules(&directory.0),
            Err(MavenModelError::ParsePom { path, .. }) if path == expected
        ));
        Ok(())
    }

    #[test]
    fn malformed_child_pom_is_an_error() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["api"]))?;
        directory.write("api/pom.xml", "<project><artifactId>")?;

        let expected = fs::canonicalize(directory.0.join("api"))?.join(POM_FILE);
        assert!(matches!(
            discover_modules(&directory.0),
            Err(MavenModelError::ParsePom { path, .. }) if path == expected
        ));
        Ok(())
    }

    #[test]
    fn cyclic_module_declaration_is_an_error() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["api"]))?;
        directory.write("api/pom.xml", &pom("api", &[".."]))?;

        assert!(matches!(
            discover_modules(&directory.0),
            Err(MavenModelError::Cycle { path }) if path == fs::canonicalize(&directory.0)?
        ));
        Ok(())
    }
}
