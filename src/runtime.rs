pub mod jdk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JdkInfo {
    pub version: String,
    pub vendor: Option<String>,
    pub runtime: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JdkStatus {
    Detected(JdkInfo),
    NotFound,
    Unavailable(String),
}

