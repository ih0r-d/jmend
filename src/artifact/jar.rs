use super::*;
use std::{fs::File, io::Read, path::Path};

const MAX_ENTRIES: usize = 100_000;
const MAX_ENTRY_BYTES: u64 = 64 * 1024 * 1024;
const MAX_TOTAL_BYTES: u64 = 1024 * 1024 * 1024;

pub fn inspect(path: &Path) -> Result<ArtifactEvidence, ArtifactError> {
    let file = File::open(path)?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|error| ArtifactError::Malformed(error.to_string()))?;
    if archive.len() > MAX_ENTRIES {
        return Err(ArtifactError::Malformed(
            "JAR exceeds entry-count safety limit".into(),
        ));
    }
    let mut manifest = None;
    let mut classes = Vec::new();
    let mut total_size = 0_u64;
    for index in 0..archive.len() {
        let mut entry = archive
            .by_index(index)
            .map_err(|error| ArtifactError::Malformed(error.to_string()))?;
        if entry.size() > MAX_ENTRY_BYTES {
            return Err(ArtifactError::Malformed(format!(
                "JAR entry {} exceeds safety limit",
                entry.name()
            )));
        }
        let name = entry.name().replace('\\', "/");
        if name.eq_ignore_ascii_case("META-INF/MANIFEST.MF") {
            let bytes = read_bounded(&mut entry)?;
            add_total_size(&mut total_size, bytes.len())?;
            manifest = Some(parse_manifest(&bytes));
        } else if name.ends_with(".class") {
            let location = versioned_location(&name).unwrap_or(ClassLocation::Base);
            let bytes = read_bounded(&mut entry)?;
            add_total_size(&mut total_size, bytes.len())?;
            classes.push(super::classfile::inspect_bytes(
                &bytes,
                Some(name),
                location,
            )?);
        }
    }
    let multi_release = manifest
        .as_ref()
        .is_some_and(|manifest| manifest.multi_release);
    Ok(ArtifactEvidence {
        path: path.to_path_buf(),
        kind: ArtifactKind::Jar,
        manifest,
        multi_release,
        classes,
    })
}

fn add_total_size(total: &mut u64, bytes: usize) -> Result<(), ArtifactError> {
    *total = total.saturating_add(bytes as u64);
    if *total > MAX_TOTAL_BYTES {
        return Err(ArtifactError::Malformed(
            "JAR exceeds cumulative uncompressed-size safety limit".into(),
        ));
    }
    Ok(())
}

fn read_bounded(reader: &mut impl Read) -> Result<Vec<u8>, ArtifactError> {
    let mut bytes = Vec::new();
    reader.take(MAX_ENTRY_BYTES + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > MAX_ENTRY_BYTES {
        return Err(ArtifactError::Malformed(
            "JAR entry exceeds safety limit".into(),
        ));
    }
    Ok(bytes)
}

fn versioned_location(name: &str) -> Option<ClassLocation> {
    let suffix = name.strip_prefix("META-INF/versions/")?;
    let release = suffix.split('/').next()?.parse().ok()?;
    Some(ClassLocation::Versioned(release))
}

fn parse_manifest(bytes: &[u8]) -> ManifestEvidence {
    let text = String::from_utf8_lossy(bytes).replace("\r\n", "\n");
    let mut logical = Vec::<String>::new();
    for line in text.lines() {
        if let Some(continuation) = line.strip_prefix(' ') {
            if let Some(previous) = logical.last_mut() {
                previous.push_str(continuation);
            }
        } else {
            logical.push(line.to_string());
        }
    }
    let value = |key: &str| {
        logical.iter().find_map(|line| {
            let (name, value) = line.split_once(':')?;
            name.eq_ignore_ascii_case(key)
                .then(|| value.trim().to_string())
        })
    };
    ManifestEvidence {
        main_class: value("Main-Class"),
        automatic_module_name: value("Automatic-Module-Name"),
        multi_release: value("Multi-Release")
            .is_some_and(|value| value.eq_ignore_ascii_case("true")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        fs,
        io::Write,
        sync::atomic::{AtomicU64, Ordering},
    };
    use zip::{ZipWriter, write::SimpleFileOptions};
    static NEXT: AtomicU64 = AtomicU64::new(0);
    fn jar(entries: &[(&str, &[u8])]) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "jmend-jar-{}-{}.jar",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        let file = fs::File::create(&path).unwrap();
        let mut writer = ZipWriter::new(file);
        for (name, bytes) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
        path
    }
    #[test]
    fn parses_relevant_manifest_fields_and_continuations() {
        let manifest = parse_manifest(b"Manifest-Version: 1.0\r\nMain-Class: app.\r\n Main\r\nAutomatic-Module-Name: app.module\r\nMulti-Release: true\r\n");
        assert_eq!(manifest.main_class.as_deref(), Some("app.Main"));
        assert_eq!(
            manifest.automatic_module_name.as_deref(),
            Some("app.module")
        );
        assert!(manifest.multi_release);
    }
    #[test]
    fn recognizes_versioned_entries() {
        assert_eq!(
            versioned_location("META-INF/versions/25/a/A.class"),
            Some(ClassLocation::Versioned(25))
        );
    }
    #[test]
    fn inspects_regular_fat_and_empty_jars() {
        let class = include_bytes!("../../tests/fixtures/java21/fixtures/StaticEvidence.class");
        let path = jar(&[
            ("BOOT-INF/classes/fixtures/StaticEvidence.class", class),
            ("BOOT-INF/lib/dependency.jar", b"nested"),
        ]);
        let evidence = inspect(&path).unwrap();
        assert_eq!(evidence.classes.len(), 1);
        assert_eq!(evidence.classes[0].version.java, Some(21));
        fs::remove_file(path).unwrap();
        let empty = jar(&[("README.txt", b"nothing")]);
        assert!(inspect(&empty).unwrap().classes.is_empty());
        fs::remove_file(empty).unwrap();
    }
    #[test]
    fn preserves_mixed_and_multi_release_bytecode_structure() {
        let manifest = b"Manifest-Version: 1.0\r\nMulti-Release: true\r\n";
        let path = jar(&[
            ("META-INF/MANIFEST.MF", manifest),
            (
                "fixtures/StaticEvidence.class",
                include_bytes!("../../tests/fixtures/java17/fixtures/StaticEvidence.class"),
            ),
            (
                "META-INF/versions/21/fixtures/StaticEvidence.class",
                include_bytes!("../../tests/fixtures/java21/fixtures/StaticEvidence.class"),
            ),
            (
                "META-INF/versions/25/fixtures/StaticEvidence.class",
                include_bytes!("../../tests/fixtures/java25/fixtures/StaticEvidence.class"),
            ),
        ]);
        let evidence = inspect(&path).unwrap();
        assert!(evidence.multi_release);
        assert_eq!(
            evidence
                .classes
                .iter()
                .map(|c| c.location)
                .collect::<Vec<_>>(),
            vec![
                ClassLocation::Base,
                ClassLocation::Versioned(21),
                ClassLocation::Versioned(25)
            ]
        );
        assert_eq!(
            evidence
                .classes
                .iter()
                .map(|c| c.version.java.unwrap())
                .collect::<Vec<_>>(),
            vec![17, 21, 25]
        );
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn corrupt_jar_and_malformed_class_entry_fail_cleanly() {
        let path = std::env::temp_dir().join(format!("jmend-corrupt-{}.jar", std::process::id()));
        fs::write(&path, b"not zip").unwrap();
        assert!(inspect(&path).is_err());
        fs::remove_file(path).unwrap();
        let path = jar(&[("Broken.class", b"broken")]);
        assert!(inspect(&path).is_err());
        fs::remove_file(path).unwrap();
    }
}
