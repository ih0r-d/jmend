//! Compiled JVM artifact evidence. This layer collects facts; it does not create findings.

pub(crate) mod catalog;
pub(crate) mod classfile;
pub(crate) mod discovery;
pub(crate) mod jar;

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArtifactKind {
    Class,
    Jar,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactEvidence {
    pub path: PathBuf,
    pub kind: ArtifactKind,
    pub manifest: Option<ManifestEvidence>,
    pub multi_release: bool,
    pub classes: Vec<ClassEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ManifestEvidence {
    pub main_class: Option<String>,
    pub automatic_module_name: Option<String>,
    pub multi_release: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassLocation {
    Direct,
    Base,
    Versioned(u16),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StructuralAnalysis {
    Complete,
    Unsupported { reason: String },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BytecodeVersion {
    pub minor: u16,
    pub major: u16,
    pub java: Option<JavaRelease>,
}

/// Java release corresponding to a known JVM class-file major version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavaRelease {
    Java1_0Or1_1,
    Java1_2,
    Java1_3,
    Java1_4,
    Standard(u16),
}

impl JavaRelease {
    #[must_use]
    pub const fn from_class_major(major: u16) -> Option<Self> {
        match major {
            45 => Some(Self::Java1_0Or1_1),
            46 => Some(Self::Java1_2),
            47 => Some(Self::Java1_3),
            48 => Some(Self::Java1_4),
            49..=69 => Some(Self::Standard(major - 44)),
            _ => None,
        }
    }
}

impl std::fmt::Display for JavaRelease {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Java1_0Or1_1 => formatter.write_str("1.0/1.1"),
            Self::Java1_2 => formatter.write_str("1.2"),
            Self::Java1_3 => formatter.write_str("1.3"),
            Self::Java1_4 => formatter.write_str("1.4"),
            Self::Standard(release) => release.fmt(formatter),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassEvidence {
    pub entry: Option<String>,
    pub location: ClassLocation,
    pub version: BytecodeVersion,
    pub analysis: StructuralAnalysis,
    pub identity: Option<ClassIdentity>,
    pub fields: Vec<FieldEvidence>,
    pub methods: Vec<MethodEvidence>,
    pub type_references: Vec<String>,
    pub method_calls: Vec<MethodCallEvidence>,
    pub field_accesses: Vec<FieldAccessEvidence>,
    pub api_usages: Vec<ApiUsageEvidence>,
    pub invokedynamic: Vec<InvokeDynamicEvidence>,
    pub bootstrap_methods: Vec<BootstrapMethodEvidence>,
    pub method_handles: Vec<MemberReference>,
    pub jdk_internal_references: Vec<String>,
    pub module: Option<ModuleEvidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassKind {
    Class,
    Interface,
    Annotation,
    Enum,
    Record,
    Module,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassIdentity {
    pub name: String,
    pub kind: ClassKind,
    pub access_flags: u16,
    pub superclass: Option<String>,
    pub interfaces: Vec<String>,
    pub signature: Option<String>,
    pub inner_classes: Vec<InnerClassEvidence>,
    pub nest_host: Option<String>,
    pub nest_members: Vec<String>,
    pub permitted_subclasses: Vec<String>,
    pub record_components: Vec<MemberSignature>,
    pub annotations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InnerClassEvidence {
    pub inner: String,
    pub outer: Option<String>,
    pub name: Option<String>,
    pub access_flags: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemberSignature {
    pub name: String,
    pub descriptor: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldEvidence {
    pub owner: String,
    pub name: String,
    pub descriptor: String,
    pub access_flags: u16,
    pub signature: Option<String>,
    pub annotations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodEvidence {
    pub owner: String,
    pub name: String,
    pub descriptor: String,
    pub access_flags: u16,
    pub signature: Option<String>,
    pub exceptions: Vec<String>,
    pub annotations: Vec<String>,
    pub native: bool,
    pub synthetic: bool,
    pub bridge: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MemberReference {
    pub owner: String,
    pub name: String,
    pub descriptor: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InvocationKind {
    Virtual,
    Special,
    Static,
    Interface,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MethodCallEvidence {
    pub caller: MemberSignature,
    pub instruction: usize,
    pub target: MemberReference,
    pub kind: InvocationKind,
    /// A constant loaded by the immediately preceding instruction, when present.
    /// This is a local bytecode fact, not a claim about the runtime argument value.
    pub preceding_constant: Option<ConstantEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConstantEvidence {
    String(String),
    Class(String),
    Integer(i32),
    FloatBits(u32),
    Long(i64),
    DoubleBits(u64),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldAccessKind {
    GetStatic,
    PutStatic,
    GetField,
    PutField,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldAccessEvidence {
    pub caller: MemberSignature,
    pub instruction: usize,
    pub target: MemberReference,
    pub kind: FieldAccessKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiUsageKind {
    Reflection,
    DynamicClassLoading,
    NativeLibraryLoading,
    ResourceAccess,
    ServiceLoading,
    MethodHandles,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StaticResolution {
    Static(String),
    Unresolved,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiUsageEvidence {
    pub call: MethodCallEvidence,
    pub kind: ApiUsageKind,
    pub target: StaticResolution,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvokeDynamicEvidence {
    pub caller: MemberSignature,
    pub instruction: usize,
    pub name: String,
    pub descriptor: String,
    pub bootstrap_method: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BootstrapMethodEvidence {
    pub method: MemberReference,
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleEvidence {
    pub name: String,
    pub requires: Vec<String>,
    pub exports: Vec<String>,
    pub opens: Vec<String>,
    pub uses: Vec<String>,
    pub provides: Vec<MemberSignature>,
}

#[derive(Debug)]
pub enum ArtifactError {
    Io(std::io::Error),
    Unsupported(String),
    Malformed(String),
}

impl std::fmt::Display for ArtifactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(f, "cannot read artifact: {error}"),
            Self::Unsupported(reason) => f.write_str(reason),
            Self::Malformed(reason) => write!(f, "malformed JVM artifact: {reason}"),
        }
    }
}

impl std::error::Error for ArtifactError {}

impl From<std::io::Error> for ArtifactError {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}

pub fn inspect(path: &std::path::Path) -> Result<ArtifactEvidence, ArtifactError> {
    let mut artifact = match path.extension().and_then(|extension| extension.to_str()) {
        Some(extension) if extension.eq_ignore_ascii_case("class") => classfile::inspect_file(path),
        Some(extension) if extension.eq_ignore_ascii_case("jar") => jar::inspect(path),
        _ => Err(ArtifactError::Unsupported(format!(
            "unsupported artifact: {}",
            path.display()
        ))),
    }?;
    catalog::enrich(&mut artifact);
    Ok(artifact)
}

#[cfg(test)]
mod tests {
    use super::JavaRelease;

    #[test]
    fn maps_known_class_file_majors_independently_of_project_baseline() {
        for (major, release) in [(52, 8), (55, 11), (61, 17), (65, 21), (69, 25)] {
            assert_eq!(
                JavaRelease::from_class_major(major),
                Some(JavaRelease::Standard(release))
            );
        }
        assert_eq!(JavaRelease::from_class_major(70), None);
    }
}
