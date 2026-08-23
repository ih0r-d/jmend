use crate::artifact::{ArtifactEvidence, ArtifactKind, ClassKind, StructuralAnalysis};
use crate::{analysis::finding::FindingSeverity, commands::check::CheckResult, runtime::JdkStatus};
use anstream::AutoStream;
use anstyle::{AnsiColor, Style};
use std::io::{self, Write};

const TITLE: Style = AnsiColor::BrightCyan.on_default().bold();
const WARNING: Style = AnsiColor::Yellow.on_default().bold();
const ERROR: Style = AnsiColor::Red.on_default().bold();
const MUTED: Style = AnsiColor::BrightBlack.on_default();
const LABEL: Style = Style::new().bold();
const LABEL_WIDTH: usize = 16;
const PRIMARY_WIDTH: usize = 14;

const BANNER: &str = include_str!("banner.txt");

pub fn render_check(result: &CheckResult) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = AutoStream::auto(stdout.lock());

    render(&mut output, result)
}

pub fn render_artifact(artifact: &ArtifactEvidence) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = AutoStream::auto(stdout.lock());
    render_artifact_to(&mut output, artifact)
}

fn render_artifact_to(output: &mut dyn Write, artifact: &ArtifactEvidence) -> io::Result<()> {
    writeln!(
        output,
        "{TITLE}JMend{TITLE:#} {MUTED}v{}{MUTED:#}",
        env!("CARGO_PKG_VERSION")
    )?;
    writeln!(output)?;
    render_row(
        output,
        "Artifact",
        &artifact.path.display().to_string(),
        None,
    )?;
    render_row(
        output,
        "Type",
        match artifact.kind {
            ArtifactKind::Class => "CLASS",
            ArtifactKind::Jar => "JAR",
        },
        None,
    )?;
    let versions = artifact
        .classes
        .iter()
        .map(|class| {
            class.version.java.map_or_else(
                || format!("major {}", class.version.major),
                |java| format!("Java {java}"),
            )
        })
        .fold(Vec::new(), |mut values, value| {
            if !values.contains(&value) {
                values.push(value);
            }
            values
        });
    if !versions.is_empty() {
        render_row(output, "Bytecode", &versions.join(", "), None)?;
    }
    let module = artifact.classes.iter().any(|class| {
        class
            .identity
            .as_ref()
            .is_some_and(|identity| identity.kind == ClassKind::Module)
    });
    render_row(output, "Module", if module { "yes" } else { "no" }, None)?;
    if artifact.kind == ArtifactKind::Jar {
        render_row(
            output,
            "Multi-Release",
            if artifact.multi_release { "yes" } else { "no" },
            None,
        )?;
    }
    if artifact
        .classes
        .iter()
        .any(|class| matches!(class.analysis, StructuralAnalysis::Unsupported { .. }))
    {
        render_row(output, "Analysis", "partially unsupported", None)?;
    }
    writeln!(output)?;
    let usages = artifact
        .classes
        .iter()
        .flat_map(|class| &class.api_usages)
        .collect::<Vec<_>>();
    let has_native_method = artifact
        .classes
        .iter()
        .any(|class| class.methods.iter().any(|method| method.native));
    let has_method_handles = artifact
        .classes
        .iter()
        .any(|class| !class.method_handles.is_empty());
    if !usages.is_empty() || has_native_method || has_method_handles {
        writeln!(output, "{LABEL}Static evidence{LABEL:#}")?;
        for (label, kind) in [
            ("Reflection", crate::artifact::ApiUsageKind::Reflection),
            (
                "Dynamic loading",
                crate::artifact::ApiUsageKind::DynamicClassLoading,
            ),
            (
                "Native/JNI",
                crate::artifact::ApiUsageKind::NativeLibraryLoading,
            ),
            ("Resources", crate::artifact::ApiUsageKind::ResourceAccess),
            (
                "ServiceLoader",
                crate::artifact::ApiUsageKind::ServiceLoading,
            ),
            (
                "Method handles",
                crate::artifact::ApiUsageKind::MethodHandles,
            ),
        ] {
            let detected = usages.iter().any(|usage| usage.kind == kind)
                || (kind == crate::artifact::ApiUsageKind::NativeLibraryLoading
                    && has_native_method)
                || (kind == crate::artifact::ApiUsageKind::MethodHandles && has_method_handles);
            if detected {
                render_row(output, label, "detected", None)?;
            }
        }
        writeln!(output)?;
    }
    writeln!(output, "{MUTED}0 findings{MUTED:#}")
}

pub fn render_root(help: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = AutoStream::auto(stdout.lock());

    render_root_to(&mut output, help)
}

fn render(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    render_compact_header(output, result)?;
    render_fingerprint(output, result)?;
    render_context_evidence(output, result)?;
    render_findings_count(output, result)?;

    Ok(())
}

fn render_root_to(output: &mut dyn Write, help: &str) -> io::Result<()> {
    render_banner(output)?;
    writeln!(
        output,
        "{TITLE}JMend{TITLE:#} {MUTED}v{}{MUTED:#}",
        env!("CARGO_PKG_VERSION")
    )?;
    writeln!(output)?;
    write!(output, "{help}")
}

fn render_banner(output: &mut dyn Write) -> io::Result<()> {
    write!(output, "{TITLE}{BANNER}{TITLE:#}")?;
    writeln!(output)
}

fn render_compact_header(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    writeln!(
        output,
        "{TITLE}JMend{TITLE:#} {MUTED}v{}{MUTED:#} {MUTED}·{MUTED:#} {} {}",
        env!("CARGO_PKG_VERSION"),
        display_os(&result.host.os),
        display_architecture(&result.host.architecture)
    )?;
    writeln!(output)
}

fn display_os(os: &str) -> &str {
    match os {
        "macos" => "macOS",
        "linux" => "Linux",
        "windows" => "Windows",
        other => other,
    }
}

fn display_architecture(architecture: &str) -> &str {
    match architecture {
        "aarch64" => "arm64",
        other => other,
    }
}

fn render_fingerprint(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    let project = &result.project_context.project;
    let module_count = match project.modules.len() {
        0 => None,
        1 => Some("1 module".to_string()),
        count => Some(format!("{count} modules")),
    };
    let build_tool = project.build_tool.to_string();
    render_row(output, "Project", &build_tool, module_count.as_deref())?;

    match &result.project_context.jdk {
        JdkStatus::Detected(jdk) => render_row(output, "JDK", &jdk.version, jdk.vendor.as_deref())?,
        JdkStatus::NotFound => render_row(output, "JDK", "Not found", None)?,
        JdkStatus::Unavailable(reason) => {
            render_row(output, "JDK", &format!("Unavailable: {reason}"), None)?
        }
    }

    let targets = aggregate_targets(project);
    if !targets.is_empty() {
        render_row(output, "Target", &targets.join(", "), None)?;
    }

    if let Some(runtime) = &result.project_context.build_tool_runtime {
        render_row(
            output,
            "Build Tool",
            &format!("{} {}", runtime.tool, runtime.version),
            Some(&runtime.source.to_string()),
        )?;
        if let Some(jdk) = &runtime.jdk {
            render_row(output, "Build JDK", &jdk.version, jdk.vendor.as_deref())?;
        }
    }

    let artifacts = project
        .build_units()
        .flat_map(|unit| &unit.artifacts)
        .count();
    if artifacts > 0 {
        render_row(output, "Artifacts", &artifacts.to_string(), None)?;
    }

    writeln!(output)
}

fn render_row(
    output: &mut dyn Write,
    label: &str,
    primary: &str,
    secondary: Option<&str>,
) -> io::Result<()> {
    match secondary {
        Some(secondary) => writeln!(
            output,
            "{LABEL}{label:<LABEL_WIDTH$}{LABEL:#}{primary:<PRIMARY_WIDTH$}{MUTED}[{secondary}]{MUTED:#}"
        ),
        None => writeln!(output, "{LABEL}{label:<LABEL_WIDTH$}{LABEL:#}{primary}"),
    }
}

fn aggregate_targets(project: &crate::project::Project) -> Vec<String> {
    let mut targets = Vec::new();
    for target in project.build_units().flat_map(|module| &module.jvm_targets) {
        let value = format!("{} {}", target.language, target.version);
        if !targets.contains(&value) {
            targets.push(value);
        }
    }
    targets
}

fn render_context_evidence(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    let project = &result.project_context.project;

    let languages = aggregate_languages(project)
        .into_iter()
        .map(|language| (language.kind.to_string(), language.version.as_deref()))
        .collect::<Vec<_>>();
    render_evidence_values(output, "Languages", &languages)?;

    let frameworks = aggregate_frameworks(project)
        .into_iter()
        .map(|framework| (framework.kind.to_string(), framework.version.as_deref()))
        .collect::<Vec<_>>();
    render_evidence_values(output, "Frameworks", &frameworks)
}

fn aggregate_languages(project: &crate::project::Project) -> Vec<&crate::project::JvmLanguage> {
    let mut languages = Vec::new();
    for language in project.build_units().flat_map(|module| &module.languages) {
        if !languages.contains(&language) {
            languages.push(language);
        }
    }
    languages
}

fn aggregate_frameworks(project: &crate::project::Project) -> Vec<&crate::project::JvmFramework> {
    let mut frameworks = Vec::new();
    for framework in project.build_units().flat_map(|module| &module.frameworks) {
        if !frameworks.contains(&framework) {
            frameworks.push(framework);
        }
    }
    frameworks
}

fn render_evidence_values(
    output: &mut dyn Write,
    label: &str,
    values: &[(String, Option<&str>)],
) -> io::Result<()> {
    if values.is_empty() {
        return Ok(());
    }

    for (index, (primary, secondary)) in values.iter().enumerate() {
        if index == 0 {
            write!(output, "{LABEL}{label:<LABEL_WIDTH$}{LABEL:#}")?;
        } else {
            write!(output, "{:<LABEL_WIDTH$}", "")?;
        }
        render_value(output, primary, *secondary)?;
    }
    Ok(())
}

fn render_value(output: &mut dyn Write, primary: &str, secondary: Option<&str>) -> io::Result<()> {
    match secondary {
        Some(secondary) => writeln!(
            output,
            "{primary:<PRIMARY_WIDTH$}{MUTED}[{secondary}]{MUTED:#}"
        ),
        None => writeln!(output, "{primary}"),
    }
}

fn render_findings_count(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    let count = result.findings.len();
    let style = if result.findings.iter().any(|finding| {
        matches!(
            finding.severity,
            FindingSeverity::Error | FindingSeverity::Critical
        )
    }) {
        ERROR
    } else if result
        .findings
        .iter()
        .any(|finding| finding.severity == FindingSeverity::Warning)
    {
        WARNING
    } else {
        MUTED
    };
    let noun = if count == 1 { "finding" } else { "findings" };
    writeln!(output, "{style}{count} {noun}{style:#}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cli::Cli,
        project::{
            BuildTool, BuildToolRuntime, BuildToolSource, BuildWrapper, JvmFramework,
            JvmFrameworkKind, JvmLanguage, JvmLanguageKind, JvmTarget, Project, ProjectContext,
            ProjectModule,
        },
        runtime::{HostPlatform, JdkInfo},
    };
    use anstream::{ColorChoice, StripStream};
    use clap::CommandFactory;
    use std::{error::Error, path::PathBuf};

    fn check_result(build_tool: BuildTool, wrapper: Option<BuildWrapper>) -> CheckResult {
        let jdk = JdkStatus::Detected(JdkInfo {
            version: "25.0.2".to_string(),
            vendor: Some("Oracle".to_string()),
            runtime: None,
        });
        CheckResult {
            project_context: ProjectContext::new(
                Project {
                    root: PathBuf::from("project"),
                    build_tool,
                    wrapper,
                    root_module: ProjectModule::new(PathBuf::from("project")),
                    modules: Vec::new(),
                },
                jdk,
            ),
            host: HostPlatform::new("test-os", "future-arch"),
            findings: Vec::new(),
        }
    }

    fn render_plain(result: &CheckResult) -> Result<String, Box<dyn Error>> {
        let mut output = StripStream::new(Vec::new());
        render(&mut output, result)?;
        Ok(String::from_utf8(output.into_inner())?)
    }

    fn render_ansi_then_strip(result: &CheckResult) -> Result<(String, String), Box<dyn Error>> {
        let mut ansi = AutoStream::new(Vec::new(), ColorChoice::AlwaysAnsi);
        render(&mut ansi, result)?;
        let ansi = ansi.into_inner();

        let mut stripped = StripStream::new(Vec::new());
        stripped.write_all(&ansi)?;
        Ok((
            String::from_utf8(ansi)?,
            String::from_utf8(stripped.into_inner())?,
        ))
    }

    #[test]
    fn root_entry_renders_banner_branding_and_generated_help() -> Result<(), Box<dyn Error>> {
        let help = Cli::command().render_help().to_string();
        let mut output = StripStream::new(Vec::new());
        render_root_to(&mut output, &help)?;
        let output = String::from_utf8(output.into_inner())?;

        assert!(output.contains(BANNER.trim()));
        assert!(output.contains(&format!("JMend v{}", env!("CARGO_PKG_VERSION"))));
        assert!(
            output.contains("JVM ecosystem diagnostics with GraalVM and Native Image awareness")
        );
        assert!(output.contains("Usage: jmend [COMMAND]"));
        assert!(output.contains("check"));
        Ok(())
    }

    #[test]
    fn renders_only_collected_fingerprint_evidence() -> Result<(), Box<dyn Error>> {
        let output = render_plain(&check_result(BuildTool::Maven, None))?;

        assert!(!output.contains('\u{1b}'));
        assert!(output.contains(&format!(
            "JMend v{} · test-os future-arch",
            env!("CARGO_PKG_VERSION")
        )));
        assert!(!output.contains(BANNER.trim()));
        assert!(!output.contains(":: Status ::"));
        assert!(!output.contains(":: JVM Project ::"));
        assert!(!output.contains(":: Java"));
        assert!(output.contains("Project         Maven"));
        assert!(!output.contains("[0 modules]"));
        assert!(!output.contains("Maven project detected"));
        assert!(output.contains("JDK             25.0.2        [Oracle]"));
        for value in [
            "Planned",
            "Coming soon",
            "Not analyzed yet",
            "Languages",
            "Frameworks",
            "Dependencies",
            "Classpath",
            "GraalVM",
            "Native Image",
            "HEALTHY",
        ] {
            assert!(!output.contains(value));
        }
        assert!(output.contains("0 findings"));
        Ok(())
    }

    #[test]
    fn renders_arbitrary_jdk_vendor_without_affecting_graalvm_status() -> Result<(), Box<dyn Error>>
    {
        let mut result = check_result(BuildTool::Maven, None);
        result.project_context.jdk = JdkStatus::Detected(JdkInfo {
            version: "21.0.2".to_string(),
            vendor: Some("Example Custom JDK Vendor".to_string()),
            runtime: Some("Custom GraalVM-compatible Runtime".to_string()),
        });

        let output = render_plain(&result)?;

        assert!(output.contains("JDK             21.0.2        [Example Custom JDK Vendor]"));
        assert!(!output.contains("GraalVM"));
        Ok(())
    }

    #[test]
    fn aligns_project_and_jdk_primary_and_secondary_columns() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Maven, None);
        result.project_context.project.modules = (1..=10)
            .map(|number| ProjectModule::new(PathBuf::from(format!("project/module-{number}"))))
            .collect();
        result.project_context.jdk = JdkStatus::Detected(JdkInfo {
            version: "25".to_string(),
            vendor: Some("GraalVM Community".to_string()),
            runtime: None,
        });

        let output = render_plain(&result)?;
        let fingerprint = output
            .lines()
            .filter(|line| line.starts_with("Project") || line.starts_with("JDK"))
            .collect::<Vec<_>>()
            .join("\n");

        assert_eq!(
            fingerprint,
            "Project         Maven         [10 modules]\nJDK             25            [GraalVM Community]"
        );
        Ok(())
    }

    #[test]
    fn ansi_styles_do_not_change_visible_alignment() -> Result<(), Box<dyn Error>> {
        let result = check_result(BuildTool::Maven, None);
        let plain = render_plain(&result)?;
        let (ansi, stripped) = render_ansi_then_strip(&result)?;

        assert!(ansi.contains('\u{1b}'));
        assert_eq!(stripped, plain);
        assert!(stripped.contains("Project         Maven"));
        assert!(stripped.contains("JDK             25.0.2        [Oracle]"));
        Ok(())
    }

    #[test]
    fn omits_jdk_vendor_metadata_when_unavailable() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Maven, None);
        result.project_context.jdk = JdkStatus::Detected(JdkInfo {
            version: "21.0.2".to_string(),
            vendor: None,
            runtime: None,
        });

        let output = render_plain(&result)?;

        assert!(output.lines().any(|line| line == "JDK             21.0.2"));
        assert!(!output.contains("21.0.2 ["));
        assert!(!output.lines().any(|line| line.ends_with("[]")));
        Ok(())
    }

    #[test]
    fn platform_presentation_preserves_unknown_values() {
        assert_eq!(display_os("future-os"), "future-os");
        assert_eq!(display_architecture("future-arch"), "future-arch");
        assert_eq!(display_os("macos"), "macOS");
        assert_eq!(display_architecture("aarch64"), "arm64");
    }

    #[test]
    fn renders_singular_module_count_as_secondary_metadata() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Maven, None);
        result
            .project_context
            .project
            .modules
            .push(ProjectModule::new(PathBuf::from("project/service")));

        let output = render_plain(&result)?;

        assert!(output.contains("Project         Maven         [1 module]"));
        Ok(())
    }

    #[test]
    fn root_module_is_not_included_in_child_module_count() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Maven, None);
        result
            .project_context
            .project
            .root_module
            .jvm_targets
            .push(crate::project::JvmTarget::new(JvmLanguageKind::Java, "21"));

        let output = render_plain(&result)?;

        assert!(output.contains("Project         Maven"));
        assert!(!output.contains("[1 module]"));
        Ok(())
    }

    #[test]
    fn renders_plural_module_count_as_secondary_metadata() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Maven, None);
        result.project_context.project.modules = (1..=7)
            .map(|number| ProjectModule::new(PathBuf::from(format!("project/module-{number}"))))
            .collect();

        let output = render_plain(&result)?;

        assert!(output.contains("Project         Maven         [7 modules]"));
        Ok(())
    }

    #[test]
    fn renders_gradle_without_exposing_wrapper_metadata() -> Result<(), Box<dyn Error>> {
        let result = check_result(
            BuildTool::Gradle,
            Some(BuildWrapper {
                executable: PathBuf::from("project/gradlew"),
                metadata: Some(PathBuf::from(
                    "project/gradle/wrapper/gradle-wrapper.properties",
                )),
            }),
        );

        let output = render_plain(&result)?;

        assert!(output.contains("Project         Gradle"));
        assert!(!output.contains("wrapper"));
        assert!(!output.contains("gradlew"));
        Ok(())
    }

    #[test]
    fn renders_multiple_languages_with_optional_versions() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Gradle, None);
        result.project_context.project.modules = vec![
            ProjectModule {
                name: Some("api".to_string()),
                path: PathBuf::from("project/api"),
                build_file: Some(PathBuf::from("project/api/build.gradle.kts")),
                languages: vec![JvmLanguage::new(
                    JvmLanguageKind::Java,
                    Some("21".to_string()),
                )],
                frameworks: Vec::new(),
                jvm_targets: Vec::new(),
                artifacts: Vec::new(),
            },
            ProjectModule {
                name: Some("analytics".to_string()),
                path: PathBuf::from("project/analytics"),
                build_file: Some(PathBuf::from("project/analytics/build.gradle.kts")),
                languages: vec![
                    JvmLanguage::new(JvmLanguageKind::Java, Some("21".to_string())),
                    JvmLanguage::new(JvmLanguageKind::Kotlin, Some("2.2.0".to_string())),
                ],
                frameworks: Vec::new(),
                jvm_targets: Vec::new(),
                artifacts: Vec::new(),
            },
        ];

        let output = render_plain(&result)?;

        assert!(output.contains("Project         Gradle        [2 modules]"));
        assert!(output.contains("Languages       Java          [21]"));
        assert!(output.contains("                Kotlin        [2.2.0]"));
        Ok(())
    }

    #[test]
    fn renders_multiple_frameworks_with_optional_versions() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Maven, None);
        let mut module = ProjectModule::new(PathBuf::from("project/service"));
        module.frameworks = vec![
            JvmFramework::new(JvmFrameworkKind::SpringBoot, Some("4.1.0".to_string())),
            JvmFramework::new(JvmFrameworkKind::Hibernate, Some("7.1.0".to_string())),
        ];
        result.project_context.project.modules.push(module);

        let output = render_plain(&result)?;

        assert!(output.contains("Frameworks      Spring Boot   [4.1.0]"));
        assert!(output.contains("                Hibernate     [7.1.0]"));
        Ok(())
    }

    #[test]
    fn renders_build_runtime_jdk_and_target_as_independent_evidence() -> Result<(), Box<dyn Error>>
    {
        let mut result = check_result(BuildTool::Maven, None);
        result.project_context.jdk = JdkStatus::Detected(JdkInfo {
            version: "25".to_string(),
            vendor: Some("JMend Runtime Vendor".to_string()),
            runtime: None,
        });
        result
            .project_context
            .project
            .root_module
            .jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, "17"));
        result.project_context.build_tool_runtime = Some(BuildToolRuntime {
            tool: BuildTool::Maven,
            version: "3.9.11".to_string(),
            executable: PathBuf::from("project/mvnw"),
            source: BuildToolSource::Wrapper,
            jdk: Some(JdkInfo {
                version: "21".to_string(),
                vendor: Some("Build Vendor".to_string()),
                runtime: None,
            }),
        });

        let output = render_plain(&result)?;

        assert!(output.contains("JDK             25            [JMend Runtime Vendor]"));
        assert!(output.contains("Target          Java 17"));
        assert!(output.contains("Build Tool      Maven 3.9.11  [wrapper]"));
        assert!(output.contains("Build JDK       21            [Build Vendor]"));
        assert!(output.contains("0 findings"));
        Ok(())
    }

    #[test]
    fn mixed_module_targets_are_rendered_as_a_factual_aggregate() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Maven, None);
        result
            .project_context
            .project
            .root_module
            .jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, "21"));
        let mut child = ProjectModule::new(PathBuf::from("project/legacy"));
        child
            .jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, "17"));
        result.project_context.project.modules.push(child);

        let output = render_plain(&result)?;

        assert!(output.contains("Target          Java 21, Java 17"));
        assert!(!output.lines().any(|line| line == "Target          Java 21"));
        Ok(())
    }

    #[test]
    fn missing_optional_build_runtime_does_not_render_misleading_rows() -> Result<(), Box<dyn Error>>
    {
        let result = check_result(BuildTool::Gradle, None);
        let output = render_plain(&result)?;

        assert!(!output.contains("Build Tool"));
        assert!(!output.contains("Build JDK"));
        Ok(())
    }
}
