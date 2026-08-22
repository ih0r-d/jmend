# JMend

```text
     _ __  __                _
    | |  \/  | ___ _ __   __| |
 _  | | |\/| |/ _ \ '_ \ / _` |
| |_| | |  | |  __/ | | | (_| |
 \___/|_|  |_|\___|_| |_|\__,_|
```

**JVM & GraalVM diagnostics for JVM projects.**

[![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)](https://www.rust-lang.org/)
[![Task](https://img.shields.io/badge/Task-Taskfile-29BEB0?logo=task)](https://taskfile.dev/)
[![License](https://img.shields.io/github/license/ih0r-d/jmend)](LICENSE)

JMend is a developer tool for inspecting JVM projects, their build environment,
runtime configuration, dependencies, classpath, and GraalVM compatibility.

The project is written in Rust and designed as a fast, cross-platform CLI with
a future interactive TUI for deeper project diagnostics.

> **Status:** JMend is under active development. The current version provides
> the initial JVM project fingerprint; deeper analyzers are being implemented
> incrementally.

## Quick Start

Run JMend from a JVM project:

```console
jmend check
```

Example:

```text
JMend v0.1.0 · macOS arm64 · HEALTHY

Project         Maven         [10 modules]
JDK             25            [GraalVM Community]

0 errors · 0 warnings
```

Running JMend without a command displays the CLI entry point and available
commands:

```console
jmend
```

## Project Fingerprint

JMend currently detects:

- Maven and Gradle JVM projects
- Maven multi-module project structure
- local JDK version
- JDK vendor
- Maven and Gradle wrapper metadata
- host operating system and architecture

The internal project model supports multi-module JVM projects from the
beginning, allowing different modules to expose their own languages,
frameworks, build metadata, and runtime information.

## Planned Analysis

JMend is being built incrementally around independent analyzers.

Planned capabilities include:

- JVM language and language-level detection
- framework detection
- Maven and Gradle build-tool diagnostics
- dependency graph analysis
- dependency conflict detection
- classpath inspection
- bytecode compatibility analysis
- native library inspection
- GraalVM and Polyglot diagnostics
- Native Image diagnostics
- structured JSON output
- interactive TUI

The compact `jmend check` command is intended to remain a quick project health
overview. Detailed inspection and navigation will belong to the TUI and
specialized commands.

## Development

JMend requires a Rust toolchain compatible with the Rust 2024 edition.

Run all validation:

```console
task verify
```

Run JMend against the current project:

```console
task run-check
```

Or directly with Cargo:

```console
cargo run --bin jmend -- check
```

Individual checks:

```console
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

## Roadmap

Development is tracked through
[GitHub Issues](https://github.com/ih0r-d/jmend/issues) and
[Milestones](https://github.com/ih0r-d/jmend/milestones).

The current milestone focuses on the initial **Project Fingerprint** and the
foundation required for deeper JVM diagnostics.

## License

See [LICENSE](LICENSE).