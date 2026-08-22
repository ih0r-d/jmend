use super::{JvmLanguageKind, JvmTarget, ProjectModule};
use quick_xml::{Reader, escape::unescape, events::Event};
use std::{
    collections::HashSet,
    error::Error,
    fmt, fs, io,
    path::{Path, PathBuf},
};

const POM_FILE: &str = "pom.xml";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MavenProjectModel {
    pub root_module: ProjectModule,
    pub modules: Vec<ProjectModule>,
}

pub fn inspect_project(root: &Path) -> Result<MavenProjectModel, MavenModelError> {
    let root = canonicalize(root)?;
    let root_pom = root.join(POM_FILE);
    let model = read_pom(&root_pom)?;
    let root_module = module_from_model(root.clone(), root_pom.clone(), &model);
    let mut state = DiscoveryState {
        modules: Vec::new(),
        discovered: HashSet::new(),
        active: HashSet::from([root]),
    };

    discover_declared(&root_pom, &model.modules, &mut state)?;
    Ok(MavenProjectModel {
        root_module,
        modules: state.modules,
    })
}

pub fn discover_modules(root: &Path) -> Result<Vec<ProjectModule>, MavenModelError> {
    Ok(inspect_project(root)?.modules)
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

        let module = module_from_model(module_path.clone(), pom_path.clone(), &model);
        state.modules.push(module);

        state.active.insert(module_path.clone());
        let result = discover_declared(&pom_path, &model.modules, state);
        state.active.remove(&module_path);
        result?;
    }

    Ok(())
}

fn module_from_model(path: PathBuf, pom_path: PathBuf, model: &PomStructure) -> ProjectModule {
    let mut module = ProjectModule::new(path);
    module.name.clone_from(&model.artifact_id);
    module.build_file = Some(pom_path);
    if let Some(version) = model.jvm_target() {
        module
            .jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, version));
    }
    module
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
    compiler: MavenCompilerConfiguration,
}

impl PomStructure {
    /// Deterministic direct-POM precedence:
    /// plugin release > release property > plugin target > target property.
    fn jvm_target(&self) -> Option<String> {
        [
            self.compiler.plugin_release.as_deref(),
            self.compiler.release_property.as_deref(),
            self.compiler.plugin_target.as_deref(),
            self.compiler.target_property.as_deref(),
        ]
        .into_iter()
        .flatten()
        .next()
        .and_then(|value| resolve_target(value, &self.compiler))
    }
}

#[derive(Debug, Default)]
struct MavenCompilerConfiguration {
    release_property: Option<String>,
    target_property: Option<String>,
    plugin_release: Option<String>,
    plugin_target: Option<String>,
}

#[derive(Clone, Copy)]
enum Capture {
    ArtifactId,
    Module,
    ReleaseProperty,
    TargetProperty,
    PluginArtifactId,
    PluginRelease,
    PluginTarget,
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
    let mut element_path = Vec::<Vec<u8>>::new();
    let mut plugin_artifact_id = None;
    let mut plugin_release = None;
    let mut plugin_target = None;
    let mut capture = None;
    let mut captured = String::new();
    let mut structure = PomStructure::default();
    let mut saw_project = false;

    loop {
        match reader.read_event_into(&mut buffer) {
            Ok(Event::Start(element)) => {
                depth += 1;
                let name = element.local_name().as_ref().to_vec();
                element_path.push(name.clone());
                if name == b"project" && depth == 1 {
                    saw_project = true;
                } else if name == b"modules" && depth == 2 {
                    modules_depth = Some(depth);
                } else if name == b"artifactId" && depth == 2 {
                    capture = Some(Capture::ArtifactId);
                    captured.clear();
                } else if name == b"module" && modules_depth == Some(2) && depth == 3 {
                    capture = Some(Capture::Module);
                    captured.clear();
                } else if path_is(
                    &element_path,
                    &[b"project", b"properties", b"maven.compiler.release"],
                ) {
                    capture = Some(Capture::ReleaseProperty);
                    captured.clear();
                } else if path_is(
                    &element_path,
                    &[b"project", b"properties", b"maven.compiler.target"],
                ) {
                    capture = Some(Capture::TargetProperty);
                    captured.clear();
                } else if in_build_plugin(&element_path) && name == b"artifactId" {
                    capture = Some(Capture::PluginArtifactId);
                    captured.clear();
                } else if in_compiler_configuration(&element_path, b"release") {
                    capture = Some(Capture::PluginRelease);
                    captured.clear();
                } else if in_compiler_configuration(&element_path, b"target") {
                    capture = Some(Capture::PluginTarget);
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
                let name = element.local_name().as_ref().to_vec();
                if name == b"artifactId"
                    && depth == 2
                    && matches!(capture, Some(Capture::ArtifactId))
                {
                    structure.artifact_id = non_empty(&captured);
                    capture = None;
                } else if name == b"module"
                    && depth == 3
                    && matches!(capture, Some(Capture::Module))
                {
                    let module = non_empty(&captured).ok_or_else(|| MavenModelError::ParsePom {
                        path: path.to_path_buf(),
                        message: "empty Maven module declaration".to_string(),
                    })?;
                    structure.modules.push(module);
                    capture = None;
                } else if name == b"maven.compiler.release"
                    && matches!(capture, Some(Capture::ReleaseProperty))
                {
                    structure.compiler.release_property = non_empty(&captured);
                    capture = None;
                } else if name == b"maven.compiler.target"
                    && matches!(capture, Some(Capture::TargetProperty))
                {
                    structure.compiler.target_property = non_empty(&captured);
                    capture = None;
                } else if name == b"artifactId"
                    && matches!(capture, Some(Capture::PluginArtifactId))
                {
                    plugin_artifact_id = non_empty(&captured);
                    capture = None;
                } else if name == b"release" && matches!(capture, Some(Capture::PluginRelease)) {
                    plugin_release = non_empty(&captured);
                    capture = None;
                } else if name == b"target" && matches!(capture, Some(Capture::PluginTarget)) {
                    plugin_target = non_empty(&captured);
                    capture = None;
                } else if name == b"plugin" && in_build_plugin(&element_path) {
                    if plugin_artifact_id.as_deref() == Some("maven-compiler-plugin") {
                        structure.compiler.plugin_release = plugin_release.take();
                        structure.compiler.plugin_target = plugin_target.take();
                    }
                    plugin_artifact_id = None;
                    plugin_release = None;
                    plugin_target = None;
                } else if name == b"modules" && depth == 2 {
                    modules_depth = None;
                }
                element_path.pop();
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

fn path_is(path: &[Vec<u8>], expected: &[&[u8]]) -> bool {
    path.len() == expected.len()
        && path
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual.as_slice() == *expected)
}

fn in_build_plugin(path: &[Vec<u8>]) -> bool {
    path.len() >= 4
        && path[0].as_slice() == b"project"
        && path[1].as_slice() == b"build"
        && path[2].as_slice() == b"plugins"
        && path[3].as_slice() == b"plugin"
}

fn in_compiler_configuration(path: &[Vec<u8>], setting: &[u8]) -> bool {
    path_is(
        path,
        &[
            b"project",
            b"build",
            b"plugins",
            b"plugin",
            b"configuration",
            setting,
        ],
    )
}

fn resolve_target(value: &str, compiler: &MavenCompilerConfiguration) -> Option<String> {
    let value = value.trim();
    let resolved = match value {
        "${maven.compiler.release}" => compiler.release_property.as_deref()?,
        "${maven.compiler.target}" => compiler.target_property.as_deref()?,
        _ if value.contains("${") => return None,
        _ => value,
    };

    is_java_target(resolved).then(|| resolved.to_string())
}

fn is_java_target(value: &str) -> bool {
    let numeric = value.strip_prefix("1.").unwrap_or(value);
    !numeric.is_empty() && numeric.bytes().all(|byte| byte.is_ascii_digit())
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
    use crate::project::JvmTarget;
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

    fn pom_with_body(artifact_id: &str, body: &str) -> String {
        format!(
            "<project xmlns=\"http://maven.apache.org/POM/4.0.0\"><modelVersion>4.0.0</modelVersion><artifactId>{artifact_id}</artifactId>{body}</project>"
        )
    }

    fn compiler_plugin(configuration: &str) -> String {
        format!(
            "<build><plugins><plugin><groupId>org.apache.maven.plugins</groupId><artifactId>maven-compiler-plugin</artifactId><configuration>{configuration}</configuration></plugin></plugins></build>"
        )
    }

    fn discover_single_target(body: &str) -> Result<Option<JvmTarget>, Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &["module"]))?;
        directory.write("module/pom.xml", &pom_with_body("module", body))?;

        Ok(discover_modules(&directory.0)?
            .into_iter()
            .next()
            .and_then(|module| module.jvm_targets.into_iter().next()))
    }

    #[test]
    fn project_without_declared_modules_is_empty() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(POM_FILE, &pom("root", &[]))?;

        let project = inspect_project(&directory.0)?;

        assert!(project.modules.is_empty());
        assert_eq!(project.root_module.name.as_deref(), Some("root"));
        assert_eq!(project.root_module.path, fs::canonicalize(&directory.0)?);
        assert_eq!(
            project.root_module.build_file,
            Some(fs::canonicalize(&directory.0)?.join(POM_FILE))
        );
        assert!(project.root_module.jvm_targets.is_empty());
        Ok(())
    }

    #[test]
    fn collects_release_property_for_single_module_root() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(
            POM_FILE,
            &pom_with_body(
                "application",
                "<properties><maven.compiler.release>21</maven.compiler.release></properties>",
            ),
        )?;

        let project = inspect_project(&directory.0)?;

        assert_eq!(
            project.root_module.jvm_targets,
            [JvmTarget::new(JvmLanguageKind::Java, "21")]
        );
        assert!(project.modules.is_empty());
        Ok(())
    }

    #[test]
    fn collects_plugin_release_for_single_module_root() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(
            POM_FILE,
            &pom_with_body("application", &compiler_plugin("<release>17</release>")),
        )?;

        let project = inspect_project(&directory.0)?;

        assert_eq!(
            project.root_module.jvm_targets,
            [JvmTarget::new(JvmLanguageKind::Java, "17")]
        );
        assert!(project.modules.is_empty());
        Ok(())
    }

    #[test]
    fn collects_target_property_for_single_module_root() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(
            POM_FILE,
            &pom_with_body(
                "application",
                "<properties><maven.compiler.target>11</maven.compiler.target></properties>",
            ),
        )?;

        let project = inspect_project(&directory.0)?;

        assert_eq!(
            project.root_module.jvm_targets,
            [JvmTarget::new(JvmLanguageKind::Java, "11")]
        );
        assert!(project.modules.is_empty());
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
        assert!(modules[0].jvm_targets.is_empty());
        Ok(())
    }

    #[test]
    fn collects_maven_compiler_release_property() -> Result<(), Box<dyn Error>> {
        let target = discover_single_target(
            "<properties><maven.compiler.release>21</maven.compiler.release></properties>",
        )?;

        assert_eq!(target, Some(JvmTarget::new(JvmLanguageKind::Java, "21")));
        Ok(())
    }

    #[test]
    fn collects_maven_compiler_plugin_release() -> Result<(), Box<dyn Error>> {
        let target = discover_single_target(&compiler_plugin("<release>17</release>"))?;

        assert_eq!(target, Some(JvmTarget::new(JvmLanguageKind::Java, "17")));
        Ok(())
    }

    #[test]
    fn collects_maven_compiler_target_property() -> Result<(), Box<dyn Error>> {
        let target = discover_single_target(
            "<properties><maven.compiler.target>11</maven.compiler.target></properties>",
        )?;

        assert_eq!(target, Some(JvmTarget::new(JvmLanguageKind::Java, "11")));
        Ok(())
    }

    #[test]
    fn collects_maven_compiler_plugin_target() -> Result<(), Box<dyn Error>> {
        let target = discover_single_target(&compiler_plugin("<target>1.8</target>"))?;

        assert_eq!(target, Some(JvmTarget::new(JvmLanguageKind::Java, "1.8")));
        Ok(())
    }

    #[test]
    fn plugin_release_has_precedence_over_all_other_declarations() -> Result<(), Box<dyn Error>> {
        let body = format!(
            "<properties><maven.compiler.release>20</maven.compiler.release><maven.compiler.target>11</maven.compiler.target></properties>{}",
            compiler_plugin("<release>21</release><target>17</target>")
        );

        assert_eq!(
            discover_single_target(&body)?,
            Some(JvmTarget::new(JvmLanguageKind::Java, "21"))
        );
        Ok(())
    }

    #[test]
    fn release_property_has_precedence_over_plugin_target() -> Result<(), Box<dyn Error>> {
        let body = format!(
            "<properties><maven.compiler.release>21</maven.compiler.release></properties>{}",
            compiler_plugin("<target>17</target>")
        );

        assert_eq!(
            discover_single_target(&body)?,
            Some(JvmTarget::new(JvmLanguageKind::Java, "21"))
        );
        Ok(())
    }

    #[test]
    fn plugin_target_has_precedence_over_target_property() -> Result<(), Box<dyn Error>> {
        let body = format!(
            "<properties><maven.compiler.target>11</maven.compiler.target></properties>{}",
            compiler_plugin("<target>17</target>")
        );

        assert_eq!(
            discover_single_target(&body)?,
            Some(JvmTarget::new(JvmLanguageKind::Java, "17"))
        );
        Ok(())
    }

    #[test]
    fn resolves_compiler_plugin_reference_to_supported_target_property()
    -> Result<(), Box<dyn Error>> {
        let body = format!(
            "<properties><maven.compiler.release>21</maven.compiler.release></properties>{}",
            compiler_plugin("<release>${maven.compiler.release}</release>")
        );

        assert_eq!(
            discover_single_target(&body)?,
            Some(JvmTarget::new(JvmLanguageKind::Java, "21"))
        );
        Ok(())
    }

    #[test]
    fn preserves_different_and_missing_targets_per_module() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(
            POM_FILE,
            &pom_with_body(
                "root",
                "<properties><maven.compiler.release>21</maven.compiler.release></properties><modules><module>module-a</module><module>module-b</module><module>module-c</module></modules>",
            ),
        )?;
        directory.write(
            "module-a/pom.xml",
            &pom_with_body(
                "module-a",
                "<properties><maven.compiler.release>17</maven.compiler.release></properties>",
            ),
        )?;
        directory.write(
            "module-b/pom.xml",
            &pom_with_body("module-b", &compiler_plugin("<target>21</target>")),
        )?;
        directory.write("module-c/pom.xml", &pom("module-c", &[]))?;

        let project = inspect_project(&directory.0)?;

        assert_eq!(
            project.root_module.jvm_targets,
            [JvmTarget::new(JvmLanguageKind::Java, "21")]
        );
        assert_eq!(
            project.modules[0].jvm_targets,
            [JvmTarget::new(JvmLanguageKind::Java, "17")]
        );
        assert_eq!(
            project.modules[1].jvm_targets,
            [JvmTarget::new(JvmLanguageKind::Java, "21")]
        );
        assert!(project.modules[2].jvm_targets.is_empty());
        assert_eq!(project.modules.len(), 3);
        Ok(())
    }

    #[test]
    fn aggregation_does_not_propagate_root_target_to_child() -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(
            POM_FILE,
            &pom_with_body(
                "root",
                "<properties><maven.compiler.release>21</maven.compiler.release></properties><modules><module>child</module></modules>",
            ),
        )?;
        directory.write("child/pom.xml", &pom("child", &[]))?;

        let project = inspect_project(&directory.0)?;

        assert_eq!(
            project.root_module.jvm_targets,
            [JvmTarget::new(JvmLanguageKind::Java, "21")]
        );
        assert!(project.modules[0].jvm_targets.is_empty());
        Ok(())
    }

    #[test]
    fn declared_local_parent_is_not_treated_as_resolved_effective_model()
    -> Result<(), Box<dyn Error>> {
        let directory = TestDirectory::new()?;
        directory.write(
            POM_FILE,
            &pom_with_body(
                "root",
                "<groupId>example</groupId><version>1</version><properties><maven.compiler.release>21</maven.compiler.release></properties><modules><module>child</module></modules>",
            ),
        )?;
        directory.write(
            "child/pom.xml",
            "<project><modelVersion>4.0.0</modelVersion><parent><groupId>example</groupId><artifactId>root</artifactId><version>1</version><relativePath>..</relativePath></parent><artifactId>child</artifactId></project>",
        )?;

        let project = inspect_project(&directory.0)?;

        assert!(project.modules[0].jvm_targets.is_empty());
        Ok(())
    }

    #[test]
    fn source_only_and_java_version_do_not_imply_a_target() -> Result<(), Box<dyn Error>> {
        let body = format!(
            "<properties><java.version>21</java.version></properties>{}",
            compiler_plugin("<source>17</source>")
        );

        assert_eq!(discover_single_target(&body)?, None);
        Ok(())
    }

    #[test]
    fn unresolved_or_non_numeric_target_remains_unknown() -> Result<(), Box<dyn Error>> {
        let body = format!(
            "<properties><java.version>21</java.version><maven.compiler.target>17</maven.compiler.target></properties>{}",
            compiler_plugin("<release>${java.version}</release><target>11</target>")
        );

        assert_eq!(discover_single_target(&body)?, None);
        assert_eq!(
            discover_single_target(
                "<properties><maven.compiler.target>not-a-version</maven.compiler.target></properties>"
            )?,
            None
        );
        Ok(())
    }

    #[test]
    fn ignores_compiler_configuration_from_other_plugins() -> Result<(), Box<dyn Error>> {
        let target = discover_single_target(
            "<build><plugins><plugin><artifactId>other-plugin</artifactId><configuration><release>21</release><target>17</target></configuration></plugin></plugins></build>",
        )?;

        assert_eq!(target, None);
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
