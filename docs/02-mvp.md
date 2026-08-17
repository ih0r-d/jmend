# JDoctor --- MVP

## Objective

The first JDoctor release should prove one core idea:

> A native tool can inspect a real JVM project and provide useful
> dependency/classpath diagnostics through both CLI and TUI interfaces.

The MVP must remain small enough to build while learning Rust.

## Supported Project Type

Initial support:

-   Maven and Gradle project detection;
-   single-module Java project;
-   cross-platform development.

Resolved Gradle models and multi-module support are intentionally deferred.

## MVP Commands

### Interactive Mode

``` bash
jdoctor
```

Opens the TUI for the current project.

### Check

``` bash
jdoctor check
```

Runs diagnostics and prints findings without opening the TUI.

### Why

``` bash
jdoctor why <artifact>
```

Shows dependency paths that introduce an artifact.

### Inspect

``` bash
jdoctor inspect <jar>
```

Shows basic JAR information.

## MVP Phase 1 --- Project Detection

JDoctor must:

-   use the current directory;
-   detect `pom.xml`, `build.gradle`, or `build.gradle.kts`;
-   identify the project as Maven or Gradle;
-   determine project root;
-   expose basic project metadata.

Example:

``` text
Project: payment-service
Path: /Users/user/projects/payment-service
Build: Maven
```

## MVP Phase 2 --- Maven Integration

JDoctor must use the project's Maven wrapper when available.

Priority:

``` text
./mvnw
mvn
```

Maven remains responsible for dependency resolution.

JDoctor must obtain enough resolved dependency/classpath information for
analysis.

JDoctor must not implement Maven dependency resolution itself.

## MVP Phase 3 --- JAR Model

JDoctor must be able to scan JAR files from the resolved classpath.

For each JAR, collect at least:

-   path;
-   artifact identity when known;
-   classes;
-   resources;
-   manifest presence;
-   `META-INF/services` entries.

No bytecode instruction analysis is required yet.

## MVP Phase 4 --- Initial Analyzers

### Duplicate Class Analyzer

Detect the same `.class` path in multiple classpath entries.

Finding should contain:

-   class name;
-   containing JARs;
-   severity;
-   evidence.

### Duplicate Resource Analyzer

Detect duplicated resource paths.

Filtering may be required to avoid excessive noise.

### Service Provider Analyzer

Inspect `META-INF/services` and expose provider collisions or duplicate
definitions.

### Dependency Version Analyzer

Detect multiple versions/requests of the same artifact where the Maven
model provides enough information.

## MVP Phase 5 --- Finding Model

All analyzers must return the same finding model.

Required concepts:

``` text
Finding
Severity
Category
Evidence
```

Initial severities:

``` text
INFO
WARNING
ERROR
CRITICAL
```

The analysis core must not format findings specifically for CLI or TUI.

## MVP Phase 6 --- CLI Output

``` bash
jdoctor check
```

should produce concise output such as:

``` text
JDoctor — payment-service

2 warnings

WARNING  Duplicate class
         com.example.Utils

         foo-core-1.2.jar
         legacy-sdk-4.1.jar

WARNING  Multiple dependency versions
         jackson-core

         2.18.4
         2.20.0
```

Initial exit-code behavior can remain simple and be refined later.

## MVP Phase 7 --- TUI

After the CLI/core is stable, add the interactive interface.

Initial TUI requires only:

-   project summary;
-   findings list;
-   filtering by category/severity;
-   finding details;
-   keyboard navigation;
-   rescan.

Conceptual layout:

``` text
┌ JDoctor — payment-service ─────────────────────────────┐
│ Maven | Java 21 | 4 findings                          │
├──────────────────────┬────────────────────────────────┤
│ Findings             │ Details                        │
│                      │                                │
│ > Duplicate class    │ com.example.Utils              │
│   Duplicate class    │                                │
│   Version conflict   │ foo-core-1.2.jar               │
│   Service provider   │ legacy-sdk-4.1.jar             │
│                      │                                │
├──────────────────────┴────────────────────────────────┤
│ / filter       r rescan       q quit                  │
└───────────────────────────────────────────────────────┘
```

The TUI must remain a presentation layer over the same analysis core
used by `jdoctor check`.

## MVP Phase 8 --- JAR Inspection

``` bash
jdoctor inspect foo.jar
```

Initial information:

-   file path;
-   size;
-   manifest;
-   class count;
-   resource count;
-   service descriptors.

Interactive JAR browsing can come later.

## Explicitly Out of MVP

The following are not required for the first usable release:

-   resolved Gradle model and classpath integration;
-   multi-module Maven;
-   bytecode instruction analysis;
-   linkage analysis;
-   `NoSuchMethodError` prediction;
-   `NoSuchFieldError` prediction;
-   stack-trace explanation;
-   Spring diagnostics;
-   GraalVM diagnostics;
-   vulnerability scanning;
-   JFR;
-   JVM attach;
-   heap/GC/thread monitoring;
-   build comparison;
-   automatic fixes;
-   IDE plugins;
-   web UI;
-   remote analysis.

## Initial Rust Stack

Start with the minimum required dependencies.

### Initial

-   Rust stable, Edition 2024;
-   `clap` --- CLI;
-   `thiserror` --- application/domain errors.

### Add When Needed

-   `zip` --- JAR reading;
-   `serde` / `serde_json` --- structured output and persistence;
-   `petgraph` --- dependency graphs;
-   `ratatui` --- TUI;
-   `crossterm` --- terminal backend;
-   `tracing` --- internal diagnostics;
-   `rayon` --- parallel JAR scanning if performance requires it.

Do not introduce `tokio` unless an actual asynchronous use case appears.

## Development Strategy

The project is also used to learn Rust.

Development should therefore progress through small vertical slices.

Recommended order:

``` text
Rust basics
    ↓
CLI
    ↓
Project detection
    ↓
Maven integration
    ↓
JAR scanning
    ↓
Finding model
    ↓
Initial analyzers
    ↓
TUI
    ↓
Advanced JVM analysis
```

Avoid introducing abstractions before they are needed.

In particular:

-   do not reproduce Spring-style service/factory architecture in Rust;
-   do not create many crates at the beginning;
-   do not use `.clone()` only to silence ownership errors;
-   prefer understanding ownership and borrowing before working around
    them;
-   keep the analysis core independent from UI.

## Definition of MVP Done

The MVP is considered complete when a developer can:

``` bash
brew install jdoctor
cd some-maven-project
jdoctor
```

and interactively inspect useful classpath findings.

The same project must also support:

``` bash
jdoctor check
```

for non-interactive diagnostics.

At minimum, JDoctor must detect real duplicate-class or related
classpath findings from the resolved Maven project and explain where
they were found.
