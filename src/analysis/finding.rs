#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingCategory {
    Project,
    Dependency,
    Classpath,
    Compatibility,
    Runtime,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub code: String,
    pub severity: FindingSeverity,
    pub category: FindingCategory,
    pub title: String,
    pub description: String,
}
