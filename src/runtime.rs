pub mod jdk;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostPlatform {
    pub os: String,
    pub architecture: String,
}

impl HostPlatform {
    pub fn detect() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            architecture: std::env::consts::ARCH.to_string(),
        }
    }

    pub fn new(os: impl Into<String>, architecture: impl Into<String>) -> Self {
        Self {
            os: os.into(),
            architecture: architecture.into(),
        }
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_platform_uses_rust_platform_constants() {
        let platform = HostPlatform::detect();

        assert_eq!(platform.os, std::env::consts::OS);
        assert_eq!(platform.architecture, std::env::consts::ARCH);
    }
}
