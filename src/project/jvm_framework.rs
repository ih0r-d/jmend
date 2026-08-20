use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JvmFrameworkKind {
    SpringBoot,
    Quarkus,
    Micronaut,
    Helidon,
    Hibernate,
}

impl fmt::Display for JvmFrameworkKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SpringBoot => formatter.write_str("Spring Boot"),
            Self::Quarkus => formatter.write_str("Quarkus"),
            Self::Micronaut => formatter.write_str("Micronaut"),
            Self::Helidon => formatter.write_str("Helidon"),
            Self::Hibernate => formatter.write_str("Hibernate"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmFramework {
    pub kind: JvmFrameworkKind,
    pub version: Option<String>,
}

impl JvmFramework {
    pub fn new(kind: JvmFrameworkKind, version: Option<String>) -> Self {
        Self { kind, version }
    }
}
