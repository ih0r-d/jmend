# JMend — Product Concept

## Positioning

**JMend — JVM and GraalVM readiness diagnostics and risk analysis.**

JMend is an independent JVM ecosystem diagnostics and risk-analysis tool. It
is intended to analyze project structure, toolchains, dependencies, classpath,
bytecode, native components, GraalVM, Native Image, and Polyglot usage to find
compatibility problems and engineering risks before they reach builds, CI, or
production.

The current `0.1` implementation is the project-fingerprint foundation. It
detects supported project roots, Maven modules, wrapper metadata, the local
JDK, and the host platform. The deeper analysis in this document is planned.

## Product principle

> **Build tools and runtimes are data sources, not the product.**

Maven, Gradle, JDKs, GraalVM, project files, classpaths, bytecode, native
binaries, and generated metadata can provide evidence. JMend's independent
value comes from:

1. collecting structured evidence;
2. normalizing it into a common project model;
3. correlating information across different layers;
4. generating findings, risks, and readiness conclusions.

JMend should not merely invoke existing commands and reformat their output.
It should avoid duplicating build-tool behavior when no independent diagnostic
value is added.

## Collection and analysis

Collection answers factual questions: what modules, JDKs, resolved
dependencies, bytecode versions, native binaries, and GraalVM metadata exist?
Analysis asks whether those facts form a valid and safe combination and what
runtime, build, compatibility, security, performance, or Native Image risk
follows.

For example, collection may establish that a project targets Java 21 and a
dependency contains Java 25 bytecode. Analysis turns those facts into a JVM
compatibility finding. This separation is a fundamental architecture
principle: evidence gathering must not silently become a diagnostic conclusion.

## Ecosystem scope

JMend is not Java-only. Its future compatibility scope includes Java, Kotlin,
Scala, Groovy, and Clojure projects and Maven, Gradle, SBT, Mill, and Ant build
tools. Project context may include Spring Boot, Quarkus, Micronaut, Helidon,
Hibernate, and other significant JVM frameworks and platforms.

These are ecosystem targets. Today, JMend detects Maven and Gradle roots and
models these language, framework, and build-tool kinds, but it does not yet
detect languages or frameworks, execute build tools, or support every modeled
build tool.

## Analysis domains

The long-term domains are:

- **JVM / Toolchain:** runtime and target compatibility, module target
  differences, build-tool JDK differences, bytecode levels, JDK-internal APIs,
  and JPMS issues.
- **Build:** project-model consistency, runtime/tool mismatch, wrapper and
  toolchain configuration, and multi-module inconsistencies.
- **Dependencies:** conflicting versions, graph inconsistencies, compatibility
  risks, and changes that affect GraalVM or Native Image readiness.
- **Classpath / Bytecode:** duplicate classes, split packages, conflicting
  resources and service providers, unsupported bytecode, and reliably
  detectable missing classes, methods, or fields.
- **Native components:** `.so`, `.dylib`, and `.dll` inventory; OS and
  architecture mismatch; and JNI/native dependency risks.
- **GraalVM:** runtime compatibility, dependency/version consistency,
  reachability-metadata compatibility, and configuration risks.
- **Native Image:** reflection, resources, serialization, proxies, JNI,
  reachability metadata, native-library architecture, metadata versions, and
  AOT-specific compatibility. Native Image is a first-class domain rather than
  a hidden metadata detail.
- **Polyglot:** Polyglot API and guest-language detection, GraalPy and GraalJS,
  Engine and Context lifecycle/configuration, HostAccess and sandbox boundaries,
  language/runtime compatibility, cross-language dependencies, and potential
  performance, security, or lifecycle risks.

None of these domain analyzers is implemented in the current foundation.

## Correlation is the differentiator

JMend should find problems across layers, not just inventory each layer:

- Java 21 module + Java 25 dependency bytecode → JVM compatibility risk.
- arm64 Native Image target + x86_64 JNI library → native architecture risk.
- dependency 2.4 + reachability metadata for 2.1 → metadata compatibility risk.
- Polyglot API + unrestricted host access → Polyglot security risk.
- incompatible GraalVM/Polyglot versions across modules → cross-module risk.

These are conceptual examples, not current functionality.

## Findings and readiness

A finding should eventually explain what JMend found, where it found it, why it
matters, the supporting evidence, the affected module or artifact, the
readiness impact, and a useful remediation. A future risk model may use
Critical, High, Medium, Low, and Info severities.

The current Rust model contains code, severity, category, title, and
description fields with Info, Warning, Error, and Critical severities. It emits
no findings today. The model may evolve deliberately rather than being changed
only to imitate the future terminology.

## Interfaces

`jmend check` remains the compact overview: eventually a small fingerprint,
readiness checks, and a findings/risk summary. Until analyzers exist it must
show only real fingerprint data and must not fake readiness.

A future TUI will support detailed exploration of Overview, Modules,
Toolchains, Languages, Frameworks, Dependencies, Classpath, Bytecode, Native,
GraalVM, Native Image, Polyglot, and Findings. CLI, TUI, CI, JSON, and SARIF
should consume the same structured analysis result.

Future user-facing CI can apply deterministic exit policies and severity
thresholds, compare baselines, report only new findings, emit JSON or SARIF,
and create GitHub annotations and PR summaries. Repository workflows currently
test and package JMend itself; they are not that product capability.

## Anti-goals

JMend should not become:

- a generic build-tool wrapper or replacement for Maven or Gradle;
- a replacement for GraalVM Native Build Tools or a `native-image` wrapper;
- a replacement for `jdeps` or dependency managers;
- a prettier frontend that only aggregates command output;
- a generic dependency-listing tool;
- a framework-specific analyzer limited to Spring;
- a Java-only tool.

Existing tools remain useful evidence sources. JMend must add independent,
cross-layer analysis value.
