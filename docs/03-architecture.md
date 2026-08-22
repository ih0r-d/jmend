# JMend Architecture

## Current implementation boundary

The current vertical slice detects the nearest Maven or Gradle project,
discovers Maven modules, records wrapper metadata, detects the local JDK and
host platform, creates a structured `CheckResult`, and renders `jmend check`.
It has analysis and finding interfaces, but registers no analyzers and emits no
findings. Dependencies, classpaths, bytecode, native components, GraalVM,
Native Image, Polyglot, JSON, CI mode, and TUI are planned.

## Architectural direction

```text
Sources
├── Project files / project model
├── Maven / Gradle / future build tools
├── JDK / toolchains
├── Dependencies
├── Resolved classpath
├── JARs / bytecode
├── Native libraries
├── GraalVM metadata
├── Native Image metadata
└── Polyglot configuration / usage

        ↓

Collection / Inspection

        ↓

Normalized Evidence / Project Model

        ↓

Analyzers

        ↓

Findings / Risks / Readiness

        ↓

CLI / future TUI / CI / JSON / SARIF
```

“Collection / Inspection” is a conceptual responsibility, not a requirement
to introduce a Rust `Collector` type now. New types should follow working
capabilities and the repository's existing `project`, `runtime`, `analysis`,
`commands`, and `output` boundaries.

> **Build tools and runtimes are data sources, not the product.**

JMend may invoke or inspect external tools to obtain authoritative data, but it
normalizes evidence and derives its own cross-layer findings. Command output is
not itself a diagnostic conclusion.

## Collection versus analysis

Collection and inspection answer what exists: modules, toolchains,
dependencies, bytecode levels, native binaries, GraalVM metadata, or Polyglot
configuration. Analysis answers whether the collected combination is valid,
compatible, or risky and explains why.

For example:

```text
Evidence: module target = Java 21
Evidence: dependency X class version = Java 25
Analysis: dependency X cannot run on the declared target/runtime
Finding: JVM compatibility risk
```

This boundary prevents detectors from embedding policy, supports reuse of
evidence by multiple analyzers, and makes findings traceable.

## Existing boundaries

### CLI and commands

`cli` owns syntax. `commands` translates it into application operations.
`main` reports application errors and selects process exit behavior. A check
operation returns a `CheckResult`; it does not format output.

### Project and runtime

`project` owns the normalized project/module context and build-tool metadata.
Detection walks upward to the nearest supported root. Maven discovery follows
direct top-level `<modules>` declarations recursively, represents aggregator
and leaf modules once, and excludes the root from the module count. It does not
evaluate profiles, interpolation, inheritance, or an effective Maven model.
Gradle multi-project discovery is not implemented.

The model can represent Maven, Gradle, SBT, Mill, and Ant; detection currently
supports only Maven and Gradle build files. It can represent Java, Kotlin,
Scala, Groovy, and Clojure and several framework kinds, but no language or
framework detector exists yet.

The installed JDK is separate from a module's Java target. Current JDK
detection obtains `java.version`, `java.vendor`, and `java.runtime.name` from
standard JVM properties. It does not infer project targets or GraalVM readiness.

### Analysis and findings

An `Analyzer` accepts an `AnalysisContext` and returns findings or an analysis
error. Findings currently contain code, severity, category, title, and
description. Current severities are Info, Warning, Error, and Critical; a
future risk vocabulary may evolve to Critical, High, Medium, Low, and Info.

Eventually a finding should carry or link to evidence, affected modules or
artifacts, explanation, remediation, and readiness impact. No findings are
emitted today.

An application error means JMend could not complete an operation. A finding is
a successful diagnostic conclusion about the target. Expected project defects
must remain findings rather than application failures.

### Output

`output` transforms a complete domain result for a consumer. The compact check
currently shows real host, project/build-tool, module-count, and JDK metadata;
planned rows are visibly marked planned. It must not imply that absent
analyzers have passed.

Future CLI, TUI, user-facing CI, JSON, and SARIF should consume the same
structured analysis result. The TUI is the detailed exploration interface,
with potential sections for Overview, Modules, Toolchains, Languages,
Frameworks, Dependencies, Classpath, Bytecode, Native, GraalVM, Native Image,
Polyglot, and Findings.

Repository-development CI currently formats, lints, tests, documents, builds,
and packages JMend. It is separate from future product CI features such as
severity policy, baselines, SARIF, or PR annotations.

## Analysis domains and correlation

The primary long-term domains are JVM/toolchain, build, dependencies,
classpath/bytecode, native components, GraalVM, Native Image, and Polyglot.
Native Image remains first-class rather than being folded into a generic
GraalVM metadata flag. Polyglot is a major specialization encompassing API and
guest-language usage, Engine/Context/HostAccess/sandbox configuration, and
lifecycle, security, performance, and cross-language risks.

Analyzers should correlate facts across domains. Conceptual examples include:

- module Java 21 target + Java 25 dependency bytecode → compatibility risk;
- arm64 Native Image target + x86_64 JNI library → architecture risk;
- dependency 2.4 + reachability metadata for 2.1 → metadata risk;
- unrestricted Polyglot host access → security/configuration risk;
- incompatible GraalVM/Polyglot versions between modules → cross-module risk.

These examples are architectural targets, not implemented checks.

## Extension rules

- Prefer authoritative build-tool models for build-specific facts; do not
  implement a second dependency resolver.
- Keep Maven, Gradle, and future tool adapters isolated from generic evidence.
- Keep framework metadata as context unless framework-specific analysis adds
  independent value.
- Keep generic JVM analyzers independent of optional framework capabilities.
- Preserve dependency direction: presentation depends on core analysis; core
  analysis never depends on presentation.
- Prefer explainable, reliable findings over speculative warnings.

## Anti-goals

JMend is not a build-tool wrapper or replacement, a `native-image` wrapper, a
replacement for GraalVM Native Build Tools, `jdeps`, or dependency managers, a
command-output aggregator, a Spring-only analyzer, or a Java-only tool.
