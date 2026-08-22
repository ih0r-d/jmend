# JMend — Use Cases

## Current workflow: project fingerprint

A developer runs:

```console
jmend check
```

Today JMend finds the nearest Maven or Gradle project root, discovers declared
Maven modules recursively, records wrapper metadata, detects local JDK
version/vendor/runtime and host OS/architecture, and attempts one wrapper-first
build-tool version query. When available, Maven or Gradle version and its
execution-JDK evidence are retained separately from the JMend runtime JDK and
module targets. The compact result contains only collected evidence and real
findings, never roadmap placeholders. It does not yet resolve dependencies or
emit diagnostic findings. Running `jmend` displays the branded CLI entry point
and help for implemented commands; there is no TUI yet.

## Planned diagnostic workflows

The following use cases describe product direction, not implemented commands
or analyzers.

### JVM and toolchain readiness

JMend collects module targets, runtime and build-tool JDKs, and class-file
versions, then correlates them. A Java 21 target combined with dependency
bytecode requiring Java 25 should yield an evidence-backed compatibility
finding rather than merely two unrelated facts.

### Build and multi-module consistency

JMend compares effective project models, wrappers, toolchains, targets, and
module configuration to identify reliably provable mismatches. Maven and
Gradle still own build execution and dependency resolution; their models are
evidence for JMend analysis.

### Dependency, classpath, and bytecode risks

Planned inspection includes dependency paths and conflicts, duplicate classes,
split packages, resource and service-provider collisions, unsupported
bytecode, and missing referenced classes/methods/fields where reliable. Future
drill-down may explain dependency origin or inspect JAR contents, but JMend is
not intended as a generic dependency listing or archive viewer.

### Native component readiness

JMend inventories bundled `.so`, `.dylib`, and `.dll` components and correlates
their operating system and architecture with build/runtime targets. For
example, an arm64 Native Image target with an x86_64 JNI library should produce
a native architecture incompatibility finding.

### GraalVM and Native Image readiness

JMend evaluates GraalVM runtime and dependency consistency and treats Native
Image as a first-class domain. Planned evidence covers reachability metadata,
reflection, resources, serialization, proxies, JNI, and native libraries. A
resolved library at version 2.4 paired with metadata intended for 2.1 should
produce a metadata compatibility risk when that conclusion is supportable.

### Polyglot risks

JMend identifies Polyglot API and guest-language usage and analyzes Engine,
Context, HostAccess, sandbox, lifecycle, and cross-language configuration. For
example, unrestricted host access should become an explainable security finding
in the appropriate context. GraalPy and GraalJS are important future targets.

### Detailed exploration

A future TUI will explore Overview, Modules, Toolchains, Languages, Frameworks,
Dependencies, Classpath, Bytecode, Native, GraalVM, Native Image, Polyglot, and
Findings. It will consume the same structured result as CLI and CI rather than
hosting separate diagnostic logic.

### CI readiness gate

The conceptual flow is:

```text
PR / build
    ↓
JMend analysis
    ↓
Findings
    ↓
Exit policy / PR annotations / reports
```

Planned capabilities include deterministic exit codes, severity thresholds,
baselines, only-new-findings mode, JSON, SARIF, GitHub Code Scanning
annotations, and PR summaries. The repository's current CI validates and
packages JMend itself; it is not the future user-facing CI mode.

## Finding experience

Every future finding should answer:

1. What did JMend find?
2. Where did it find it, and what module or artifact is affected?
3. Why does it matter, and what readiness dimension is affected?
4. What evidence supports the conclusion?
5. How can the developer investigate or remediate it?

Language and framework detection enrich this context. They are evidence, not
the primary product value; compatibility conclusions produced by correlating
evidence are the value.
