# JMend --- Product Concept

## Overview

JMend — JVM and GraalVM diagnostics, compatibility checks, and project
analysis.

The current implementation provides a native CLI that detects Maven and Gradle
projects and the local JDK. The architecture is prepared for future analysis
of resolved dependencies, classpaths, JAR files, bytecode, GraalVM, and Native
Image projects; those analyzers and the planned TUI are not implemented yet.

JMend is a standalone developer tool. It does not require integration
into the target application.

## Problem

JVM projects can fail because of issues that are not obvious from
`pom.xml` or `build.gradle` alone.

Typical examples:

-   dependency version conflicts;
-   duplicate classes and resources;
-   split packages;
-   conflicting `ServiceLoader` providers;
-   incompatible bytecode versions;
-   missing classes;
-   binary incompatibilities;
-   potential `NoSuchMethodError`;
-   potential `NoSuchFieldError`;
-   `ClassNotFoundException`;
-   `NoClassDefFoundError`;
-   JDK and build-toolchain mismatches.

Developers often need several different tools and manual investigation
to determine the actual cause.

## Goal

JMend should detect JVM project problems and explain:

1.  What is wrong?
2.  Why is it wrong?
3.  Where did it come from?
4.  What should the developer investigate next?

The primary focus is pre-flight diagnostics: detect problems before they
become difficult runtime failures.

## Target Users

-   Java and JVM developers;
-   library/framework maintainers;
-   developers working with large Maven or Gradle projects;
-   developers debugging dependency and classpath issues;
-   CI/CD pipelines that need JVM project diagnostics.

## Core Principles

### Native Tool

JMend is distributed as a standalone native executable.

Target usage:

``` bash
brew install jmend
```

The JMend process itself does not require a JVM. A JDK or project
build tool may be required when project-specific information must be
resolved.

### CLI + TUI

Interactive usage:

``` bash
jmend
```

Automation and CI:

``` bash
jmend check
```

Both interfaces use the same analysis engine and finding model.

### Explain, Not Just Report

A finding should include evidence and context.

Instead of:

``` text
Multiple versions of foo-core detected.
```

JMend should eventually be able to report:

``` text
foo-client was compiled against foo-core 1.4.

The effective runtime classpath contains foo-core 2.0.

foo-client invokes:
Foo.connect(String)

foo-core 2.0 provides:
Foo.connect(String, Duration)

Potential NoSuchMethodError.
```

### Do Not Reimplement Maven or Gradle

Maven and Gradle remain responsible for dependency resolution.

JMend consumes the effective project/classpath information and
performs diagnostics on top of it.

### UI-Independent Core

``` text
Project
   |
   v
Project Model
   |
   v
Analyzers
   |
   v
Findings
  /   \
CLI   TUI
```

Diagnostic logic must not depend on terminal UI code.

### Extensible Analysis

Diagnostics should be implemented as independent analyzers.

Potential analyzers:

-   project analyzer;
-   dependency analyzer;
-   classpath analyzer;
-   duplicate-class analyzer;
-   linkage analyzer;
-   bytecode analyzer;
-   JDK analyzer;
-   Spring analyzer;
-   GraalVM analyzer.

## Core Domains

### Project

-   project root;
-   Maven/Gradle detection;
-   modules;
-   Java version;
-   packaging;
-   build files.

### Dependencies

-   resolved dependencies;
-   dependency paths;
-   version conflicts;
-   convergence problems;
-   dependency origin.

### Classpath

-   duplicate classes;
-   duplicate resources;
-   split packages;
-   conflicting service providers;
-   missing classes.

### JAR Inspection

-   manifest;
-   classes;
-   resources;
-   `META-INF/services`;
-   multi-release JAR structure;
-   class metadata.

### Bytecode and Linkage

Future analysis of JVM class references should enable detection of
potential:

-   `NoSuchMethodError`;
-   `NoSuchFieldError`;
-   missing referenced classes;
-   incompatible class versions.

## Finding Model

A finding is the central diagnostic result.

Conceptually:

``` text
id
severity
category
title
description
evidence
location
suggestion
```

Initial severities:

-   INFO
-   WARNING
-   ERROR
-   CRITICAL

## Primary Commands

``` bash
jmend
jmend check
jmend why <dependency>
jmend inspect <jar>
```

Future:

``` bash
jmend explain <stacktrace-file>
jmend compare <old.jar> <new.jar>
```

## Future Extensions

### Spring

-   configuration diagnostics;
-   configuration metadata;
-   profiles;
-   auto-configuration inspection.

### GraalVM

-   Polyglot artifact consistency;
-   language artifact versions;
-   runtime resource diagnostics.

### Security

-   vulnerable dependency findings.

### Comparison

-   dependency changes;
-   binary API changes;
-   class changes;
-   newly introduced classpath problems.

## Non-Goals

JMend is not intended to replace:

-   Maven;
-   Gradle;
-   IDEs;
-   JFR;
-   VisualVM;
-   JVM profilers;
-   application monitoring platforms;
-   Spring administration tools.

## Distribution

Initial targets:

-   macOS Apple Silicon;
-   macOS x86-64;
-   Linux x86-64;
-   Linux ARM64.

Primary installation target:

``` bash
brew install jmend
```

Additional distribution may include GitHub Releases and crates.io.

## Implementation

JMend is implemented in Rust.

Rust is an implementation choice because the project benefits from:

-   fast startup;
-   native binaries;
-   predictable resource usage;
-   safe low-level parsing;
-   efficient processing of large JAR/classpath sets;
-   strong CLI/TUI ecosystem;
-   cross-platform distribution.

JMend is a JVM developer tool implemented in Rust, not a Rust tool for
JVM developers.
