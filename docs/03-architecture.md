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

`project` owns the normalized project/build-unit context and build-tool
metadata. A `Project` represents the detected build or workspace. Its
`root_module` represents the root build unit and preserves evidence from the
root build file. Its `modules` collection contains declared child build units.
The root is deliberately excluded from `modules.len()`, preserving the compact
CLI's existing child-module count: a single-module build has no count suffix,
while three declared children are shown as three modules. `build_units()`
iterates over the root and children when a derived view needs all build units.

Detection walks upward to the nearest supported root. Maven aggregation follows
direct top-level `<modules>` declarations recursively and represents child
aggregators and leaves once. It does not evaluate profiles, general property
interpolation, inheritance, or an effective Maven model. Gradle multi-project
discovery is not implemented.

The model can represent Maven, Gradle, SBT, Mill, and Ant; detection currently
supports only Maven and Gradle build files. It can represent Java, Kotlin,
Scala, Groovy, and Clojure and several framework kinds, but no language or
framework detector exists yet.

The installed JDK is separate from a module's Java target. Current JDK
detection obtains `java.version`, `java.vendor`, and `java.runtime.name` from
standard JVM properties. It does not infer project targets or GraalVM readiness.

`ProjectModule.jvm_targets` is the authoritative location for JVM target
evidence; `Project` has no target field. The root build unit and every child
therefore retain independent evidence. Any future project-wide summary must be
derived rather than becoming a second source of truth.

Maven inspection currently collects a direct Java `JvmTarget` only from
resolvable compiler declarations in that build unit's own POM. Precedence is
deterministic: Maven Compiler Plugin `<release>`, `maven.compiler.release`,
Maven Compiler Plugin `<target>`, then `maven.compiler.target`. Release
semantics therefore win over target semantics. An unresolved higher-priority
declaration makes the direct target unknown rather than allowing a fallback.
Exact plugin references to the two supported properties are resolved; other
expressions remain unknown. `java.version` and `<source>` alone never imply a
bytecode target. This is evidence collection only and is not compared with the
JDK by this layer.

Direct target evidence is explicitly declared and resolved from the build
unit's own metadata. Effective target evidence would include values produced by
build-tool inheritance and effective configuration. JMend does not yet claim
to compute the Maven effective target. The generic `JvmTarget` can gain
provenance and declared/effective resolution metadata without exposing Maven
property or XML concepts outside the Maven layer.

Maven aggregation and inheritance are separate relationships. A child listed
under `<modules>` does not necessarily inherit from that aggregator. JMend does
not copy root target evidence to children, even when a local `<parent>` is
declared; reliable parent/property/plugin inheritance is deferred until an
appropriately scoped effective-model implementation exists. Unknown evidence
is preferred to an incorrect inherited target.

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
