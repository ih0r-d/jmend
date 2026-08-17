use crate::project::ProjectContext;
use std::io::{self, Write};

pub fn write_project(output: &mut dyn Write, context: &ProjectContext) -> io::Result<()> {
    writeln!(output, "JDoctor")?;
    writeln!(output)?;
    writeln!(output, "Project root: {}", context.project.root.display())?;
    writeln!(output, "Build tool: {}", context.project.build_tool)
}
