use super::{JvmFramework, JvmLanguage};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectModule {
    pub name: Option<String>,
    pub path: PathBuf,
    pub build_file: Option<PathBuf>,
    pub languages: Vec<JvmLanguage>,
    pub frameworks: Vec<JvmFramework>,
    pub target_runtime: Option<String>,
}

impl ProjectModule {
    pub fn new(path: PathBuf) -> Self {
        Self {
            name: None,
            path,
            build_file: None,
            languages: Vec::new(),
            frameworks: Vec::new(),
            target_runtime: None,
        }
    }
}
