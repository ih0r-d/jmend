use crate::runtime::HostPlatform;
use crate::runtime::JdkStatus;
use crate::runtime::jdk::JdkDetectionError;
use crate::{
    analysis::finding::Finding,
    error::AppError,
    process::{CommandExecutor, ProcessExecutor},
    project::{ProjectContext, build_runtime, detector},
};
use std::path::Path;

#[derive(Debug)]
pub struct CheckResult {
    pub project_context: ProjectContext,
    pub host: HostPlatform,
    pub findings: Vec<Finding>,
}

pub fn run(start: &Path) -> Result<CheckResult, AppError> {
    run_with(start, &ProcessExecutor)
}

pub fn run_with(start: &Path, executor: &dyn CommandExecutor) -> Result<CheckResult, AppError> {
    let project = detector::detect(start)?;
    let build_tool_runtime = build_runtime::collect(&project, executor).ok();
    let jdk = match crate::runtime::jdk::detect_with(executor) {
        Ok(info) => JdkStatus::Detected(info),
        Err(JdkDetectionError::VersionNotFound) => JdkStatus::NotFound,
        Err(error) => JdkStatus::Unavailable(error.to_string()),
    };

    let mut project_context = ProjectContext::new(project, jdk);
    project_context.build_tool_runtime = build_tool_runtime;
    Ok(CheckResult {
        project_context,
        host: HostPlatform::detect(),
        findings: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::process::{CommandOutput, CommandRequest};
    use std::{
        cell::RefCell,
        collections::VecDeque,
        fs, io,
        sync::atomic::{AtomicU64, Ordering},
    };

    static NEXT_DIRECTORY: AtomicU64 = AtomicU64::new(0);

    enum FakeResult {
        Output(CommandOutput),
        Error(io::ErrorKind),
    }

    struct FakeExecutor {
        results: RefCell<VecDeque<FakeResult>>,
        requests: RefCell<Vec<CommandRequest>>,
    }

    impl FakeExecutor {
        fn new(results: Vec<FakeResult>) -> Self {
            Self {
                results: RefCell::new(results.into()),
                requests: RefCell::new(Vec::new()),
            }
        }
    }

    impl CommandExecutor for FakeExecutor {
        fn execute(&self, request: &CommandRequest) -> io::Result<CommandOutput> {
            self.requests.borrow_mut().push(request.clone());
            match self.results.borrow_mut().pop_front().expect("fake result") {
                FakeResult::Output(output) => Ok(output),
                FakeResult::Error(kind) => Err(io::Error::new(kind, "fake error")),
            }
        }
    }

    struct TestDirectory(std::path::PathBuf);

    impl TestDirectory {
        fn maven_project() -> io::Result<Self> {
            let sequence = NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "jmend-check-runtime-{}-{sequence}",
                std::process::id()
            ));
            fs::create_dir(&path)?;
            fs::write(
                path.join("pom.xml"),
                "<project><artifactId>app</artifactId><properties><maven.compiler.release>17</maven.compiler.release></properties></project>",
            )?;
            Ok(Self(path))
        }
    }

    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn success(stdout: &str, stderr: &str) -> FakeResult {
        FakeResult::Output(CommandOutput {
            success: true,
            status: Some(0),
            stdout: stdout.to_string(),
            stderr: stderr.to_string(),
        })
    }

    #[test]
    fn runtime_build_jdk_and_module_target_remain_independent()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = TestDirectory::maven_project()?;
        let executor = FakeExecutor::new(vec![
            success(
                "Apache Maven 3.9.11\nJava version: 21, vendor: Build Vendor",
                "",
            ),
            success(
                "",
                "java.version = 25\njava.vendor = Runtime Vendor\njava.runtime.name = Runtime JDK",
            ),
        ]);

        let result = run_with(&directory.0, &executor)?;

        assert_eq!(
            result.project_context.jdk,
            JdkStatus::Detected(crate::runtime::JdkInfo {
                version: "25".to_string(),
                vendor: Some("Runtime Vendor".to_string()),
                runtime: Some("Runtime JDK".to_string()),
            })
        );
        assert_eq!(
            result
                .project_context
                .build_tool_runtime
                .as_ref()
                .and_then(|runtime| runtime.jdk.as_ref())
                .map(|jdk| jdk.version.as_str()),
            Some("21")
        );
        assert_eq!(
            result.project_context.project.root_module.jvm_targets[0].version,
            "17"
        );
        assert!(result.findings.is_empty());
        assert_eq!(executor.requests.borrow().len(), 2);
        Ok(())
    }

    #[test]
    fn build_tool_collection_failure_is_optional_evidence() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = TestDirectory::maven_project()?;
        let executor = FakeExecutor::new(vec![
            FakeResult::Error(io::ErrorKind::NotFound),
            success("", "java.version = 25"),
        ]);

        let result = run_with(&directory.0, &executor)?;

        assert_eq!(result.project_context.build_tool_runtime, None);
        assert!(matches!(result.project_context.jdk, JdkStatus::Detected(_)));
        assert!(result.findings.is_empty());
        Ok(())
    }
}
