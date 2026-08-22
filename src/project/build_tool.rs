use std::{fmt, path::PathBuf};

const MAVEN_BUILD_FILES: &[&str] = &["pom.xml"];
const MAVEN_WRAPPER_EXECUTABLES: &[&str] = &["mvnw", "mvnw.cmd"];
const MAVEN_WINDOWS_WRAPPER_EXECUTABLES: &[&str] = &["mvnw.cmd", "mvnw"];
const MAVEN_WRAPPER_METADATA: &[&str] = &[".mvn", "wrapper", "maven-wrapper.properties"];

const GRADLE_BUILD_FILES: &[&str] = &["build.gradle", "build.gradle.kts"];
const GRADLE_WRAPPER_EXECUTABLES: &[&str] = &["gradlew", "gradlew.bat"];
const GRADLE_WINDOWS_WRAPPER_EXECUTABLES: &[&str] = &["gradlew.bat", "gradlew"];
const GRADLE_WRAPPER_METADATA: &[&str] = &["gradle", "wrapper", "gradle-wrapper.properties"];

const SBT_BUILD_FILES: &[&str] = &["build.sbt"];
const MILL_BUILD_FILES: &[&str] = &["build.mill", "build.sc"];
const ANT_BUILD_FILES: &[&str] = &["build.xml"];
const NO_WRAPPER_EXECUTABLES: &[&str] = &[];
const NO_WRAPPER_METADATA: &[&str] = &[];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildTool {
    Maven,
    Gradle,
    Sbt,
    Mill,
    Ant,
}

impl BuildTool {
    pub const DETECTABLE: [Self; 2] = [Self::Maven, Self::Gradle];

    pub const fn descriptor(self) -> BuildToolDescriptor {
        match self {
            Self::Maven => BuildToolDescriptor {
                build_files: MAVEN_BUILD_FILES,
                wrapper_executables: MAVEN_WRAPPER_EXECUTABLES,
                wrapper_metadata: MAVEN_WRAPPER_METADATA,
                system_command: "mvn",
            },
            Self::Gradle => BuildToolDescriptor {
                build_files: GRADLE_BUILD_FILES,
                wrapper_executables: GRADLE_WRAPPER_EXECUTABLES,
                wrapper_metadata: GRADLE_WRAPPER_METADATA,
                system_command: "gradle",
            },
            Self::Sbt => BuildToolDescriptor {
                build_files: SBT_BUILD_FILES,
                wrapper_executables: NO_WRAPPER_EXECUTABLES,
                wrapper_metadata: NO_WRAPPER_METADATA,
                system_command: "sbt",
            },
            Self::Mill => BuildToolDescriptor {
                build_files: MILL_BUILD_FILES,
                wrapper_executables: NO_WRAPPER_EXECUTABLES,
                wrapper_metadata: NO_WRAPPER_METADATA,
                system_command: "mill",
            },
            Self::Ant => BuildToolDescriptor {
                build_files: ANT_BUILD_FILES,
                wrapper_executables: NO_WRAPPER_EXECUTABLES,
                wrapper_metadata: NO_WRAPPER_METADATA,
                system_command: "ant",
            },
        }
    }

    pub const fn wrapper_executables_for(
        self,
        platform: crate::process::CommandPlatform,
    ) -> &'static [&'static str] {
        match (self, platform) {
            (Self::Maven, crate::process::CommandPlatform::Windows) => {
                MAVEN_WINDOWS_WRAPPER_EXECUTABLES
            }
            (Self::Gradle, crate::process::CommandPlatform::Windows) => {
                GRADLE_WINDOWS_WRAPPER_EXECUTABLES
            }
            _ => self.descriptor().wrapper_executables(),
        }
    }
}

impl fmt::Display for BuildTool {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Maven => formatter.write_str("Maven"),
            Self::Gradle => formatter.write_str("Gradle"),
            Self::Sbt => formatter.write_str("SBT"),
            Self::Mill => formatter.write_str("Mill"),
            Self::Ant => formatter.write_str("Ant"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildToolDescriptor {
    build_files: &'static [&'static str],
    wrapper_executables: &'static [&'static str],
    wrapper_metadata: &'static [&'static str],
    system_command: &'static str,
}

impl BuildToolDescriptor {
    pub const fn build_files(self) -> &'static [&'static str] {
        self.build_files
    }

    pub const fn wrapper_executables(self) -> &'static [&'static str] {
        self.wrapper_executables
    }

    pub const fn wrapper_metadata(self) -> &'static [&'static str] {
        self.wrapper_metadata
    }

    pub const fn system_command(self) -> &'static str {
        self.system_command
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildWrapper {
    pub executable: PathBuf,
    pub metadata: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::CommandPlatform;

    #[test]
    fn wrapper_candidates_prefer_platform_native_scripts() {
        assert_eq!(
            BuildTool::Maven.wrapper_executables_for(CommandPlatform::Unix)[0],
            "mvnw"
        );
        assert_eq!(
            BuildTool::Maven.wrapper_executables_for(CommandPlatform::Windows)[0],
            "mvnw.cmd"
        );
        assert_eq!(
            BuildTool::Gradle.wrapper_executables_for(CommandPlatform::Unix)[0],
            "gradlew"
        );
        assert_eq!(
            BuildTool::Gradle.wrapper_executables_for(CommandPlatform::Windows)[0],
            "gradlew.bat"
        );
    }
}
