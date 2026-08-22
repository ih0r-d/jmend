use crate::{analysis::finding::FindingSeverity, commands::check::CheckResult, runtime::JdkStatus};
use anstream::AutoStream;
use anstyle::{AnsiColor, Style};
use std::io::{self, Write};

const TITLE: Style = AnsiColor::BrightCyan.on_default().bold();
const SUCCESS: Style = AnsiColor::Green.on_default().bold();
const WARNING: Style = AnsiColor::Yellow.on_default().bold();
const ERROR: Style = AnsiColor::Red.on_default().bold();
const MUTED: Style = AnsiColor::BrightBlack.on_default();
const LABEL: Style = Style::new().bold();
const LABEL_WIDTH: usize = 16;
const CHECK_LABEL_WIDTH: usize = LABEL_WIDTH - 2;
const PRIMARY_WIDTH: usize = 14;

const BANNER: &str = include_str!("banner.txt");

pub fn render_check(result: &CheckResult) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = AutoStream::auto(stdout.lock());

    render(&mut output, result)
}

pub fn render_root(help: &str) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = AutoStream::auto(stdout.lock());

    render_root_to(&mut output, help)
}

fn render(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    render_compact_header(output, result)?;
    render_fingerprint(output, result)?;
    render_analyzers(output, result)?;
    render_counts(output, result)?;

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
    let (status, status_style) = diagnostic_status(result);
    writeln!(
        output,
        "{TITLE}JMend{TITLE:#} {MUTED}v{}{MUTED:#} {MUTED}·{MUTED:#} {} {} {MUTED}·{MUTED:#} {status_style}{status}{status_style:#}",
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

fn diagnostic_status(result: &CheckResult) -> (&'static str, Style) {
    let has_errors = result.findings.iter().any(|finding| {
        matches!(
            finding.severity,
            FindingSeverity::Error | FindingSeverity::Critical
        )
    });

    let has_warnings = result
        .findings
        .iter()
        .any(|finding| finding.severity == FindingSeverity::Warning);

    if has_errors {
        ("ISSUES FOUND", ERROR)
    } else if has_warnings {
        ("WARNINGS", WARNING)
    } else {
        ("HEALTHY", SUCCESS)
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

fn render_analyzers(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    let project = &result.project_context.project;

    let languages = aggregate_languages(project)
        .into_iter()
        .map(|language| (language.kind.to_string(), language.version.as_deref()))
        .collect::<Vec<_>>();
    render_analyzer_values(output, "Languages", &languages)?;

    let frameworks = aggregate_frameworks(project)
        .into_iter()
        .map(|framework| (framework.kind.to_string(), framework.version.as_deref()))
        .collect::<Vec<_>>();
    render_analyzer_values(output, "Frameworks", &frameworks)?;

    for label in ["Dependencies", "Classpath", "GraalVM"] {
        render_planned(output, label)?;
    }

    writeln!(output)
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

fn render_analyzer_values(
    output: &mut dyn Write,
    label: &str,
    values: &[(String, Option<&str>)],
) -> io::Result<()> {
    if values.is_empty() {
        return render_planned(output, label);
    }

    for (index, (primary, secondary)) in values.iter().enumerate() {
        if index == 0 {
            write!(output, "{SUCCESS}✓{SUCCESS:#} {label:<CHECK_LABEL_WIDTH$}")?;
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

fn render_planned(output: &mut dyn Write, label: &str) -> io::Result<()> {
    writeln!(
        output,
        "{MUTED}◇{MUTED:#} {label:<CHECK_LABEL_WIDTH$}{MUTED}Planned{MUTED:#}"
    )
}

fn render_counts(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    let errors = result
        .findings
        .iter()
        .filter(|finding| {
            matches!(
                finding.severity,
                FindingSeverity::Error | FindingSeverity::Critical
            )
        })
        .count();

    let warnings = result
        .findings
        .iter()
        .filter(|finding| finding.severity == FindingSeverity::Warning)
        .count();

    let error_style = if errors == 0 { MUTED } else { ERROR };
    let warning_style = if warnings == 0 { MUTED } else { WARNING };
    writeln!(
        output,
        "{error_style}{errors} errors{error_style:#} {MUTED}·{MUTED:#} {warning_style}{warnings} warnings{warning_style:#}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        cli::Cli,
        project::{
            BuildTool, BuildWrapper, JvmFramework, JvmFrameworkKind, JvmLanguage, JvmLanguageKind,
            Project, ProjectContext, ProjectModule,
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
        assert!(output.contains("JVM and GraalVM diagnostics"));
        assert!(output.contains("Usage: jmend [COMMAND]"));
        assert!(output.contains("check"));
        Ok(())
    }

    #[test]
    fn renders_status_project_jdk_and_skipped_checks_without_duplicate_summary()
    -> Result<(), Box<dyn Error>> {
        let output = render_plain(&check_result(BuildTool::Maven, None))?;

        assert!(!output.contains('\u{1b}'));
        assert!(output.contains(&format!(
            "JMend v{} · test-os future-arch · HEALTHY",
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
        for area in [
            "Languages",
            "Frameworks",
            "Dependencies",
            "Classpath",
            "GraalVM",
        ] {
            assert!(output.contains(&format!("◇ {area:<14}Planned")));
        }
        let overview_rows = output
            .lines()
            .filter(|line| line.starts_with('◇'))
            .collect::<Vec<_>>();
        assert_eq!(overview_rows.len(), 5);
        assert!(!overview_rows.iter().any(|line| line.contains("Native")));
        for value in ["Java [", "Kotlin", "Spring Boot", "Hibernate"] {
            assert!(!output.contains(value));
        }
        assert!(output.contains("0 errors · 0 warnings"));
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
        assert!(output.contains("◇ GraalVM       Planned"));
        assert!(!output.contains("GraalVM CE"));
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
            },
        ];

        let output = render_plain(&result)?;

        assert!(output.contains("Project         Gradle        [2 modules]"));
        assert!(output.contains("✓ Languages     Java          [21]"));
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

        assert!(output.contains("✓ Frameworks    Spring Boot   [4.1.0]"));
        assert!(output.contains("                Hibernate     [7.1.0]"));
        Ok(())
    }
}
