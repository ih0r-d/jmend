# JMend Roadmap

This roadmap describes scope, not release dates or promises. Items are complete
only when the repository implementation and tests support them.

## 0.1 — Project Fingerprint

Foundation scope:

- project/root detection;
- Maven and Gradle build-file detection;
- Maven multi-module discovery;
- JDK and host environment metadata;
- structured project/module and findings foundation;
- compact CLI and branded root command;
- repository CI and snapshot builds.

Implemented today: all items above, including wrapper metadata detection.
Still-valid follow-up work associated with the fingerprint includes language
and framework detection, Gradle multi-project support, build-tool runtime and
version data, project Java target detection, and the first compatibility
findings. These should not be marked complete merely because supporting model
types or planned output rows exist.

Language/framework data is project context, build-tool metadata is evidence,
and compatibility findings are the first core diagnostic value.

## 0.2 — JVM Analysis Foundation

Focus:

- evidence and inspection model;
- JVM language and target detection;
- build-tool runtime and toolchain metadata;
- dependency and resolved-classpath model;
- bytecode inspection foundation;
- first cross-layer JVM findings.

## 0.3 — GraalVM & Native Image

Focus:

- GraalVM detection and compatibility;
- Native Image configuration inspection;
- reachability metadata;
- reflection, resources, JNI, serialization, and proxy analysis;
- native-library and architecture analysis;
- Native Image readiness findings.

## 0.4 — Polyglot Analysis

Focus:

- Polyglot API usage and guest languages;
- Engine and Context configuration;
- HostAccess and sandbox/security analysis;
- lifecycle and performance risks;
- cross-language compatibility findings.

## 0.5 — CI & Baselines

Focus:

- stable user-facing CI mode and deterministic exit policy;
- severity thresholds;
- baselines and only-new-findings mode;
- JSON and SARIF;
- GitHub annotations and PR summary integration.

The existing GitHub Actions workflows are repository-development validation
and snapshot packaging. They do not constitute the `0.5` product CI mode.

## Planning guardrails

- Do not duplicate Maven or Gradle when no independent analysis follows.
- Do not present collected facts as findings.
- Do not claim modeled ecosystems or planned analyzers as supported.
- Keep Native Image first-class and Polyglot a major specialization.
- Deliver correlation and explainability before breadth-only inventory.
