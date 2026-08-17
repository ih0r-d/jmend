pub mod analyzer;
pub mod finding;

use crate::project::ProjectContext;

#[derive(Debug)]
pub struct AnalysisContext<'a> {
    pub project: &'a ProjectContext,
}

impl<'a> AnalysisContext<'a> {
    pub fn new(project: &'a ProjectContext) -> Self {
        Self { project }
    }
}
