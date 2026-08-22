# JMend — Scope and Delivery Principles

This document replaces the older dependency/JAR/TUI MVP plan. The immediate
foundation and future milestones are tracked in [the roadmap](04-roadmap.md).
No future analyzer described here is implemented unless explicitly stated.

## Product objective

JMend is an independent JVM ecosystem diagnostics and risk-analysis tool with
deep GraalVM, Native Image, and Polyglot awareness. Its core proof is not that
a native CLI can invoke a build tool; it is that structured evidence from many
layers can be normalized, correlated, and turned into explainable findings.

> **Build tools and runtimes are data sources, not the product.**

## Current foundation

The `0.1` code currently implements:

- upward JVM project/root detection using Maven and Gradle build files;
- recursive discovery of direct Maven `<modules>` declarations;
- direct Maven Java target evidence for root and child build units;
- Maven and Gradle wrapper executable/metadata detection;
- wrapper-first Maven and Gradle version and execution-JDK evidence;
- local JDK version, vendor, and runtime detection;
- host operating-system and architecture metadata;
- structured project, module, language/framework metadata, capability,
  analysis, finding, and check-result types;
- a branded root CLI and compact `jmend check` output;
- repository validation and cross-platform snapshot workflows.

Modeled language, framework, capability, or build-tool variants do not mean
they are detected or analyzed. In particular, Gradle multi-project discovery,
language/framework detection, Gradle and effective/inherited Maven target
resolution, broader build-tool toolchain data, dependency/classpath inspection,
and findings remain unfinished.

## Delivery principles

- Collection and inspection establish facts; analyzers derive conclusions.
- Project and module data are normalized independently of presentation.
- Findings, not process failures, represent detected project risks.
- Analyzers must add independent value rather than duplicate Maven, Gradle,
  `jdeps`, GraalVM Native Build Tools, or dependency managers.
- CLI, future TUI, CI, JSON, and SARIF consume the same structured result.
- Multi-module and multi-language projects remain first-class.
- New claims appear in user-facing output only after supporting analysis exists.
- Capabilities are delivered incrementally without release-date promises.

## Explicitly outside the current foundation

- dependency graph or classpath resolution and analysis;
- JAR and bytecode inspection;
- linkage diagnostics;
- native-library analysis;
- GraalVM, Native Image, or Polyglot analysis;
- a TUI or specialized inspection/explanation commands;
- JSON, SARIF, CI policy, baselines, or PR annotations;
- vulnerability scanning, runtime attach, JFR, heap/GC/thread monitoring;
- automatic fixes, IDE plugins, web UI, or remote analysis.

These exclusions describe current implementation status, not permanent product
anti-goals. The planned analysis sequence is defined in the roadmap.
