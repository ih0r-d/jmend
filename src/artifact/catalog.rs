//! Declarative recognition of semantic APIs over normalized bytecode facts.
//!
//! The class-file adapter deliberately knows nothing about these rules. Catalogs
//! can grow or be replaced without changing binary parsing or domain models.

use super::{
    ApiUsageEvidence, ApiUsageKind, ArtifactEvidence, ClassEvidence, ConstantEvidence,
    MemberReference, StaticResolution,
};

#[derive(Clone, Copy)]
enum TextMatcher {
    Exact(&'static str),
    Prefix(&'static str),
}

impl TextMatcher {
    fn matches(self, value: &str) -> bool {
        match self {
            Self::Exact(expected) => value == expected,
            Self::Prefix(prefix) => value.starts_with(prefix),
        }
    }
}

#[derive(Clone, Copy)]
struct MethodRule {
    owner: TextMatcher,
    name: TextMatcher,
    descriptor: Option<TextMatcher>,
    kinds: &'static [ApiUsageKind],
}

#[derive(Clone, Copy)]
struct StaticStringRule {
    owner: &'static str,
    name: &'static str,
    descriptor: &'static str,
}

impl StaticStringRule {
    fn matches(self, member: &MemberReference) -> bool {
        member.owner == self.owner
            && member.name == self.name
            && member.descriptor == self.descriptor
    }
}

impl MethodRule {
    fn matches(self, member: &MemberReference) -> bool {
        self.owner.matches(&member.owner)
            && self.name.matches(&member.name)
            && self
                .descriptor
                .is_none_or(|matcher| matcher.matches(&member.descriptor))
    }
}

const REFLECTION: &[ApiUsageKind] = &[ApiUsageKind::Reflection];
const REFLECTION_LOADING: &[ApiUsageKind] =
    &[ApiUsageKind::Reflection, ApiUsageKind::DynamicClassLoading];
const DYNAMIC_LOADING: &[ApiUsageKind] = &[ApiUsageKind::DynamicClassLoading];
const NATIVE_LOADING: &[ApiUsageKind] = &[ApiUsageKind::NativeLibraryLoading];
const RESOURCE_ACCESS: &[ApiUsageKind] = &[ApiUsageKind::ResourceAccess];
const SERVICE_LOADING: &[ApiUsageKind] = &[ApiUsageKind::ServiceLoading];
const METHOD_HANDLES: &[ApiUsageKind] = &[ApiUsageKind::MethodHandles];

macro_rules! method_rule {
    ($owner:literal, $name:literal, $kinds:ident) => {
        MethodRule {
            owner: TextMatcher::Exact($owner),
            name: TextMatcher::Exact($name),
            descriptor: None,
            kinds: $kinds,
        }
    };
}

const METHOD_RULES: &[MethodRule] = &[
    method_rule!("java/lang/Class", "forName", REFLECTION_LOADING),
    method_rule!("java/lang/Class", "getDeclaredMethod", REFLECTION),
    method_rule!("java/lang/Class", "getMethod", REFLECTION),
    method_rule!("java/lang/Class", "getDeclaredField", REFLECTION),
    method_rule!("java/lang/Class", "getField", REFLECTION),
    method_rule!("java/lang/reflect/Method", "invoke", REFLECTION),
    method_rule!("java/lang/reflect/Constructor", "newInstance", REFLECTION),
    method_rule!("java/lang/reflect/Field", "get", REFLECTION),
    method_rule!("java/lang/reflect/Field", "set", REFLECTION),
    method_rule!(
        "java/lang/reflect/AccessibleObject",
        "setAccessible",
        REFLECTION
    ),
    method_rule!("java/lang/ClassLoader", "loadClass", DYNAMIC_LOADING),
    method_rule!("java/lang/System", "load", NATIVE_LOADING),
    method_rule!("java/lang/System", "loadLibrary", NATIVE_LOADING),
    method_rule!("java/lang/Class", "getResource", RESOURCE_ACCESS),
    method_rule!("java/lang/Class", "getResourceAsStream", RESOURCE_ACCESS),
    method_rule!("java/lang/ClassLoader", "getResource", RESOURCE_ACCESS),
    method_rule!(
        "java/lang/ClassLoader",
        "getResourceAsStream",
        RESOURCE_ACCESS
    ),
    method_rule!("java/util/ServiceLoader", "load", SERVICE_LOADING),
    MethodRule {
        owner: TextMatcher::Prefix("java/lang/invoke/MethodHandle"),
        name: TextMatcher::Prefix(""),
        descriptor: None,
        kinds: METHOD_HANDLES,
    },
];

const JDK_INTERNAL_NAMESPACES: &[TextMatcher] = &[
    TextMatcher::Prefix("sun/"),
    TextMatcher::Prefix("com/sun/"),
    TextMatcher::Prefix("jdk/internal/"),
];

// Each descriptor has exactly one String parameter. For verified JVM bytecode,
// an immediately preceding LDC string is therefore the value consumed by the
// invocation, rather than merely a nearby constant.
const STATIC_STRING_RULES: &[StaticStringRule] = &[
    StaticStringRule {
        owner: "java/lang/Class",
        name: "forName",
        descriptor: "(Ljava/lang/String;)Ljava/lang/Class;",
    },
    StaticStringRule {
        owner: "java/lang/ClassLoader",
        name: "loadClass",
        descriptor: "(Ljava/lang/String;)Ljava/lang/Class;",
    },
    StaticStringRule {
        owner: "java/lang/System",
        name: "load",
        descriptor: "(Ljava/lang/String;)V",
    },
    StaticStringRule {
        owner: "java/lang/System",
        name: "loadLibrary",
        descriptor: "(Ljava/lang/String;)V",
    },
    StaticStringRule {
        owner: "java/lang/Class",
        name: "getResource",
        descriptor: "(Ljava/lang/String;)Ljava/net/URL;",
    },
    StaticStringRule {
        owner: "java/lang/Class",
        name: "getResourceAsStream",
        descriptor: "(Ljava/lang/String;)Ljava/io/InputStream;",
    },
    StaticStringRule {
        owner: "java/lang/ClassLoader",
        name: "getResource",
        descriptor: "(Ljava/lang/String;)Ljava/net/URL;",
    },
    StaticStringRule {
        owner: "java/lang/ClassLoader",
        name: "getResourceAsStream",
        descriptor: "(Ljava/lang/String;)Ljava/io/InputStream;",
    },
];

pub(crate) fn enrich(artifact: &mut ArtifactEvidence) {
    for class in &mut artifact.classes {
        enrich_class(class);
    }
}

fn enrich_class(class: &mut ClassEvidence) {
    class.api_usages = class
        .method_calls
        .iter()
        .flat_map(|call| {
            METHOD_RULES
                .iter()
                .filter(move |rule| rule.matches(&call.target))
                .flat_map(move |rule| {
                    rule.kinds.iter().map(move |kind| ApiUsageEvidence {
                        call: call.clone(),
                        kind: *kind,
                        target: static_resolution(call),
                    })
                })
        })
        .collect();
    class.jdk_internal_references = class
        .type_references
        .iter()
        .filter(|reference| {
            JDK_INTERNAL_NAMESPACES
                .iter()
                .any(|matcher| matcher.matches(reference))
        })
        .cloned()
        .collect();
}

fn static_resolution(call: &super::MethodCallEvidence) -> StaticResolution {
    let rule_matches = STATIC_STRING_RULES
        .iter()
        .any(|rule| rule.matches(&call.target));
    match (rule_matches, call.preceding_constant.as_ref()) {
        (true, Some(ConstantEvidence::String(value))) => StaticResolution::Static(value.clone()),
        _ => StaticResolution::Unresolved,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact::{ClassLocation, classfile};

    const JAVA25: &[u8] =
        include_bytes!("../../tests/fixtures/java25/fixtures/StaticEvidence.class");

    #[test]
    fn catalog_classifies_normalized_calls_without_parser_knowledge() {
        let class = classfile::inspect_bytes(JAVA25, None, ClassLocation::Direct).unwrap();
        assert!(class.api_usages.is_empty());
        assert!(class.jdk_internal_references.is_empty());

        let mut artifact = ArtifactEvidence {
            path: "fixture.class".into(),
            kind: crate::artifact::ArtifactKind::Class,
            manifest: None,
            multi_release: false,
            classes: vec![class],
        };
        enrich(&mut artifact);
        let evidence = &artifact.classes[0];
        assert!(evidence.api_usages.iter().any(|usage| {
            matches!(&usage.target, StaticResolution::Static(target) if target == "fixtures.Plugin")
        }));
        assert!(evidence.api_usages.iter().any(|usage| {
            usage.call.caller.name == "loadDynamic" && usage.target == StaticResolution::Unresolved
        }));
        for kind in [
            ApiUsageKind::NativeLibraryLoading,
            ApiUsageKind::ResourceAccess,
            ApiUsageKind::ServiceLoading,
            ApiUsageKind::MethodHandles,
        ] {
            assert!(evidence.api_usages.iter().any(|usage| usage.kind == kind));
        }
        assert!(
            evidence
                .jdk_internal_references
                .iter()
                .any(|name| name == "sun/misc/Unsafe")
        );
    }

    #[test]
    fn nearby_string_is_not_a_static_target_without_an_exact_stack_safe_rule() {
        let call = crate::artifact::MethodCallEvidence {
            caller: crate::artifact::MemberSignature {
                name: "caller".into(),
                descriptor: "()V".into(),
            },
            instruction: 1,
            target: MemberReference {
                owner: "java/lang/invoke/MethodHandles".into(),
                name: "lookup".into(),
                descriptor: "()Ljava/lang/invoke/MethodHandles$Lookup;".into(),
            },
            kind: crate::artifact::InvocationKind::Static,
            preceding_constant: Some(ConstantEvidence::String("unrelated".into())),
        };
        assert_eq!(static_resolution(&call), StaticResolution::Unresolved);
    }
}
