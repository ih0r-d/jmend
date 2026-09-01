use super::*;
use ristretto_classfile::{
    ClassAccessFlags, ClassFile, Constant, ConstantPool, MethodAccessFlags,
    attributes::{Annotation, Attribute, Instruction},
};
use std::{collections::BTreeSet, fs::File, io::Read, panic::AssertUnwindSafe, path::Path};

const MAX_CLASS_BYTES: usize = 64 * 1024 * 1024;

pub(crate) fn inspect_file(path: &Path) -> Result<ArtifactEvidence, ArtifactError> {
    let mut bytes = Vec::new();
    File::open(path)?
        .take((MAX_CLASS_BYTES + 1) as u64)
        .read_to_end(&mut bytes)?;
    if bytes.len() > MAX_CLASS_BYTES {
        return Err(ArtifactError::Malformed(
            "class file exceeds 64 MiB safety limit".into(),
        ));
    }
    let class = inspect_bytes(&bytes, None, ClassLocation::Direct)?;
    Ok(ArtifactEvidence {
        path: path.to_path_buf(),
        kind: ArtifactKind::Class,
        manifest: None,
        multi_release: false,
        classes: vec![class],
    })
}

pub(crate) fn inspect_bytes(
    bytes: &[u8],
    entry: Option<String>,
    location: ClassLocation,
) -> Result<ClassEvidence, ArtifactError> {
    let version = read_header(bytes)?;
    let mut evidence = empty_evidence(entry, location, version);
    if bytes.len() > MAX_CLASS_BYTES {
        return Err(ArtifactError::Malformed(
            "class file exceeds 64 MiB safety limit".into(),
        ));
    }
    if ristretto_classfile::Version::from(version.major, version.minor).is_err() {
        evidence.analysis = StructuralAnalysis::Unsupported {
            reason: format!(
                "class-file version {}.{} is not supported by the structural parser",
                version.major, version.minor
            ),
        };
        return Ok(evidence);
    }
    let parsed = std::panic::catch_unwind(AssertUnwindSafe(|| ClassFile::from_bytes(bytes)))
        .map_err(|_| ArtifactError::Malformed("class parser aborted".into()))?
        .map_err(|error| ArtifactError::Malformed(error.to_string()))?;
    normalize(&parsed, &mut evidence)?;
    Ok(evidence)
}

fn read_header(bytes: &[u8]) -> Result<BytecodeVersion, ArtifactError> {
    if bytes.len() < 8 {
        return Err(ArtifactError::Malformed("truncated class header".into()));
    }
    if bytes[..4] != [0xca, 0xfe, 0xba, 0xbe] {
        return Err(ArtifactError::Malformed("invalid class magic".into()));
    }
    let minor = u16::from_be_bytes([bytes[4], bytes[5]]);
    let major = u16::from_be_bytes([bytes[6], bytes[7]]);
    Ok(BytecodeVersion {
        minor,
        major,
        java: JavaRelease::from_class_major(major),
    })
}

fn empty_evidence(
    entry: Option<String>,
    location: ClassLocation,
    version: BytecodeVersion,
) -> ClassEvidence {
    ClassEvidence {
        entry,
        location,
        version,
        analysis: StructuralAnalysis::Complete,
        identity: None,
        fields: vec![],
        methods: vec![],
        type_references: vec![],
        method_calls: vec![],
        field_accesses: vec![],
        api_usages: vec![],
        invokedynamic: vec![],
        bootstrap_methods: vec![],
        method_handles: vec![],
        jdk_internal_references: vec![],
        module: None,
    }
}

fn normalize(class: &ClassFile<'_>, out: &mut ClassEvidence) -> Result<(), ArtifactError> {
    let cp = &class.constant_pool;
    let owner = class.class_name().map_err(malformed)?.to_string();
    let mut types = BTreeSet::new();
    for constant in cp.iter() {
        match constant {
            Constant::Class(index) => {
                types.insert(utf8(cp, *index)?);
            }
            Constant::MethodHandle {
                reference_index, ..
            } => {
                if let Ok(member) = member_ref(cp, *reference_index) {
                    out.method_handles.push(member);
                }
            }
            _ => {}
        }
    }
    let superclass = (class.super_class != 0)
        .then(|| class_name(cp, class.super_class))
        .transpose()?;
    let interfaces = class
        .interfaces
        .iter()
        .map(|index| class_name(cp, *index))
        .collect::<Result<Vec<_>, _>>()?;
    let class_signature = signature(cp, &class.attributes)?;
    let class_annotations = annotations(cp, &class.attributes)?;
    let mut identity = ClassIdentity {
        name: owner.clone(),
        kind: class_kind(class),
        access_flags: class.access_flags.bits(),
        superclass,
        interfaces,
        signature: class_signature,
        inner_classes: vec![],
        nest_host: None,
        nest_members: vec![],
        permitted_subclasses: vec![],
        record_components: vec![],
        annotations: class_annotations,
    };
    normalize_class_attributes(cp, &class.attributes, &mut identity, out)?;
    out.identity = Some(identity);

    for field in &class.fields {
        let descriptor = utf8(cp, field.descriptor_index)?;
        add_descriptor_types(&descriptor, &mut types);
        out.fields.push(FieldEvidence {
            owner: owner.clone(),
            name: utf8(cp, field.name_index)?,
            descriptor,
            access_flags: field.access_flags.bits(),
            signature: signature(cp, &field.attributes)?,
            annotations: annotations(cp, &field.attributes)?,
        });
    }
    for method in &class.methods {
        let name = utf8(cp, method.name_index)?;
        let descriptor = utf8(cp, method.descriptor_index)?;
        add_descriptor_types(&descriptor, &mut types);
        let caller = MemberSignature {
            name: name.clone(),
            descriptor: descriptor.clone(),
        };
        let exceptions = method
            .attributes
            .iter()
            .find_map(|attribute| {
                if let Attribute::Exceptions {
                    exception_indexes, ..
                } = attribute
                {
                    Some(exception_indexes)
                } else {
                    None
                }
            })
            .map(|indexes| indexes.iter().map(|index| class_name(cp, *index)).collect())
            .transpose()?
            .unwrap_or_default();
        out.methods.push(MethodEvidence {
            owner: owner.clone(),
            name,
            descriptor,
            access_flags: method.access_flags.bits(),
            signature: signature(cp, &method.attributes)?,
            exceptions,
            annotations: annotations(cp, &method.attributes)?,
            native: method.access_flags.contains(MethodAccessFlags::NATIVE),
            synthetic: method.access_flags.contains(MethodAccessFlags::SYNTHETIC),
            bridge: method.access_flags.contains(MethodAccessFlags::BRIDGE),
        });
        for attribute in &method.attributes {
            if let Attribute::Code { code, .. } = attribute {
                normalize_code(cp, &caller, code, out)?;
            }
        }
    }
    out.type_references = types.into_iter().collect();
    out.method_handles.sort();
    out.method_handles.dedup();
    Ok(())
}

fn class_kind(class: &ClassFile<'_>) -> ClassKind {
    if class.access_flags.contains(ClassAccessFlags::MODULE) {
        ClassKind::Module
    } else if class
        .attributes
        .iter()
        .any(|a| matches!(a, Attribute::Record { .. }))
    {
        ClassKind::Record
    } else if class.access_flags.contains(ClassAccessFlags::ANNOTATION) {
        ClassKind::Annotation
    } else if class.access_flags.contains(ClassAccessFlags::ENUM) {
        ClassKind::Enum
    } else if class.access_flags.contains(ClassAccessFlags::INTERFACE) {
        ClassKind::Interface
    } else {
        ClassKind::Class
    }
}

fn normalize_code(
    cp: &ConstantPool<'_>,
    caller: &MemberSignature,
    code: &[Instruction],
    out: &mut ClassEvidence,
) -> Result<(), ArtifactError> {
    let mut preceding_constant = None;
    for (offset, instruction) in code.iter().enumerate() {
        let current_constant = match instruction {
            Instruction::Ldc(index) => constant_evidence(cp, u16::from(*index)),
            Instruction::Ldc_w(index) | Instruction::Ldc2_w(index) => constant_evidence(cp, *index),
            _ => None,
        };
        let invocation = match instruction {
            Instruction::Invokevirtual(i) => Some((*i, InvocationKind::Virtual)),
            Instruction::Invokespecial(i) => Some((*i, InvocationKind::Special)),
            Instruction::Invokestatic(i) => Some((*i, InvocationKind::Static)),
            Instruction::Invokeinterface(i, _) => Some((*i, InvocationKind::Interface)),
            _ => None,
        };
        if let Some((index, kind)) = invocation {
            let call = MethodCallEvidence {
                caller: caller.clone(),
                instruction: offset,
                target: member_ref(cp, index)?,
                kind,
                preceding_constant: preceding_constant.take(),
            };
            out.method_calls.push(call);
        } else if let Some((index, kind)) = field_instruction(instruction) {
            out.field_accesses.push(FieldAccessEvidence {
                caller: caller.clone(),
                instruction: offset,
                target: member_ref(cp, index)?,
                kind,
            });
        } else if let Instruction::Invokedynamic(index) = instruction
            && let Constant::InvokeDynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } = cp.try_get(*index).map_err(malformed)?
        {
            let (name, descriptor) = name_and_type(cp, *name_and_type_index)?;
            out.invokedynamic.push(InvokeDynamicEvidence {
                caller: caller.clone(),
                instruction: offset,
                name,
                descriptor,
                bootstrap_method: *bootstrap_method_attr_index,
            });
        }
        preceding_constant = current_constant;
    }
    Ok(())
}

fn field_instruction(instruction: &Instruction) -> Option<(u16, FieldAccessKind)> {
    match instruction {
        Instruction::Getstatic(i) => Some((*i, FieldAccessKind::GetStatic)),
        Instruction::Putstatic(i) => Some((*i, FieldAccessKind::PutStatic)),
        Instruction::Getfield(i) => Some((*i, FieldAccessKind::GetField)),
        Instruction::Putfield(i) => Some((*i, FieldAccessKind::PutField)),
        _ => None,
    }
}

fn normalize_class_attributes(
    cp: &ConstantPool<'_>,
    attributes: &[Attribute],
    identity: &mut ClassIdentity,
    out: &mut ClassEvidence,
) -> Result<(), ArtifactError> {
    for attribute in attributes {
        match attribute {
            Attribute::InnerClasses { classes, .. } => {
                for item in classes {
                    identity.inner_classes.push(InnerClassEvidence {
                        inner: class_name(cp, item.class_info_index)?,
                        outer: optional_class(cp, item.outer_class_info_index)?,
                        name: optional_utf8(cp, item.name_index)?,
                        access_flags: item.access_flags.bits(),
                    });
                }
            }
            Attribute::NestHost {
                host_class_index, ..
            } => identity.nest_host = Some(class_name(cp, *host_class_index)?),
            Attribute::NestMembers { class_indexes, .. } => {
                identity.nest_members = class_indexes
                    .iter()
                    .map(|i| class_name(cp, *i))
                    .collect::<Result<_, _>>()?
            }
            Attribute::PermittedSubclasses { class_indexes, .. } => {
                identity.permitted_subclasses = class_indexes
                    .iter()
                    .map(|i| class_name(cp, *i))
                    .collect::<Result<_, _>>()?
            }
            Attribute::Record { records, .. } => {
                identity.record_components = records
                    .iter()
                    .map(|r| {
                        Ok(MemberSignature {
                            name: utf8(cp, r.name_index)?,
                            descriptor: utf8(cp, r.descriptor_index)?,
                        })
                    })
                    .collect::<Result<_, ArtifactError>>()?
            }
            Attribute::BootstrapMethods { methods, .. } => {
                for method in methods {
                    if let Ok(reference) = method_handle(cp, method.bootstrap_method_ref) {
                        out.bootstrap_methods.push(BootstrapMethodEvidence {
                            method: reference,
                            arguments: method
                                .arguments
                                .iter()
                                .filter_map(|i| cp.get(*i).map(ToString::to_string))
                                .collect(),
                        });
                    }
                }
            }
            Attribute::Module {
                module_name_index,
                requires,
                exports,
                opens,
                uses,
                provides,
                ..
            } => {
                out.module = Some(ModuleEvidence {
                    name: cp
                        .try_get_module(*module_name_index)
                        .map_err(malformed)?
                        .to_string(),
                    requires: requires
                        .iter()
                        .map(|r| {
                            cp.try_get_module(r.index)
                                .map(ToString::to_string)
                                .map_err(malformed)
                        })
                        .collect::<Result<_, _>>()?,
                    exports: exports
                        .iter()
                        .filter_map(|e| cp.try_get_package(e.index).ok().map(ToString::to_string))
                        .collect(),
                    opens: opens
                        .iter()
                        .filter_map(|e| cp.try_get_package(e.index).ok().map(ToString::to_string))
                        .collect(),
                    uses: uses
                        .iter()
                        .map(|i| class_name(cp, *i))
                        .collect::<Result<_, _>>()?,
                    provides: provides
                        .iter()
                        .filter_map(|p| {
                            Some(MemberSignature {
                                name: class_name(cp, p.index).ok()?,
                                descriptor: p
                                    .with_index
                                    .iter()
                                    .filter_map(|i| class_name(cp, *i).ok())
                                    .collect::<Vec<_>>()
                                    .join(","),
                            })
                        })
                        .collect(),
                });
            }
            _ => {}
        }
    }
    Ok(())
}

fn signature(cp: &ConstantPool<'_>, attrs: &[Attribute]) -> Result<Option<String>, ArtifactError> {
    attrs
        .iter()
        .find_map(|a| {
            if let Attribute::Signature {
                signature_index, ..
            } = a
            {
                Some(*signature_index)
            } else {
                None
            }
        })
        .map(|i| utf8(cp, i))
        .transpose()
}
fn annotations(cp: &ConstantPool<'_>, attrs: &[Attribute]) -> Result<Vec<String>, ArtifactError> {
    let mut result = vec![];
    for attr in attrs {
        match attr {
            Attribute::RuntimeVisibleAnnotations { annotations, .. }
            | Attribute::RuntimeInvisibleAnnotations { annotations, .. } => {
                for a in annotations {
                    result.push(annotation(cp, a)?);
                }
            }
            _ => {}
        }
    }
    Ok(result)
}
fn annotation(cp: &ConstantPool<'_>, a: &Annotation) -> Result<String, ArtifactError> {
    utf8(cp, a.type_index)
}
fn utf8(cp: &ConstantPool<'_>, i: u16) -> Result<String, ArtifactError> {
    cp.try_get_utf8(i)
        .map(ToString::to_string)
        .map_err(malformed)
}
fn optional_utf8(cp: &ConstantPool<'_>, i: u16) -> Result<Option<String>, ArtifactError> {
    if i == 0 {
        Ok(None)
    } else {
        utf8(cp, i).map(Some)
    }
}
fn class_name(cp: &ConstantPool<'_>, i: u16) -> Result<String, ArtifactError> {
    cp.try_get_class(i)
        .map(ToString::to_string)
        .map_err(malformed)
}
fn optional_class(cp: &ConstantPool<'_>, i: u16) -> Result<Option<String>, ArtifactError> {
    if i == 0 {
        Ok(None)
    } else {
        class_name(cp, i).map(Some)
    }
}
fn name_and_type(cp: &ConstantPool<'_>, i: u16) -> Result<(String, String), ArtifactError> {
    let (n, d) = cp.try_get_name_and_type(i).map_err(malformed)?;
    Ok((utf8(cp, *n)?, utf8(cp, *d)?))
}
fn member_ref(cp: &ConstantPool<'_>, i: u16) -> Result<MemberReference, ArtifactError> {
    let (c, n) = match cp.try_get(i).map_err(malformed)? {
        Constant::FieldRef {
            class_index,
            name_and_type_index,
        }
        | Constant::MethodRef {
            class_index,
            name_and_type_index,
        }
        | Constant::InterfaceMethodRef {
            class_index,
            name_and_type_index,
        } => (*class_index, *name_and_type_index),
        _ => {
            return Err(ArtifactError::Malformed(format!(
                "constant #{i} is not a member reference"
            )));
        }
    };
    let (name, descriptor) = name_and_type(cp, n)?;
    Ok(MemberReference {
        owner: class_name(cp, c)?,
        name,
        descriptor,
    })
}
fn method_handle(cp: &ConstantPool<'_>, i: u16) -> Result<MemberReference, ArtifactError> {
    let (_, r) = cp.try_get_method_handle(i).map_err(malformed)?;
    member_ref(cp, *r)
}
fn malformed(error: impl std::fmt::Display) -> ArtifactError {
    ArtifactError::Malformed(error.to_string())
}
fn constant_evidence(cp: &ConstantPool<'_>, index: u16) -> Option<ConstantEvidence> {
    match cp.get(index)? {
        Constant::String(value) => utf8(cp, *value).ok().map(ConstantEvidence::String),
        Constant::Class(value) => utf8(cp, *value).ok().map(ConstantEvidence::Class),
        Constant::Integer(value) => Some(ConstantEvidence::Integer(*value)),
        Constant::Float(value) => Some(ConstantEvidence::FloatBits(value.to_bits())),
        Constant::Long(value) => Some(ConstantEvidence::Long(*value)),
        Constant::Double(value) => Some(ConstantEvidence::DoubleBits(value.to_bits())),
        _ => None,
    }
}
fn add_descriptor_types(descriptor: &str, types: &mut BTreeSet<String>) {
    let bytes = descriptor.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'L' {
            let start = index + 1;
            let mut end = start;
            while end < bytes.len() && !matches!(bytes[end], b';' | b'<') {
                end += 1;
            }
            if end > start {
                types.insert(descriptor[start..end].to_string());
            }
            index = end;
        }
        index += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const JAVA17: &[u8] =
        include_bytes!("../../tests/fixtures/java17/fixtures/StaticEvidence.class");
    const JAVA21: &[u8] =
        include_bytes!("../../tests/fixtures/java21/fixtures/StaticEvidence.class");
    const JAVA25: &[u8] =
        include_bytes!("../../tests/fixtures/java25/fixtures/StaticEvidence.class");

    #[test]
    fn parses_real_java_17_21_and_25_classes_structurally() {
        for (bytes, java, major) in [(JAVA17, 17, 61), (JAVA21, 21, 65), (JAVA25, 25, 69)] {
            let evidence = inspect_bytes(bytes, None, ClassLocation::Direct).unwrap();
            assert_eq!(
                evidence.version,
                BytecodeVersion {
                    minor: 0,
                    major,
                    java: Some(JavaRelease::Standard(java))
                }
            );
            let identity = evidence.identity.unwrap();
            assert_eq!(identity.name, "fixtures/StaticEvidence");
            assert_eq!(identity.superclass.as_deref(), Some("java/lang/Object"));
            assert!(
                evidence
                    .fields
                    .iter()
                    .any(|field| field.name == "value" && !field.annotations.is_empty())
            );
            assert!(
                evidence
                    .methods
                    .iter()
                    .any(|method| method.name == "evidence" && !method.annotations.is_empty())
            );
            assert!(!evidence.method_calls.is_empty());
            assert!(!evidence.field_accesses.is_empty());
            assert!(!evidence.invokedynamic.is_empty());
            assert!(!evidence.bootstrap_methods.is_empty());
        }
    }

    #[test]
    fn supports_every_java_release_from_17_through_25() {
        for java in 17_u16..=25 {
            let mut bytes = JAVA25.to_vec();
            bytes[6..8].copy_from_slice(&(java + 44).to_be_bytes());
            let evidence = inspect_bytes(&bytes, None, ClassLocation::Direct).unwrap();
            assert_eq!(evidence.version.java, Some(JavaRelease::Standard(java)));
            assert!(matches!(evidence.analysis, StructuralAnalysis::Complete));
            assert!(evidence.identity.is_some());
            assert!(!evidence.method_calls.is_empty());
        }
    }

    #[test]
    fn structurally_parses_older_dependency_bytecode_supported_by_parser() {
        let evidence = inspect_bytes(
            include_bytes!("../../tests/fixtures/java8/fixtures/LegacyDependency.class"),
            None,
            ClassLocation::Direct,
        )
        .unwrap();
        assert_eq!(evidence.version.major, 52);
        assert_eq!(evidence.version.java, Some(JavaRelease::Standard(8)));
        assert_eq!(evidence.analysis, StructuralAnalysis::Complete);
        assert_eq!(
            evidence
                .identity
                .as_ref()
                .map(|identity| identity.name.as_str()),
            Some("fixtures/LegacyDependency")
        );
        assert!(!evidence.methods.is_empty());
    }

    #[test]
    fn preserves_generic_call_site_constants_without_semantic_classification() {
        let evidence = inspect_bytes(JAVA25, None, ClassLocation::Direct).unwrap();
        assert!(
            evidence
                .method_calls
                .iter()
                .any(|call| call.caller.name == "loadStatic"
                    && call.preceding_constant
                        == Some(ConstantEvidence::String("fixtures.Plugin".into())))
        );
        assert!(
            evidence
                .method_calls
                .iter()
                .any(|call| call.caller.name == "loadDynamic" && call.preceding_constant.is_none())
        );
        assert!(
            evidence
                .methods
                .iter()
                .any(|method| method.name == "nativeCall" && method.native)
        );
        assert!(evidence.api_usages.is_empty());
        assert!(evidence.jdk_internal_references.is_empty());
    }

    #[test]
    fn parses_records_sealed_types_annotations_and_modules() {
        let record = inspect_bytes(
            include_bytes!("../../tests/fixtures/java25/fixtures/Circle.class"),
            None,
            ClassLocation::Direct,
        )
        .unwrap();
        let identity = record.identity.unwrap();
        assert_eq!(identity.kind, ClassKind::Record);
        assert!(!identity.record_components.is_empty());
        assert!(
            identity
                .annotations
                .iter()
                .any(|a| a == "Lfixtures/Marker;")
        );
        let sealed = inspect_bytes(
            include_bytes!("../../tests/fixtures/java25/fixtures/Shape.class"),
            None,
            ClassLocation::Direct,
        )
        .unwrap();
        assert_eq!(
            sealed.identity.unwrap().permitted_subclasses,
            vec!["fixtures/Circle"]
        );
        let module = inspect_bytes(
            include_bytes!("../../tests/fixtures/java25/module-info.class"),
            None,
            ClassLocation::Direct,
        )
        .unwrap();
        assert_eq!(module.module.unwrap().name, "jmend.fixtures");
    }

    #[test]
    fn validates_magic_truncation_and_future_versions_without_panicking() {
        assert!(inspect_bytes(&[], None, ClassLocation::Direct).is_err());
        assert!(inspect_bytes(&[0; 8], None, ClassLocation::Direct).is_err());
        let future = [0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 70];
        let evidence = inspect_bytes(&future, None, ClassLocation::Direct).unwrap();
        assert_eq!(evidence.version.major, 70);
        assert_eq!(evidence.version.java, None);
        assert!(matches!(
            evidence.analysis,
            StructuralAnalysis::Unsupported { .. }
        ));
        assert!(evidence.identity.is_none());
    }

    #[test]
    fn malformed_supported_class_returns_error() {
        let malformed = [0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 69];
        assert!(inspect_bytes(&malformed, None, ClassLocation::Direct).is_err());
    }
}
