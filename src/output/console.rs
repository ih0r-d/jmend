use crate::{analysis::finding::FindingSeverity, commands::check::CheckResult, runtime::JdkStatus};
use anstream::AutoStream;
use anstyle::{AnsiColor, Style};
use std::io::{self, Write};

const TITLE: Style = AnsiColor::BrightCyan.on_default().bold();
const SUCCESS: Style = AnsiColor::Green.on_default().bold();
const WARNING: Style = AnsiColor::Yellow.on_default().bold();
const ERROR: Style = AnsiColor::Red.on_default().bold();
const MUTED: Style = AnsiColor::BrightBlack.on_default();

pub fn render_check(result: &CheckResult) -> io::Result<()> {
    let stdout = io::stdout();
    let mut output = AutoStream::auto(stdout.lock());

    render_header(&mut output)?;
    render_summary(&mut output, result)?;
    render_checks(&mut output, result)?;
    render_findings(&mut output, result)?;

    Ok(())
}

fn render_header(output: &mut dyn Write) -> io::Result<()> {
    writeln!(output, "{TITLE}JDoctor{TITLE:#}")?;
    writeln!(output)
}

fn render_summary(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    let project = &result.project_context.project;

    writeln!(
        output,
        "{MUTED}:: JVM Project ::{MUTED:#}  {}",
        project.build_tool
    )?;

    match &result.jdk {
        JdkStatus::Detected(jdk) => {
            writeln!(output, "{MUTED}:: Java        ::{MUTED:#}  {}", jdk.version)?;
        }
        JdkStatus::NotFound => {
            writeln!(output, "{MUTED}:: Java        ::{MUTED:#}  Not found")?;
        }
        JdkStatus::Unavailable(_) => {
            writeln!(output, "{MUTED}:: Java        ::{MUTED:#}  Unavailable")?;
        }
    }

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
            "{MUTED}:: Status      ::{MUTED:#}  {ERROR}ISSUES FOUND{ERROR:#}"
        )?;
    } else if has_warnings {
        writeln!(
            output,
            "{MUTED}:: Status      ::{MUTED:#}  {WARNING}WARNINGS{WARNING:#}"
        )?;
    } else {
        writeln!(
            output,
            "{MUTED}:: Status      ::{MUTED:#}  {SUCCESS}HEALTHY{SUCCESS:#}"
        )?;
    }

    writeln!(output)
}

fn render_checks(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    let project = &result.project_context.project;

    writeln!(
        output,
        "{SUCCESS}✓{SUCCESS:#} {:<14}{} project detected",
        "Project", project.build_tool
    )?;

    match &result.jdk {
        JdkStatus::Detected(jdk) => {
            writeln!(
                output,
                "{SUCCESS}✓{SUCCESS:#} {:<14}Java {}",
                "Runtime", jdk.version
            )?;
        }
        JdkStatus::NotFound => {
            writeln!(
                output,
                "{WARNING}!{WARNING:#} {:<14}JDK not found",
                "Runtime"
            )?;
        }
        JdkStatus::Unavailable(reason) => {
            writeln!(
                output,
                "{WARNING}!{WARNING:#} {:<14}JDK unavailable: {}",
                "Runtime", reason
            )?;
        }
    }

    writeln!(
        output,
        "{MUTED}○{MUTED:#} {:<14}Not analyzed yet",
        "Dependencies"
    )?;
    writeln!(
        output,
        "{MUTED}○{MUTED:#} {:<14}Not analyzed yet",
        "Classpath"
    )?;
    writeln!(output, "{MUTED}○{MUTED:#} {:<14}Not analyzed yet", "Native")?;
    writeln!(
        output,
        "{MUTED}○{MUTED:#} {:<14}Not analyzed yet",
        "GraalVM"
    )?;

    writeln!(output)
}

fn render_findings(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
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
