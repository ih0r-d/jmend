use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JvmLanguageKind {
    Java,
    Kotlin,
    Scala,
    Groovy,
    Clojure,
}

impl fmt::Display for JvmLanguageKind {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Java => formatter.write_str("Java"),
            Self::Kotlin => formatter.write_str("Kotlin"),
            Self::Scala => formatter.write_str("Scala"),
            Self::Groovy => formatter.write_str("Groovy"),
            Self::Clojure => formatter.write_str("Clojure"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmLanguage {
    pub kind: JvmLanguageKind,
    pub version: Option<String>,
}

impl JvmLanguage {
    pub fn new(kind: JvmLanguageKind, version: Option<String>) -> Self {
        Self { kind, version }
    }
}
