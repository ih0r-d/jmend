use super::inspect;
use crate::project::{BuildTool, Project, ProjectModule};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub(crate) fn collect(project: &mut Project) {
    let tool = project.build_tool;
    collect_for(tool, &mut project.root_module);
    for module in &mut project.modules {
        collect_for(tool, module);
    }
}

fn collect_for(tool: BuildTool, module: &mut ProjectModule) {
    let candidates = match tool {
        BuildTool::Maven => discover_maven(&module.path),
        BuildTool::Gradle => discover_gradle(&module.path),
        BuildTool::Sbt | BuildTool::Mill | BuildTool::Ant => Vec::new(),
    };
    module.artifacts = candidates
        .into_iter()
        .filter_map(|path| inspect(&path).ok())
        .collect();
}

fn discover_maven(unit: &Path) -> Vec<PathBuf> {
    let mut paths = files_with_extension(&unit.join("target"), "jar");
    if paths.is_empty() {
        collect_classes(&unit.join("target").join("classes"), &mut paths);
    }
    paths.sort();
    paths
}
fn discover_gradle(unit: &Path) -> Vec<PathBuf> {
    let mut paths = files_with_extension(&unit.join("build").join("libs"), "jar");
    if paths.is_empty() {
        collect_classes(&unit.join("build").join("classes"), &mut paths);
    }
    paths.sort();
    paths
}

fn collect_classes(directory: &Path, paths: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_classes(&path, paths);
        } else if path
            .extension()
            .and_then(|v| v.to_str())
            .is_some_and(|v| v.eq_ignore_ascii_case("class"))
        {
            paths.push(path);
        }
    }
}

fn files_with_extension(directory: &Path, extension: &str) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(directory) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|value| value.to_str())
                    .is_some_and(|value| value.eq_ignore_ascii_case(extension))
        })
        .collect::<Vec<_>>();
    paths.sort();
    paths
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::{Project, ProjectModule};
    use std::{
        fs,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };
    use zip::{ZipWriter, write::SimpleFileOptions};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    #[test]
    fn discovery_is_scoped_to_conventional_output_directory() {
        let root = std::env::temp_dir().join(format!(
            "jmend-artifact-discovery-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("target")).unwrap();
        fs::create_dir_all(root.join("other")).unwrap();
        fs::write(root.join("target/app.jar"), []).unwrap();
        fs::write(root.join("target/tests.jar"), []).unwrap();
        fs::write(root.join("other/unrelated.jar"), []).unwrap();
        let paths = discover_maven(&root);
        assert_eq!(paths.len(), 2);
        assert!(
            paths
                .iter()
                .all(|p| p.parent() == Some(root.join("target").as_path()))
        );
        fs::remove_dir_all(root).unwrap();
    }
    fn write_jar(path: &Path) {
        let file = fs::File::create(path).unwrap();
        let mut zip = ZipWriter::new(file);
        zip.start_file(
            "fixtures/StaticEvidence.class",
            SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(include_bytes!(
            "../../tests/fixtures/java17/fixtures/StaticEvidence.class"
        ))
        .unwrap();
        zip.finish().unwrap();
    }
    #[test]
    fn each_maven_build_unit_owns_zero_one_or_many_artifacts() {
        let root = std::env::temp_dir().join(format!(
            "jmend-artifact-owners-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        for unit in ["", "module-a", "module-b", "module-c"] {
            fs::create_dir_all(root.join(unit).join("target")).unwrap();
        }
        write_jar(&root.join("target/root.jar"));
        write_jar(&root.join("module-a/target/a.jar"));
        write_jar(&root.join("module-b/target/b.jar"));
        write_jar(&root.join("module-b/target/b-tests.jar"));
        fs::create_dir_all(root.join("unrelated")).unwrap();
        write_jar(&root.join("unrelated/random.jar"));
        let mut project = Project {
            root: root.clone(),
            build_tool: BuildTool::Maven,
            wrapper: None,
            root_module: ProjectModule::new(root.clone()),
            modules: ["module-a", "module-b", "module-c"]
                .into_iter()
                .map(|name| ProjectModule::new(root.join(name)))
                .collect(),
        };
        collect(&mut project);
        assert_eq!(project.root_module.artifacts.len(), 1);
        assert_eq!(project.modules[0].artifacts.len(), 1);
        assert_eq!(project.modules[1].artifacts.len(), 2);
        assert!(project.modules[2].artifacts.is_empty());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn gradle_discovery_prefers_packaged_artifacts_to_duplicate_loose_classes() {
        let root = std::env::temp_dir().join(format!(
            "jmend-gradle-artifacts-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir_all(root.join("build/libs")).unwrap();
        fs::create_dir_all(root.join("build/classes/java/main")).unwrap();
        write_jar(&root.join("build/libs/app.jar"));
        fs::write(
            root.join("build/classes/java/main/StaticEvidence.class"),
            include_bytes!("../../tests/fixtures/java21/fixtures/StaticEvidence.class"),
        )
        .unwrap();
        let mut module = ProjectModule::new(root.clone());
        collect_for(BuildTool::Gradle, &mut module);
        assert_eq!(module.artifacts.len(), 1);
        assert_eq!(module.artifacts[0].kind, crate::artifact::ArtifactKind::Jar);
        fs::remove_file(root.join("build/libs/app.jar")).unwrap();
        collect_for(BuildTool::Gradle, &mut module);
        assert_eq!(module.artifacts.len(), 1);
        assert_eq!(
            module.artifacts[0].kind,
            crate::artifact::ArtifactKind::Class
        );
        fs::remove_dir_all(root).unwrap();
    }
}
