use crate::commands::check::CheckResult;
use std::io::{self, Write};

pub fn render_check(output: &mut dyn Write, result: &CheckResult) -> io::Result<()> {
    writeln!(output, "JDoctor")?;
    writeln!(output)?;
    writeln!(
        output,
        "Project root: {}",
        result.project_context.project.root.display()
    )?;
    writeln!(
        output,
        "Build tool: {}",
        result.project_context.project.build_tool
    )
}
