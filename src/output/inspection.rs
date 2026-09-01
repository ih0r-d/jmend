use crate::{
    artifact::{
        ApiUsageKind, ArtifactEvidence, ArtifactKind, ClassEvidence, StaticResolution,
        StructuralAnalysis,
    },
    commands::inspect::Inspection,
    project::{Project, ProjectModule},
};
use anstyle::{AnsiColor, Style};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{self, Write},
    path::Path,
};

const TITLE: Style = AnsiColor::BrightCyan.on_default().bold();
const MUTED: Style = AnsiColor::BrightBlack.on_default();
const LABEL: Style = Style::new().bold();
const DETAIL_LIMIT: usize = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Category {
    Reflection,
    DynamicLoading,
    Resources,
    ServiceLoading,
    Native,
    JdkInternals,
    MethodHandles,
    InvokeDynamic,
}

impl Category {
    const ALL: [Self; 8] = [
        Self::Reflection,
        Self::DynamicLoading,
        Self::Resources,
        Self::ServiceLoading,
        Self::Native,
        Self::JdkInternals,
        Self::MethodHandles,
        Self::InvokeDynamic,
    ];

    const fn label(self) -> &'static str {
        match self {
            Self::Reflection => "Reflection",
            Self::DynamicLoading => "Dynamic loading",
            Self::Resources => "Resources",
            Self::ServiceLoading => "Service loading",
            Self::Native => "Native / JNI",
            Self::JdkInternals => "JDK internals",
            Self::MethodHandles => "Method handles",
            Self::InvokeDynamic => "Invokedynamic",
        }
    }

    const fn columns(self) -> [&'static str; 3] {
        match self {
            Self::Reflection | Self::DynamicLoading => ["API", "Location", "Target"],
            Self::Resources => ["API", "Location", "Resource"],
            Self::ServiceLoading => ["API", "Location", "Service"],
            Self::Native => ["Usage", "Location", "Target"],
            Self::JdkInternals => ["Reference", "Used by", ""],
            Self::MethodHandles => ["API", "Location", ""],
            Self::InvokeDynamic => ["Bootstrap", "Location", ""],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct DetailRow {
    sort_key: (String, String, usize),
    cells: [String; 3],
}

struct ArtifactView {
    sections: BTreeMap<Category, Vec<DetailRow>>,
}

impl ArtifactView {
    fn new(artifact: &ArtifactEvidence) -> Self {
        let names = LocationNames::new(artifact);
        let mut sections = BTreeMap::<Category, Vec<DetailRow>>::new();
        for class in &artifact.classes {
            collect_class_rows(class, &names, &mut sections);
        }
        for rows in sections.values_mut() {
            rows.sort();
            rows.dedup();
        }
        Self { sections }
    }

    fn count(&self, category: Category) -> usize {
        self.sections.get(&category).map_or(0, Vec::len)
    }

    fn has_evidence(&self) -> bool {
        self.sections.values().any(|rows| !rows.is_empty())
    }
}

struct LocationNames {
    qualified: BTreeSet<String>,
}

impl LocationNames {
    fn new(artifact: &ArtifactEvidence) -> Self {
        let mut owners = BTreeMap::<String, BTreeSet<String>>::new();
        for name in artifact
            .classes
            .iter()
            .filter_map(|class| class.identity.as_ref().map(|identity| &identity.name))
        {
            owners
                .entry(simple_name(name).to_string())
                .or_default()
                .insert(name.clone());
        }
        let qualified = owners
            .into_values()
            .filter(|names| names.len() > 1)
            .flatten()
            .collect();
        Self { qualified }
    }

    fn class(&self, internal_name: &str) -> String {
        if self.qualified.contains(internal_name) {
            human_name(internal_name)
        } else {
            simple_name(internal_name).to_string()
        }
    }
}

pub(crate) fn render(output: &mut dyn Write, inspection: &Inspection) -> io::Result<()> {
    writeln!(
        output,
        "{TITLE}JMend{TITLE:#} {MUTED}v{}{MUTED:#}",
        env!("CARGO_PKG_VERSION")
    )?;
    writeln!(output)?;
    match inspection {
        Inspection::Artifact(artifact) => render_artifact(output, artifact, None, None),
        Inspection::Project(project) => render_project(output, project),
    }
}

fn render_project(output: &mut dyn Write, project: &Project) -> io::Result<()> {
    let project_name = project
        .root_module
        .name
        .as_deref()
        .or_else(|| project.root.file_name().and_then(|name| name.to_str()))
        .unwrap_or("project");
    writeln!(output, "{LABEL}{project_name}{LABEL:#}")?;
    if project.modules.is_empty() {
        writeln!(output, "{}", project.build_tool)?;
    } else {
        writeln!(
            output,
            "{} · {} modules",
            project.build_tool,
            project.modules.len()
        )?;
    }
    writeln!(output)?;
    render_modules(output, project)?;

    let units = project.build_units().collect::<Vec<_>>();
    let artifact_count = units.iter().map(|unit| unit.artifacts.len()).sum::<usize>();
    if artifact_count == 0 {
        writeln!(output, "No compiled artifacts found.")?;
        return writeln!(
            output,
            "Run the project build before artifact evidence can be inspected."
        );
    }

    writeln!(output, "{LABEL}Evidence by module{LABEL:#}")?;
    for (index, module) in units.iter().enumerate() {
        let label = module_label(project, module, index == 0);
        if module.artifacts.is_empty() {
            writeln!(output, "  {label}  {MUTED}no artifacts{MUTED:#}")?;
            continue;
        }
        let counts = aggregate_module(module);
        let summary = Category::ALL
            .into_iter()
            .filter_map(|category| {
                counts
                    .get(&category)
                    .copied()
                    .filter(|count| *count > 0)
                    .map(|count| format!("{} {count}", category.label()))
            })
            .collect::<Vec<_>>();
        if summary.is_empty() {
            writeln!(output, "  {label}  {MUTED}no categorized evidence{MUTED:#}")?;
        } else {
            writeln!(output, "  {label}")?;
            render_wrapped_summary(output, &summary)?;
        }
    }
    writeln!(output)?;

    for (index, module) in units.iter().enumerate() {
        let label = module_label(project, module, index == 0);
        for artifact in &module.artifacts {
            let path = relative_path(&project.root, &artifact.path);
            render_artifact(output, artifact, Some(&label), Some(&path))?;
        }
    }
    Ok(())
}

fn render_wrapped_summary(output: &mut dyn Write, values: &[String]) -> io::Result<()> {
    const INDENT: &str = "    ";
    const WIDTH: usize = 96;
    let mut line = String::from(INDENT);
    for value in values {
        let separator = if line.len() == INDENT.len() {
            ""
        } else {
            " · "
        };
        if line.len() + separator.len() + value.len() > WIDTH && line.len() > INDENT.len() {
            writeln!(output, "{line}")?;
            line.clear();
            line.push_str(INDENT);
        } else {
            line.push_str(separator);
        }
        line.push_str(value);
    }
    writeln!(output, "{line}")
}

fn render_modules(output: &mut dyn Write, project: &Project) -> io::Result<()> {
    writeln!(output, "{LABEL}Modules{LABEL:#}")?;
    writeln!(
        output,
        "  {:<20} {:<18} {:<10} Bytecode",
        "Module", "Target", "Artifacts"
    )?;
    for (index, module) in project.build_units().enumerate() {
        let label = module_label(project, module, index == 0);
        let targets = module
            .jvm_targets
            .iter()
            .map(|target| format!("{} {}", target.language, target.version))
            .collect::<Vec<_>>();
        let target = if targets.is_empty() {
            "—".into()
        } else {
            targets.join(", ")
        };
        let artifacts = match module.artifacts.len() {
            0 => "—".into(),
            count => count.to_string(),
        };
        let bytecode = module_bytecode(module);
        writeln!(
            output,
            "  {label:<20} {target:<18} {artifacts:<10} {bytecode}"
        )?;
    }
    writeln!(output)
}

fn render_artifact(
    output: &mut dyn Write,
    artifact: &ArtifactEvidence,
    module: Option<&str>,
    display_path: Option<&str>,
) -> io::Result<()> {
    let view = ArtifactView::new(artifact);
    let path = display_path
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| artifact.path.display().to_string());
    match module {
        Some(module) => writeln!(output, "{LABEL}{module} · {path}{LABEL:#}")?,
        None => writeln!(output, "{LABEL}{path}{LABEL:#}")?,
    }
    writeln!(output, "{}", artifact_header(artifact))?;

    if let Some(main_class) = artifact
        .manifest
        .as_ref()
        .and_then(|manifest| manifest.main_class.as_deref())
    {
        writeln!(output)?;
        writeln!(output, "{LABEL}Artifact{LABEL:#}")?;
        writeln!(output, "  {:<18} {}", "Main class", human_name(main_class))?;
    }

    if view.has_evidence() {
        writeln!(output)?;
        writeln!(output, "{LABEL}Evidence{LABEL:#}")?;
        for category in Category::ALL {
            let count = view.count(category);
            if count > 0 {
                writeln!(output, "  {:<20} {count}", category.label())?;
            }
        }
        for category in Category::ALL {
            if let Some(rows) = view.sections.get(&category).filter(|rows| !rows.is_empty()) {
                render_section(output, category, rows)?;
            }
        }
    }
    writeln!(output)
}

fn render_section(
    output: &mut dyn Write,
    category: Category,
    rows: &[DetailRow],
) -> io::Result<()> {
    writeln!(output)?;
    writeln!(output, "{LABEL}{}{LABEL:#}", category.label())?;
    let columns = category.columns();
    if columns[2].is_empty() {
        writeln!(output, "  {:<32} {}", columns[0], columns[1])?;
    } else {
        writeln!(
            output,
            "  {:<28} {:<28} {}",
            columns[0], columns[1], columns[2]
        )?;
    }
    for row in rows.iter().take(DETAIL_LIMIT) {
        if columns[2].is_empty() {
            writeln!(output, "  {:<32} {}", row.cells[0], row.cells[1])?;
        } else {
            writeln!(
                output,
                "  {:<28} {:<28} {}",
                row.cells[0], row.cells[1], row.cells[2]
            )?;
        }
    }
    if rows.len() > DETAIL_LIMIT {
        writeln!(
            output,
            "  {MUTED}... {} more{MUTED:#}",
            rows.len() - DETAIL_LIMIT
        )?;
    }
    Ok(())
}

fn collect_class_rows(
    class: &ClassEvidence,
    names: &LocationNames,
    sections: &mut BTreeMap<Category, Vec<DetailRow>>,
) {
    let owner = class
        .identity
        .as_ref()
        .map(|identity| identity.name.as_str())
        .unwrap_or("unknown");
    for usage in &class.api_usages {
        let category = match usage.kind {
            ApiUsageKind::Reflection => Category::Reflection,
            ApiUsageKind::DynamicClassLoading => Category::DynamicLoading,
            ApiUsageKind::NativeLibraryLoading => Category::Native,
            ApiUsageKind::ResourceAccess => Category::Resources,
            ApiUsageKind::ServiceLoading => Category::ServiceLoading,
            ApiUsageKind::MethodHandles => Category::MethodHandles,
        };
        let target = match &usage.target {
            StaticResolution::Static(target) => resolved_target(category, target),
            StaticResolution::Unresolved => "unresolved".into(),
        };
        push_row(
            sections,
            category,
            owner,
            &usage.call.caller.name,
            usage.call.instruction,
            [
                short_member(&usage.call.target.owner, &usage.call.target.name),
                location(names, owner, &usage.call.caller.name),
                target,
            ],
        );
    }
    for method in class.methods.iter().filter(|method| method.native) {
        push_row(
            sections,
            Category::Native,
            owner,
            &method.name,
            usize::MAX,
            [
                "native method".into(),
                location(names, owner, &method.name),
                "—".into(),
            ],
        );
    }
    for reference in &class.jdk_internal_references {
        push_row(
            sections,
            Category::JdkInternals,
            owner,
            reference,
            0,
            [human_name(reference), names.class(owner), String::new()],
        );
    }
    for dynamic in &class.invokedynamic {
        let bootstrap = class
            .bootstrap_methods
            .get(usize::from(dynamic.bootstrap_method))
            .map(|method| short_member(&method.method.owner, &method.method.name))
            .unwrap_or_else(|| format!("bootstrap #{}", dynamic.bootstrap_method));
        push_row(
            sections,
            Category::InvokeDynamic,
            owner,
            &dynamic.caller.name,
            dynamic.instruction,
            [
                bootstrap,
                location(names, owner, &dynamic.caller.name),
                String::new(),
            ],
        );
    }
}

fn push_row(
    sections: &mut BTreeMap<Category, Vec<DetailRow>>,
    category: Category,
    owner: &str,
    member: &str,
    instruction: usize,
    cells: [String; 3],
) {
    sections.entry(category).or_default().push(DetailRow {
        sort_key: (owner.to_string(), member.to_string(), instruction),
        cells,
    });
}

fn aggregate_module(module: &ProjectModule) -> BTreeMap<Category, usize> {
    let mut counts = BTreeMap::new();
    for artifact in &module.artifacts {
        let view = ArtifactView::new(artifact);
        for category in Category::ALL {
            let count = view.count(category);
            if count > 0 {
                *counts.entry(category).or_insert(0) += count;
            }
        }
    }
    counts
}

fn artifact_header(artifact: &ArtifactEvidence) -> String {
    let mut values = vec![match artifact.kind {
        ArtifactKind::Class => "CLASS".to_string(),
        ArtifactKind::Jar => "JAR".to_string(),
    }];
    values.extend(bytecode_versions(artifact));
    if let Some(module) = artifact
        .classes
        .iter()
        .find_map(|class| class.module.as_ref().map(|module| human_name(&module.name)))
        .or_else(|| {
            artifact
                .manifest
                .as_ref()
                .and_then(|manifest| manifest.automatic_module_name.clone())
        })
    {
        values.push(format!("module {module}"));
    }
    if artifact.multi_release {
        values.push("multi-release".into());
    }
    if artifact
        .classes
        .iter()
        .any(|class| matches!(class.analysis, StructuralAnalysis::Unsupported { .. }))
    {
        values.push("partial analysis".into());
    }
    values.join(" · ")
}

fn bytecode_versions(artifact: &ArtifactEvidence) -> Vec<String> {
    let mut versions = BTreeMap::<u16, String>::new();
    for class in &artifact.classes {
        versions.entry(class.version.major).or_insert_with(|| {
            class.version.java.map_or_else(
                || format!("major {}", class.version.major),
                |java| format!("Java {java}"),
            )
        });
    }
    versions.into_values().collect()
}

fn module_bytecode(module: &ProjectModule) -> String {
    let mut versions = BTreeMap::<u16, String>::new();
    for artifact in &module.artifacts {
        for class in &artifact.classes {
            versions.entry(class.version.major).or_insert_with(|| {
                class.version.java.map_or_else(
                    || format!("major {}", class.version.major),
                    |java| format!("Java {java}"),
                )
            });
        }
    }
    if versions.is_empty() {
        "—".into()
    } else {
        versions.into_values().collect::<Vec<_>>().join(", ")
    }
}

fn module_label(project: &Project, module: &ProjectModule, root: bool) -> String {
    if root {
        return module.name.clone().unwrap_or_else(|| "root".into());
    }
    module
        .name
        .clone()
        .or_else(|| {
            module
                .path
                .strip_prefix(&project.root)
                .ok()
                .map(|path| path.display().to_string())
        })
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "module".into())
}

fn relative_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn location(names: &LocationNames, owner: &str, method: &str) -> String {
    format!("{}.{}", names.class(owner), method)
}

fn short_member(owner: &str, member: &str) -> String {
    format!("{}.{}", simple_name(owner), member)
}

fn simple_name(internal_name: &str) -> &str {
    internal_name
        .rsplit(['/', '.'])
        .next()
        .unwrap_or(internal_name)
}

fn human_name(name: &str) -> String {
    name.replace('/', ".")
}

fn resolved_target(category: Category, target: &str) -> String {
    match category {
        Category::Reflection | Category::DynamicLoading | Category::ServiceLoading => {
            human_name(target)
        }
        Category::Resources | Category::Native => target.to_string(),
        Category::JdkInternals | Category::MethodHandles | Category::InvokeDynamic => {
            target.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        artifact::{ArtifactKind, ManifestEvidence},
        project::{BuildTool, JvmLanguageKind, JvmTarget},
    };
    use anstream::StripStream;
    use std::{error::Error, path::PathBuf};

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(name)
    }

    fn render_plain(inspection: &Inspection) -> Result<String, Box<dyn Error>> {
        let mut output = StripStream::new(Vec::new());
        render(&mut output, inspection)?;
        Ok(String::from_utf8(output.into_inner())?)
    }

    #[test]
    fn standalone_class_renders_counts_details_and_human_names() -> Result<(), Box<dyn Error>> {
        let artifact = crate::artifact::inspect(&fixture(
            "tests/fixtures/java25/fixtures/StaticEvidence.class",
        ))?;
        let output = render_plain(&Inspection::Artifact(artifact))?;

        for expected in [
            "CLASS · Java 25",
            "Reflection           2",
            "Class.forName",
            "StaticEvidence.loadStatic",
            "fixtures.Plugin",
            "StaticEvidence.loadDynamic",
            "unresolved",
            "Resources            1",
            "/fixture.txt",
            "Service loading      1",
            "java.lang.Runnable",
            "Native / JNI         2",
            "sun.misc.Unsafe",
            "MethodHandles.lookup",
            "LambdaMetafactory.metafactory",
        ] {
            assert!(output.contains(expected), "missing {expected:?}\n{output}");
        }
        for absent in [
            "Reflection      detected",
            "Resources       detected",
            "0 findings",
            "Module          no",
            "Multi-Release   no",
            "(Ljava/",
            "sun/misc/Unsafe",
            ".fixture.txt",
        ] {
            assert!(!output.contains(absent), "unexpected {absent:?}\n{output}");
        }
        Ok(())
    }

    #[test]
    fn details_are_deterministic_and_bounded() -> Result<(), Box<dyn Error>> {
        let mut artifact = crate::artifact::inspect(&fixture(
            "tests/fixtures/java25/fixtures/StaticEvidence.class",
        ))?;
        let usage = artifact.classes[0]
            .api_usages
            .iter()
            .find(|usage| usage.kind == ApiUsageKind::Reflection)
            .cloned()
            .expect("reflection fixture evidence");
        artifact.classes[0].api_usages.push(usage.clone());
        artifact.classes[0].api_usages.extend((0..5).map(|index| {
            let mut occurrence = usage.clone();
            occurrence.call.instruction = 100 + index;
            occurrence
        }));

        let first = render_plain(&Inspection::Artifact(artifact.clone()))?;
        let second = render_plain(&Inspection::Artifact(artifact))?;
        assert_eq!(first, second);
        assert!(first.contains("Reflection           7"));
        assert!(first.contains("... 2 more"));
        Ok(())
    }

    #[test]
    fn renders_unresolved_resources_and_both_native_loading_apis() -> Result<(), Box<dyn Error>> {
        let mut artifact = crate::artifact::inspect(&fixture(
            "tests/fixtures/java25/fixtures/StaticEvidence.class",
        ))?;
        let class = &mut artifact.classes[0];
        let mut resource = class
            .api_usages
            .iter()
            .find(|usage| usage.kind == ApiUsageKind::ResourceAccess)
            .cloned()
            .expect("resource fixture evidence");
        resource.call.caller.name = "dynamicResource".into();
        resource.call.instruction += 1;
        resource.target = StaticResolution::Unresolved;
        class.api_usages.push(resource);

        let mut load = class
            .api_usages
            .iter()
            .find(|usage| usage.kind == ApiUsageKind::NativeLibraryLoading)
            .cloned()
            .expect("native fixture evidence");
        load.call.target.name = "load".into();
        load.call.caller.name = "loadAbsolute".into();
        load.call.instruction += 1;
        load.target = StaticResolution::Static("/opt/lib/nativefoo.so".into());
        class.api_usages.push(load);

        let output = render_plain(&Inspection::Artifact(artifact))?;
        assert!(output.contains("Resources            2"));
        assert!(output.contains("StaticEvidence.dynamicResource"));
        assert!(output.contains("unresolved"));
        assert!(output.contains("Native / JNI         3"));
        assert!(output.contains("System.loadLibrary"));
        assert!(output.contains("System.load"));
        assert!(output.contains("/opt/lib/nativefoo.so"));
        Ok(())
    }

    #[test]
    fn ambiguous_short_class_names_keep_package_qualification() -> Result<(), Box<dyn Error>> {
        let mut artifact = crate::artifact::inspect(&fixture(
            "tests/fixtures/java25/fixtures/StaticEvidence.class",
        ))?;
        artifact.classes[0].identity.as_mut().unwrap().name = "alpha/Loader".into();
        let mut second = artifact.classes[0].clone();
        second.identity.as_mut().unwrap().name = "beta/Loader".into();
        artifact.classes.push(second);
        let names = LocationNames::new(&artifact);
        assert_eq!(names.class("alpha/Loader"), "alpha.Loader");
        assert_eq!(names.class("beta/Loader"), "beta.Loader");
        Ok(())
    }

    #[test]
    fn useful_jar_metadata_is_conditional() -> Result<(), Box<dyn Error>> {
        let module = crate::artifact::inspect(&fixture("tests/fixtures/java25/module-info.class"))?;
        let artifact = ArtifactEvidence {
            path: "application.jar".into(),
            kind: ArtifactKind::Jar,
            manifest: Some(ManifestEvidence {
                main_class: Some("com/example/Application".into()),
                automatic_module_name: None,
                multi_release: true,
            }),
            multi_release: true,
            classes: module.classes,
        };
        let output = render_plain(&Inspection::Artifact(artifact))?;
        assert!(output.contains("JAR · Java 25 · module jmend.fixtures · multi-release"));
        assert!(output.contains("Main class         com.example.Application"));
        assert!(!output.contains("Module          no"));
        assert!(!output.contains("Multi-Release   no"));
        assert!(!output.contains("partial analysis"));
        Ok(())
    }

    #[test]
    fn future_major_is_the_only_partial_analysis_case() -> Result<(), Box<dyn Error>> {
        let mut old = crate::artifact::inspect(&fixture(
            "tests/fixtures/java8/fixtures/LegacyDependency.class",
        ))?;
        let old_output = render_plain(&Inspection::Artifact(old.clone()))?;
        assert!(old_output.contains("CLASS · Java 8"));
        assert!(!old_output.contains("partial analysis"));

        old.classes[0].version.major = 70;
        old.classes[0].version.java = None;
        old.classes[0].analysis = StructuralAnalysis::Unsupported {
            reason: "future".into(),
        };
        let future_output = render_plain(&Inspection::Artifact(old))?;
        assert!(future_output.contains("major 70 · partial analysis"));
        Ok(())
    }

    #[test]
    fn project_rendering_preserves_module_and_artifact_hierarchy() -> Result<(), Box<dyn Error>> {
        let mut root = ProjectModule::new(PathBuf::from("project"));
        root.name = Some("demo".into());
        root.jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, "21"));
        let mut root_artifact = crate::artifact::inspect(&fixture(
            "tests/fixtures/java21/fixtures/StaticEvidence.class",
        ))?;
        root_artifact.path = "project/target/classes/StaticEvidence.class".into();
        root.artifacts.push(root_artifact);

        let mut core = ProjectModule::new(PathBuf::from("project/core"));
        core.name = Some("core".into());
        core.jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, "17"));
        for (path, fixture_path) in [
            (
                "project/core/target/core.jar",
                "tests/fixtures/java17/fixtures/StaticEvidence.class",
            ),
            (
                "project/core/target/core-tests.jar",
                "tests/fixtures/java25/fixtures/StaticEvidence.class",
            ),
        ] {
            let mut artifact = crate::artifact::inspect(&fixture(fixture_path))?;
            artifact.path = path.into();
            core.artifacts.push(artifact);
        }
        let mut service = ProjectModule::new(PathBuf::from("project/service"));
        service.name = Some("service".into());
        service
            .jvm_targets
            .push(JvmTarget::new(JvmLanguageKind::Java, "21"));
        let project = Project {
            root: "project".into(),
            build_tool: BuildTool::Maven,
            wrapper: None,
            root_module: root,
            modules: vec![core, service],
        };
        let output = render_plain(&Inspection::Project(project))?;

        for expected in [
            "demo",
            "Maven · 2 modules",
            "Modules",
            "demo                 Java 21",
            "core                 Java 17",
            "service              Java 21",
            "Java 17, Java 25",
            "Evidence by module",
            "core · core/target/core.jar",
            "core · core/target/core-tests.jar",
            "service  no artifacts",
        ] {
            assert!(output.contains(expected), "missing {expected:?}\n{output}");
        }
        Ok(())
    }

    #[test]
    fn project_without_artifacts_succeeds_with_guidance() -> Result<(), Box<dyn Error>> {
        let project = Project {
            root: "project".into(),
            build_tool: BuildTool::Maven,
            wrapper: None,
            root_module: ProjectModule::new("project".into()),
            modules: vec![ProjectModule::new("project/api".into())],
        };
        let output = render_plain(&Inspection::Project(project))?;
        assert!(output.contains("No compiled artifacts found."));
        assert!(output.contains("Run the project build"));
        Ok(())
    }
}
