# JMend

```text
     _ __  __                _
    | |  \/  | ___ _ __   __| |
 _  | | |\/| |/ _ \ '_ \ / _` |
| |_| | |  | |  __/ | | | (_| |
 \___/|_|  |_|\___|_| |_|\__,_|
```

**JMend — JVM and GraalVM readiness diagnostics and risk analysis.**

[![Rust](https://img.shields.io/badge/Rust-2024-orange?logo=rust)](https://www.rust-lang.org/)
[![Task](https://img.shields.io/badge/Task-Taskfile-29BEB0?logo=task)](https://taskfile.dev/)
[![License](https://img.shields.io/github/license/ih0r-d/jmend)](LICENSE)

JMend is an independent JVM ecosystem diagnostics and risk-analysis tool. Its
long-term purpose is to correlate evidence across project structure,
toolchains, dependencies, classpaths, bytecode, native components, GraalVM,
Native Image, and Polyglot usage so compatibility problems and engineering
risks can be found before builds, CI, or production.

> **Status:** the current `0.1` foundation provides a project fingerprint, not
> the future analysis described below. It now collects compiled artifact and
> static bytecode evidence, but does not yet analyze dependencies, classpaths,
> GraalVM, Native Image, or Polyglot readiness, and it emits no
> diagnostic findings.

## What works today

- JVM project/root detection for Maven and Gradle build files
- recursive Maven multi-module discovery
- Maven and Gradle wrapper metadata detection
- direct Maven Java target evidence for root and child build units
- wrapper-first Maven and Gradle version/runtime JDK evidence
- automatic per-build-unit `.class`/JAR discovery from conventional Maven and Gradle outputs
- Java 17–25 class structure, bytecode, reference, and API-usage evidence
- safe JAR manifest and Multi-Release JAR inspection
- detailed `jmend inspect <path>` for projects, modules, JARs, and class files
- local JDK version, vendor, and runtime detection
- host operating-system and architecture metadata
- a structured project/module, analysis, and findings foundation
- the branded root CLI and compact `jmend check`
- repository CI validation and cross-platform snapshot builds

Run JMend from a JVM project:

```console
jmend check
jmend inspect path/to/application.jar
jmend inspect path/to/project
```

The output contains only detected, implemented information; roadmap
placeholders are intentionally excluded. Build-tool evidence is optional when
the selected wrapper or system command cannot be executed. Running `jmend`
shows the branded command entry point and help for implemented commands only.

## Direction

JMend is designed for the wider JVM ecosystem: Java, Kotlin, Scala, Groovy,
and Clojure; Maven, Gradle, SBT, Mill, and Ant; and significant JVM frameworks
and platforms. These are compatibility targets, not claims of current support.

Planned analysis spans JVM/toolchains, builds, dependencies, classpaths,
bytecode, and native components. GraalVM and Native Image are first-class
specializations, and Polyglot configuration and cross-language risks are a
major intended differentiator.

Build tools and runtimes are data sources, not the product. JMend may consume
their models and output as evidence, but its value comes from normalizing that
evidence, correlating it across layers, and producing independent findings and
readiness conclusions. It is not a Maven, Gradle, or `native-image` wrapper.

See the [product concept](docs/00-product-concept.md), [use
cases](docs/01-use-cases.md), [architecture](docs/03-architecture.md), and
[roadmap](docs/04-roadmap.md) for the planned scope and explicit anti-goals.

## Development

JMend requires a Rust toolchain compatible with the Rust 2024 edition.

```console
task verify
task run-check
```

Or run the checks directly:

```console
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
```

The current GitHub workflows validate JMend itself and create snapshot builds.
They are distinct from the planned user-facing JMend CI analysis mode.

## License

See [LICENSE](LICENSE).
