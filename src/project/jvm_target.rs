use super::JvmLanguageKind;

/// A bytecode/runtime target declared for one JVM language in a project module.
///
/// This is collected project evidence, not the JDK running JMend and not a
/// compatibility conclusion. A module may eventually contain several entries
/// when different JVM language compilers declare independent targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JvmTarget {
    pub language: JvmLanguageKind,
    pub version: String,
}

impl JvmTarget {
    pub fn new(language: JvmLanguageKind, version: impl Into<String>) -> Self {
        Self {
            language,
            version: version.into(),
        }
    }
}
