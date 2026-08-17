# JDoctor Architecture

## Current Scope

The current implementation establishes the first vertical slice: `jdoctor
check` detects a Maven or Gradle project in the current directory and prints
its root and build tool. It does not resolve dependencies or produce diagnostic
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
Console, JSON, TUI, or CI presentation
```

Only project detection and console project output are active today. The
analysis types define the boundary needed by future analyzers but no fake
analyzers or findings are registered.

## Boundaries

### CLI and commands

`cli` owns command-line syntax. `commands` translates parsed commands into
application operations. `main` is the process boundary: it reports application
errors and chooses the exit code.

### Project model and detection

`project` contains the framework-neutral project model. Its detector recognizes
`pom.xml`, `build.gradle`, and `build.gradle.kts`. A project context combines the
detected project with capabilities discovered in later phases.

Maven and Gradle will remain responsible for dependency resolution. JDoctor
will consume their resolved models and classpaths rather than implementing a
second dependency resolver.

### Analysis

The analysis core depends on project data, not on CLI or presentation code. An
analyzer receives an `AnalysisContext` and returns findings or an
`AnalysisError`. Generic JVM analyzers must not require any framework
capability.

### Output

Output modules transform domain results for a consumer. The first consumer is
the console. Future JSON, TUI, and CI output should consume the same project
contexts and findings without changing analyzers.

## Errors and Findings

An error means JDoctor could not complete an operation. Examples include being
unable to determine the current directory, inspect a build file, or write
output. Errors travel through `Result` and reach `main`, which reports them to
the user.

A finding is a successful diagnostic result about the analyzed project, such
as a duplicate class or a possible missing method. Findings have severity and
category and are suitable for every output format. Expected project defects
must be findings, not application errors.

No findings are emitted by the current project-detection slice.

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
