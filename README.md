# JMend

JVM and GraalVM diagnostics, compatibility checks, and project analysis.

JMend is a Rust 2024 command-line tool for inspecting Maven and Gradle JVM
projects. The current implementation detects the nearest supported project,
detects the local JDK, records Maven or Gradle wrapper metadata, and renders a
compact health overview.

```console
jmend check
```

Dependency, classpath, Native Image, GraalVM, JSON, and TUI analysis remain
planned and are not implemented yet.

## Development

```console
task verify
task run-check
```
