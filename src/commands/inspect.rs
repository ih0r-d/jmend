use crate::{artifact::ArtifactEvidence, error::AppError};
use std::path::Path;

pub fn run(path: &Path) -> Result<ArtifactEvidence, AppError> {
    crate::artifact::inspect(path).map_err(AppError::Artifact)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inspects_class_without_project_detection() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/java25/fixtures/StaticEvidence.class");
        let evidence = run(&path).unwrap();
        assert_eq!(evidence.classes[0].version.java, Some(25));
    }
    #[test]
    fn unsupported_input_fails_cleanly() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(run(&path).is_err());
    }
}
