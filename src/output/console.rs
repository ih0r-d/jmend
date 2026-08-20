use crate::{analysis::finding::FindingSeverity, commands::check::CheckResult, runtime::JdkStatus};
use anstream::AutoStream;
use anstyle::{AnsiColor, Style};
use std::io::{self, Write};

const TITLE: Style = AnsiColor::BrightCyan.on_default().bold();
const SUCCESS: Style = AnsiColor::Green.on_default().bold();
const WARNING: Style = AnsiColor::Yellow.on_default().bold();
const ERROR: Style = AnsiColor::Red.on_default().bold();
const MUTED: Style = AnsiColor::BrightBlack.on_default();

const BANNER: &str = include_str!("banner.txt");

pub fn render_check(result: &CheckResult) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = AutoStream::auto(stdout.lock());

    render(&mut output, result)
}

fn render(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    render_banner(output)?;
    render_status(output, result)?;
    render_checks(output, result)?;
    render_counts(output, result)?;

    Ok(())
}

fn render_banner(output: &mut dyn Write) -> io::Result<()> {
    write!(output, "{TITLE}{BANNER}{TITLE:#}")?;
    writeln!(output)?;
    writeln!(
        output,
        "{TITLE} JMend {}{TITLE:#}",
        env!("CARGO_PKG_VERSION")
    )?;
    writeln!(output)
}

fn render_status(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
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
        writeln!(
            output,
            "{MUTED}:: Status ::{MUTED:#}  {ERROR}ISSUES FOUND{ERROR:#}"
        )?;
    } else if has_warnings {
        writeln!(
            output,
            "{MUTED}:: Status ::{MUTED:#}  {WARNING}WARNINGS{WARNING:#}"
        )?;
    } else {
        writeln!(
            output,
            "{MUTED}:: Status ::{MUTED:#}  {SUCCESS}HEALTHY{SUCCESS:#}"
        )?;
    }

    writeln!(output)
}

fn render_checks(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    let project = &result.project_context.project;
    let project_description = match project.modules.len() {
        0 => project.build_tool.to_string(),
        1 => format!("{} · 1 module", project.build_tool),
        count => format!("{} · {count} modules", project.build_tool),
    };

    writeln!(
        output,
        "{SUCCESS}✓{SUCCESS:#} {:<14}{project_description}",
        "Project"
    )?;

    match &result.project_context.jdk {
        JdkStatus::Detected(jdk) => {
            let description = match &jdk.vendor {
                Some(vendor) => format!("{} [{vendor}]", jdk.version),
                None => jdk.version.clone(),
            };
            writeln!(output, "{SUCCESS}✓{SUCCESS:#} {:<14}{description}", "JDK")?;
        }
        JdkStatus::NotFound => {
            writeln!(output, "{WARNING}!{WARNING:#} {:<14}Not found", "JDK")?;
        }
        JdkStatus::Unavailable(reason) => {
            writeln!(
                output,
                "{WARNING}!{WARNING:#} {:<14}Unavailable: {}",
                "JDK", reason
            )?;
        }
    }

    let languages = aggregate_languages(project)
        .iter()
        .map(|language| format_versioned(&language.kind, language.version.as_deref()))
        .collect::<Vec<_>>();
    render_metadata_row(output, "Languages", &languages)?;

    let frameworks = aggregate_frameworks(project)
        .iter()
        .map(|framework| format_versioned(&framework.kind, framework.version.as_deref()))
        .collect::<Vec<_>>();
    render_metadata_row(output, "Frameworks", &frameworks)?;

    for label in ["Dependencies", "Classpath", "Native", "GraalVM"] {
        render_skipped(output, label)?;
    }

    writeln!(output)
}

fn aggregate_languages(project: &crate::project::Project) -> Vec<&crate::project::JvmLanguage> {
    let mut languages = Vec::new();
    for language in project.modules.iter().flat_map(|module| &module.languages) {
        if !languages.contains(&language) {
            languages.push(language);
        }
    }
    languages
}

fn aggregate_frameworks(project: &crate::project::Project) -> Vec<&crate::project::JvmFramework> {
    let mut frameworks = Vec::new();
    for framework in project.modules.iter().flat_map(|module| &module.frameworks) {
        if !frameworks.contains(&framework) {
            frameworks.push(framework);
        }
    }
    frameworks
}

fn format_versioned(name: &impl std::fmt::Display, version: Option<&str>) -> String {
    match version {
        Some(version) => format!("{name} [{version}]"),
        None => name.to_string(),
    }
}

fn render_metadata_row(output: &mut dyn Write, label: &str, values: &[String]) -> io::Result<()> {
    if values.is_empty() {
        render_skipped(output, label)
    } else {
        writeln!(
            output,
            "{SUCCESS}✓{SUCCESS:#} {label:<14}{}",
            values.join(" · ")
        )
    }
}

fn render_skipped(output: &mut dyn Write, label: &str) -> io::Result<()> {
    writeln!(output, "{MUTED}○{MUTED:#} {label:<14}Not analyzed yet")
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

    writeln!(
        output,
        "{MUTED}{errors} errors · {warnings} warnings{MUTED:#}"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        project::{
            BuildTool, BuildWrapper, JvmFramework, JvmFrameworkKind, JvmLanguage, JvmLanguageKind,
            Project, ProjectContext, ProjectModule,
        },
        runtime::JdkInfo,
    };
    use anstream::StripStream;
    use std::{error::Error, path::PathBuf};

    fn check_result(build_tool: BuildTool, wrapper: Option<BuildWrapper>) -> CheckResult {
        let jdk = JdkStatus::Detected(JdkInfo {
            version: "21.0.2".to_string(),
            vendor: Some("Oracle".to_string()),
            runtime: None,
        });
        CheckResult {
            project_context: ProjectContext::new(
                Project {
                    root: PathBuf::from("project"),
                    build_tool,
                    wrapper,
                    modules: Vec::new(),
                },
                jdk,
            ),
            findings: Vec::new(),
        }
    }

    fn render_plain(result: &CheckResult) -> Result<String, Box<dyn Error>> {
        let mut output = StripStream::new(Vec::new());
        render(&mut output, result)?;
        Ok(String::from_utf8(output.into_inner())?)
    }

    #[test]
    fn renders_status_project_jdk_and_skipped_checks_without_duplicate_summary()
    -> Result<(), Box<dyn Error>> {
        let output = render_plain(&check_result(BuildTool::Maven, None))?;

        assert!(!output.contains('\u{1b}'));
        assert!(output.contains("JMend"));
        assert!(output.contains(env!("CARGO_PKG_VERSION")));
        assert!(output.contains(":: Status ::  HEALTHY"));
        assert!(!output.contains(":: JVM Project ::"));
        assert!(!output.contains(":: Java"));
        assert!(output.contains("✓ Project       Maven"));
        assert!(!output.contains("Maven project detected"));
        assert!(output.contains("✓ JDK           21.0.2 [Oracle]"));
        for area in [
            "Languages",
            "Frameworks",
            "Dependencies",
            "Classpath",
            "Native",
            "GraalVM",
        ] {
            assert!(output.contains(&format!("○ {area:<14}Not analyzed yet")));
        }
        for value in ["Java [", "Kotlin", "Spring Boot", "Hibernate"] {
            assert!(!output.contains(value));
        }
        assert!(output.contains("0 errors · 0 warnings"));
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

        assert!(output.contains("✓ Project       Gradle"));
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
                target_runtime: None,
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
                target_runtime: None,
            },
        ];

        let output = render_plain(&result)?;

        assert!(output.contains("✓ Project       Gradle · 2 modules"));
        assert!(output.contains("✓ Languages     Java [21] · Kotlin [2.2.0]"));
        Ok(())
    }

    #[test]
    fn renders_multiple_frameworks_with_optional_versions() -> Result<(), Box<dyn Error>> {
        let mut result = check_result(BuildTool::Maven, None);
        let mut module = ProjectModule::new(PathBuf::from("project/service"));
        module.frameworks = vec![
            JvmFramework::new(JvmFrameworkKind::SpringBoot, Some("4.0.7".to_string())),
            JvmFramework::new(JvmFrameworkKind::Hibernate, Some("7.1.0".to_string())),
        ];
        result.project_context.project.modules.push(module);

        let output = render_plain(&result)?;

        assert!(output.contains("✓ Frameworks    Spring Boot [4.0.7] · Hibernate [7.1.0]"));
        Ok(())
    }
}
