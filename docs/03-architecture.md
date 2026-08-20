# JMend Architecture

## Current Scope

The current implementation establishes the first vertical slice: `jmend
check` searches the current directory and its parents for a Maven or Gradle
project, produces a structured check result, and renders its build tool plus
the detected local JDK to the console. It does not resolve
dependencies, detect project languages or frameworks, or produce diagnostic
findings yet.

The architecture is intentionally small. New modules should be added when a
working capability requires them, not in anticipation of every planned JVM or
framework integration.

## Analysis Pipeline

``` text
CLI parsing
    ↓
Command dispatch
    ↓
Project detection
    ↓
Project context and detected capabilities
    ↓
Analyzer selection (planned)
    ↓
Independent analyzers (planned)
    ↓
Findings
    ↓
CheckResult
    ↓
Console, JSON, TUI, or CI presentation
```

Only project detection, structured check results, and console output are active
today. JSON, TUI, and dedicated CI presentation are planned. The analysis types
define the boundary needed by future analyzers but no fake analyzers or findings
are registered.

## Boundaries

### CLI and commands

`cli` owns command-line syntax. `commands` translates parsed commands into
application operations. `main` is the process boundary: it reports application
errors and chooses the exit code. A check operation returns a `CheckResult`
containing its `ProjectContext` and findings. It does not write output.

### Project model and detection

`project` contains the framework-neutral project model. Each `BuildTool` has a
typed descriptor that keeps its build-file names, wrapper executable names,
wrapper metadata path, and future system command together in compile-time Rust
code. The type represents Maven, Gradle, SBT, Mill, and Ant, while detection is
currently enabled only for Maven and Gradle build files (`pom.xml`,
`build.gradle`, and `build.gradle.kts`). It then looks for a project-local
Maven or Gradle wrapper and, when present, records its executable path and
optional wrapper metadata path. Wrapper files alone never make a directory a
project.

Detection begins at the requested directory and walks upward through its
ancestors until it finds the nearest supported project root or reaches the
filesystem root. A `Project` supports zero, one, or many `ProjectModule`
values. Each module has an optional name, path, optional build file and target
runtime metadata, and collections of typed JVM language and framework
metadata. Language kinds currently modeled are Java,
Kotlin, Scala, Groovy, and Clojure. Framework kinds currently modeled are
Spring Boot, Quarkus, Micronaut, Helidon, and Hibernate. Each metadata entry
has an optional version so polyglot and multi-framework modules do not require
language-specific or framework-specific fields. `ProjectContext` combines the
project and its modules with the detected JDK and project-wide capabilities.

Module metadata is the source of truth for languages and frameworks. The
default console may aggregate distinct module values for a compact overview,
but it does not copy that aggregate into project-level state. Detailed
per-module presentation is reserved for future CLI, JSON, and TUI consumers.

The installed JDK and a project's Java language level are distinct concepts.
The JDK describes the detected runtime/toolchain, including its version and
optional vendor. A Java language entry describes source or target compatibility
only when build-tool analysis can determine it reliably. JMend does not infer a
Java language level from the installed JDK.

Maven and Gradle will remain responsible for dependency resolution. JMend
will consume their resolved models and classpaths rather than implementing a
second dependency resolver.

### Analysis

The analysis core depends on project data, not on CLI or presentation code. An
analyzer receives an `AnalysisContext` and returns findings or an
`AnalysisError`. Generic JVM analyzers must not require any framework
capability.

### Output

Output modules transform domain results for a consumer. The first consumer is
the console. Formatting begins only after a complete `CheckResult` exists. Future
JSON, TUI, and CI output will consume the same structured result, project
context, language/framework metadata, JDK state, and findings without changing
commands or analyzers. Version-list separators, brackets, labels, colors, and
other display grammar remain presentation concerns rather than domain fields.

## Errors and Findings

An error means JMend could not complete an operation. Examples include being
unable to determine the current directory, inspect a build file, or write
output. Errors travel through `Result` and reach `main`, which reports them to
the user.

A finding is a successful diagnostic result about the analyzed project, such
as a duplicate class or a possible missing method. Findings have severity and
category and are suitable for every output format. Expected project defects
must be findings, not application errors.

No findings are emitted by the current project-detection slice.

Module discovery is not implemented today, so detected projects have an empty
module collection and language/framework rows remain unanalyzed. Future Maven
discovery should consume the effective reactor/model, while Gradle discovery
should consume included projects from its settings/model. JMend must not infer
membership by recursively treating every nested build file as a module. The
model supports module values without treating structural support as a
successful detection.

## Extension Strategy

Project capabilities are detected separately from analysis. Framework-neutral
analyzers can run for every suitable JVM project. Optional Spring Boot,
Quarkus, Micronaut, GraalVM, Polyglot, or Native Image analyzers should be
selected only after the matching capability is detected.

Build-tool adapters should invoke Maven or Gradle to obtain effective project
information. Later analyzers may inspect that information, classpaths, JARs,
bytecode, JPMS metadata, and native libraries. Each addition should preserve
the direction of dependencies: presentation depends on the core; the core
does not depend on presentation.

Build execution is not implemented yet. When it is added, adapters should
prefer the detected project wrapper and fall back to the descriptor's system
command (`mvn` or `gradle`) when no wrapper is available.
