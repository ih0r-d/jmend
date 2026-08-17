use super::{AnalysisContext, finding::Finding};
use crate::error::AnalysisError;

pub trait Analyzer {
    fn analyze(&self, context: &AnalysisContext<'_>) -> Result<Vec<Finding>, AnalysisError>;
}
