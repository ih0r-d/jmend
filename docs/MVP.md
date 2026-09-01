# JMend MVP

Status: Draft  
Purpose: define the first useful product boundary and preserve the agreed CLI, analysis model, reports, and delivery sequence.

## 1. Product definition

JMend is a local, evidence-based readiness analyzer for Java projects adopting:

- a newer JDK or GraalVM release;
- GraalVM Native Image;
- GraalVM Polyglot Contexts.

JMend answers:

> Can this Java project run on the selected JDK/GraalVM target, including Native Image and its actual Polyglot Contexts? If not, what specifically blocks it, what is only a risk, and how can the result be verified?

JMend does not replace Maven, Gradle, Native Build Tools, Native Image, or the GraalVM Polyglot API. It provides a preflight before expensive builds and a consistent diagnostic report around them.

## 2. User value

### Developer value

- Find predictable Native Image failures before repeatedly running an expensive build.
- See the exact artifact, class, method, file, or configuration responsible.
- Understand whether a result is confirmed, probable, or unknown.
- Inspect the Polyglot Contexts actually created by the application instead of assuming language usage from dependencies.
- Obtain one result locally, in CI, and in machine-readable formats.

### Team value

- Make an evidence-backed go, needs-work, blocked, or unknown decision.
- Estimate migration scope before committing a sprint to it.
- Prevent new dependencies or configuration changes from reintroducing known blockers.
- Compare JDK and GraalVM targets without modifying the project first.

## 3. Product principles

1. Zero configuration by default.
2. Read-only static inspection.
3. No readiness percentages.
4. Evidence before recommendations.
5. Unknown is better than an unsupported guess.
6. Stable rule IDs and finding fingerprints.
7. One canonical analysis model for every renderer.
8. No account, cloud service, or mandatory project plugin.
9. No language-specific Python, JavaScript, Wasm, Ruby, or R advisors.
10. Any Truffle guest language must fit the same Context-based model.

## 4. Terminology

### Context site

A bytecode or source location that creates or configures org.graalvm.polyglot.Context.

### Polyglot Context

The execution environment created by the Java application. It may permit one, several, all installed, or dynamically selected guest languages.

### Language context

The language-specific runtime state initialized lazily inside a Polyglot Context.

### Static evidence

Evidence obtained without running application code: build files, resolved dependencies, class files, JAR contents, Native Image metadata, and bytecode calls.

### Runtime observation

Evidence collected while the user-selected test or application command executes with the JMend Truffle instrument enabled.

### Finding

An evidence-backed rule result with a stable rule ID, impact, confidence, location, explanation, and next action.

## 5. Delivery boundary

### MVP-A: useful static product

MVP-A must include:

- Java and Maven as the first fully supported project type;
- project, module, JDK, bytecode, dependency, and platform inventory;
- explicit target JDK and GraalVM assessment;
- Native Image evidence and an initial validated rule set;
- static Polyglot Engine and Context discovery;
- static Context configuration extraction;
- terminal output;
- canonical JSON report;
- static offline HTML report;
- stable findings and fingerprints;
- deterministic exit codes;
- validation against a representative project corpus.

### MVP-B: runtime verification

MVP-B follows after static findings are accurate:

- the verify command;
- a small Java Truffle instrument;
- observation of Context creation and close events;
- observation of language-context creation, initialization, failure, finalization, and disposal;
- observation of guest source loading and execution metadata;
- correlation of static Context sites with runtime observations where evidence permits;
- JVM verification results merged into the same AnalysisReport;
- Native runtime verification as a later increment inside MVP-B.

### Post-MVP integrations

- SARIF generation;
- NDJSON event generation;
- GitHub Action;
- baseline comparison;
- new, existing, and resolved finding states;
- scheduled JDK/GraalVM release checks;
- Gradle parity;
- dashboards through OpenSearch, Grafana, or Datadog.

## 6. Explicit non-goals

The initial product does not include:

- TUI;
- local web server;
- Tauri or another Rust GUI;
- language-specific package analysis;
- Python requirements analysis;
- npm dependency analysis;
- execution or orchestration of Polyglot workflows;
- automatic code modification;
- an OpenRewrite replacement;
- a Maven or Gradle plugin;
- an AI-generated readiness decision;
- mandatory configuration files;
- direct upload to external observability systems.

## 7. CLI contract

JMend has two product commands:

### inspect

    jmend inspect [path]

Performs static, read-only analysis. It does not compile the project, run tests, or modify files.

Examples:

    jmend inspect .

    jmend inspect . --native

    jmend inspect . \
      --target-jdk 25 \
      --target-graalvm 25.0.2 \
      --native

    jmend inspect . \
      --native \
      --report-dir target/jmend-report

### verify

    jmend verify [path] -- <user command>

Runs static inspection first, then executes the explicit user command with runtime observation enabled.

Examples:

    jmend verify . -- ./mvnw test

    jmend verify . \
      --report-dir target/jmend-report \
      -- ./mvnw verify

Future Native verification form:

    jmend verify . \
      --native \
      -- ./mvnw -Pnative nativeTest

JMend must not silently choose or run a test command in the first implementation of verify.

### Common options

    --target-jdk <version>
    --target-graalvm <version>
    --target-os <linux|macos|windows>
    --target-arch <x86_64|aarch64>
    --native
    --format <terminal|json|ndjson|sarif>
    --report-dir <path>
    --verbose
    --debug
    --debug-report <path>
    --fail-on <policy>

Default output format:

    terminal

The full report bundle is generated only when report-dir is supplied.

### Commands deliberately excluded

The MVP does not need separate:

    init
    analyze
    report
    open
    tui
    upload

## 8. Target selection

Without target options, JMend reports the detected current project and environment state.

With target options, JMend performs a what-if assessment:

    jmend inspect . \
      --target-jdk 25 \
      --target-graalvm 25.0.2 \
      --native

Meaning:

> Assess the current project as a candidate for JDK 25, GraalVM 25.0.2, and Native Image without first changing the project.

The report must distinguish detected values from requested target values.

## 9. Static project inspection

JMend must collect:

- project name and root;
- Maven wrapper and Maven version evidence;
- recursive modules;
- source, target, and release settings;
- detected and effective JDK requirements;
- current local JDK;
- OS and architecture;
- artifacts and JAR manifests;
- bytecode versions;
- resolved dependencies when safely available;
- relevant build plugins and options.

The first complete implementation targets Maven. Other build systems may be detected but must not claim equivalent assessment until their analyzers reach parity.

## 10. Native Image assessment

### Evidence inventory

JMend inspects:

- reflection APIs;
- dynamic class loading;
- resources;
- ServiceLoader and service descriptors;
- dynamic proxies;
- serialization;
- JNI;
- System.load and System.loadLibrary;
- packaged .so, .dll, and .dylib files;
- operating-system and architecture compatibility;
- Native Build Tools configuration;
- Native Image arguments;
- reachability metadata;
- META-INF/native-image content.

### Initial rule categories

- module or dependency bytecode newer than the selected JDK target;
- incompatible native library OS or architecture;
- dynamic reflection target that cannot be resolved;
- resource or service descriptor not proven reachable;
- JNI or dynamically loaded native library risk;
- metadata that does not apply to the resolved dependency version;
- conflicting or obsolete Native Image options;
- target GraalVM or component version mismatch.

### Required finding behavior

Each blocker or risk must include:

- stable rule ID;
- stable finding fingerprint;
- impact;
- confidence;
- exact subject;
- location when available;
- evidence;
- why it matters;
- next action;
- verification command when applicable.

JMend must not claim that a Native Image build will succeed solely because static inspection found no blocker.

## 11. Polyglot analysis model

### Central entity

The central entity is the Polyglot Context created by the application, not a language-specific advisor and not a dependency named after a guest language.

Internal relationship:

    Engine
      -> Context
          -> permitted languages
          -> referenced languages
          -> initialized language contexts

### Static discovery

JMend searches for:

    Context.create(...)
    Context.newBuilder(...)
    Engine.create(...)
    Engine.newBuilder(...)
    Source.create(...)
    Source.newBuilder(...)
    context.eval(...)
    context.parse(...)
    context.initialize(...)

For each Context site, JMend attempts to extract:

- creation location;
- explicit, all-installed, or dynamic permitted-language mode;
- associated explicit or implicit Engine;
- shared-engine usage;
- HostAccess;
- IOAccess;
- native access;
- process creation;
- environment access;
- thread creation;
- host class lookup and loading;
- Polyglot access;
- sandbox policy;
- options;
- language arguments;
- close behavior or try-with-resources evidence.

### Dynamic values

Example:

    Context.newBuilder(properties.getLanguage())

JMend reports:

    Permitted language mode: dynamic
    Evidence: properties.getLanguage()
    Static confidence: unresolved

It must not infer a concrete language without evidence.

### Generic rules

Rules are Context-based:

    context.language-id.dynamic
    context.language.not-available
    context.language.initialization-failed
    context.language.not-observed
    context.host-access.too-broad
    context.io-access.too-broad
    context.native-access.enabled
    context.process-access.enabled
    context.thread-access.enabled
    context.engine.shared-unsafely
    context.not-closed
    context.native.not-verified
    context.source.not-packaged

The same rule applies to any Truffle guest language.

There must not be modules such as:

    python_advisor
    javascript_advisor
    wasm_advisor

## 12. Runtime verification design

Runtime verification requires a small Java component because Rust cannot subscribe directly to JVM Truffle instrumentation events.

Responsibilities:

### Rust CLI

- inspect the project;
- prepare the verification run;
- start the explicit user command;
- collect the instrument result;
- correlate runtime and static evidence;
- evaluate rules;
- render outputs.

### Java Truffle instrument

- register as a discoverable Truffle instrument;
- observe Context creation and close;
- observe language-context creation;
- observe initialization success and failure;
- observe finalization and disposal;
- optionally observe source load and execution metadata;
- write a bounded structured trace;
- never capture guest source content by default.

### Verification semantics

If a static Context site is not exercised by the command:

    Static: detected
    Runtime: not observed

Not observed does not mean absent or broken.

If a language context fails to initialize:

    Static: referenced or permitted
    Runtime: initialization failed

This may produce a confirmed finding when the failure is reproducible and attributable.

## 13. Canonical AnalysisReport

All outputs are generated from one internal AnalysisReport.

Required top-level sections:

    schema_version
    report_id
    generated_at
    generator
    project
    environment
    target
    summary
    modules
    native_image
    polyglot
    findings

### Context example

    {
      "id": "context-01",
      "creation_site": {
        "module": "application",
        "class": "ScriptContextFactory",
        "method": "create",
        "line": 42
      },
      "discovery": {
        "static": true,
        "runtime": true
      },
      "configuration": {
        "engine": {
          "type": "shared",
          "creation_site": "PolyglotEngineConfig#create"
        },
        "permitted_languages": {
          "mode": "explicit",
          "ids": ["js", "python"]
        },
        "host_access": "scoped",
        "io_access": "none",
        "native_access": false,
        "process_creation": false,
        "thread_creation": false,
        "options": {
          "engine.WarnInterpreterOnly": "false"
        }
      },
      "static_lifecycle": {
        "close_detected": true
      },
      "runtime": {
        "jvm": {
          "status": "observed",
          "created": true,
          "closed": true,
          "languages": [
            {
              "id": "js",
              "created": true,
              "initialized": "passed",
              "sources_executed": 2
            }
          ]
        },
        "native": {
          "status": "not_run"
        }
      }
    }

### Finding model

    {
      "rule_id": "JMEND-NATIVE-004",
      "fingerprint": "sha256:...",
      "category": "native_image.resources",
      "impact": "blocker",
      "confidence": "confirmed",
      "title": "Service provider resource is not reachable",
      "location": {
        "module": "application",
        "path": "META-INF/services/org.example.EventProvider"
      },
      "evidence": [
        {
          "type": "missing_resource",
          "source": "bytecode_analysis",
          "value": "org.example.EventProvider"
        }
      ],
      "remediation": {
        "summary": "Register the provider through reachability metadata.",
        "verification_command": "jmend verify . --native"
      }
    }

### Finding dimensions

Impact:

    blocker
    high
    medium
    low

Confidence:

    confirmed
    likely
    possible
    unknown

Impact and confidence must remain independent.

### Fingerprint

A fingerprint is derived from stable evidence, for example:

    rule_id + module + artifact + location + subject

It must not be derived from the human-readable message.

## 14. Terminal output

Default:

    jmend inspect .

Example:

    JMend 0.2.0
    orders-service · Maven · Java 25

    Target
      JDK          25
      GraalVM      25.0.2
      Platform     linux-x86_64
      Native Image enabled

    Native Image
      Status       BLOCKED
      Blockers     1
      Risks        2
      Unknowns     1

    Polyglot
      Context sites detected   2

      Context #1
        Location       ScriptContextFactory#create:42
        Languages      js, python
        Host access    SCOPED
        IO access      NONE
        Lifecycle      close() detected

      Context #2
        Location       DynamicRunner#create:28
        Languages      dynamic
        Evidence       properties.getLanguage()
        Lifecycle      unresolved

    Findings
      BLOCKER  JMEND-NATIVE-004
               Service provider resource is not reachable

      RISK     JMEND-POLYGLOT-006
               Permitted language is resolved dynamically

    Result: NEEDS WORK

Verbose output adds complete evidence. Default output stays concise.

## 15. Report bundle

Command:

    jmend inspect . \
      --native \
      --report-dir target/jmend-report

Bundle:

    target/jmend-report/
    ├── index.html
    ├── report.json
    ├── findings.ndjson
    └── report.sarif

### index.html

- static and offline;
- opens directly from the filesystem;
- no server;
- no external CDN;
- no account;
- generated from the same AnalysisReport;
- must not contain source contents or secrets by default.

### report.json

- complete canonical model;
- versioned independently through schema_version;
- suitable for tooling and archival.

### findings.ndjson

- one flattened event per finding;
- an additional scan-summary event;
- suitable for transformation into OpenSearch Bulk requests;
- suitable for JSON log ingestion.

### report.sarif

- source-located findings;
- stable rule metadata;
- suitable for GitHub Code Scanning and IDE integrations.

## 16. HTML report UX

The report has four sections.

### Overview

- overall decision;
- project and build;
- current and target JDK/GraalVM;
- platform;
- Native and Polyglot status;
- highest-priority findings;
- versions are included here, not in a separate section.

### Native Image

- blockers;
- risks;
- unknowns;
- reachability metadata;
- native libraries;
- source and artifact evidence.

### Polyglot

- one entry per detected Context;
- creation site;
- Engine relationship;
- Context configuration;
- permitted, referenced, and initialized languages;
- JVM and Native observations;
- lifecycle;
- access policy;
- Context-specific findings.

The UI must not contain fixed Python, JavaScript, or Wasm panels.

### Findings

- all findings;
- filtering by impact, confidence, category, and state;
- location;
- evidence;
- why it matters;
- next action;
- verification command.

Direct links should be supported:

    index.html#native
    index.html#polyglot
    index.html#finding=JMEND-NATIVE-004

## 17. Exit codes

    0  Analysis completed and the selected policy passed.
    1  Analysis completed but the selected policy failed.
    2  Invalid arguments, unsupported input, or JMend failure.
    3  The command supplied to verify failed.

Local inspect must not fail only because findings exist unless fail-on is explicitly supplied.

Example CI policy:

    jmend inspect . \
      --native \
      --fail-on confirmed-blocker

## 18. Debugging

Verbose evidence:

    jmend inspect . --verbose

Internal diagnostics:

    jmend inspect . --debug

Shareable debug report:

    jmend inspect . \
      --debug \
      --debug-report jmend-debug.json

The debug report may include:

- analyzer durations;
- detected inputs;
- rules evaluated;
- rules skipped and reasons;
- evidence counts;
- non-sensitive errors.

It must not include credentials, environment secrets, or guest source content.

## 19. CI and GitHub Action

The GitHub Action is a thin wrapper over the released JMend CLI. It must not implement separate analysis behavior.

Target configuration:

    - name: JMend
      uses: ih0r-d/jmend@v1
      with:
        path: .
        target-jdk: "25"
        target-graalvm: "25.0.2"
        native: "true"
        fail-on: confirmed-blocker
        upload-report: "true"

Expected behavior:

- execute the same CLI available locally;
- write a concise pull-request summary;
- publish SARIF;
- add source annotations;
- upload the report bundle;
- fail only according to the explicit policy;
- support scheduled release-drift checks later.

The exact Action packaging is post-MVP and must be validated before the public contract is frozen.

## 20. External analytics

The canonical report remains vendor-neutral.

For analytics, JMend emits:

- one scan-summary event;
- one flattened event per finding.

Useful dashboard dimensions:

- project;
- branch;
- target JDK;
- target GraalVM;
- Native status;
- rule ID;
- category;
- impact;
- confidence;
- Context status;
- guest language ID when relevant;
- new, existing, or resolved state;
- scan duration.

Grafana should normally query OpenSearch or another supported data source. JMend does not upload directly in the MVP.

## 21. Configuration-file policy

There is no required .jmend.yml, jmend.toml, or equivalent file in the MVP.

Configuration is supplied through:

- detected project evidence;
- explicit CLI target options;
- the explicit command passed after verify --;
- CI Action inputs.

An optional project configuration file may be introduced only when repeated real use cases justify it, such as:

- suppressions with reasons and expiry;
- repository-wide policies;
- several named verification commands;
- custom module boundaries;
- persistent target matrices.

The format must not be selected before those use cases are validated.

## 22. Quality and validation

Before claiming general usefulness:

1. Build a validation corpus containing official GraalVM examples and real open-source Java projects.
2. Record the actual JVM and Native Image outcome.
3. Run JMend before the expensive build.
4. Compare predicted blockers, risks, and unknowns with observed failures.
5. Track false positives and unsupported claims.
6. Publish a reproducible validation matrix.

A confirmed blocker must be reproducible from its evidence.

If JMend cannot prove a condition, it must report risk, unknown, or not observed instead of blocker.

## 23. MVP acceptance criteria

MVP-A is acceptable when:

- inspect works without project configuration;
- the project is never modified;
- Maven modules and compiled artifacts are detected;
- current and target JDK/GraalVM values are distinguished;
- at least the initial Native Image rule set produces actionable evidence;
- Polyglot Context creation sites are detected from bytecode;
- explicit and dynamic permitted-language forms are distinguished;
- Context security and lifecycle configuration is represented where statically visible;
- terminal output is concise;
- report.json validates against a versioned schema;
- index.html opens offline and contains the four agreed sections;
- each finding has a stable rule ID and fingerprint;
- unknown conditions are not presented as confirmed;
- the validation corpus demonstrates real earlier detection of multiple failure classes.

MVP-B is acceptable when:

- verify runs only the explicit user command;
- Context creation and close are observed;
- language initialization success and failure are observed;
- static-only and runtime-observed Contexts are distinguished;
- source content is not captured by default;
- runtime results merge into the same report model;
- failed verification commands produce exit code 3;
- runtime instrumentation does not require application source changes.

## 24. Recommended implementation order

1. Freeze AnalysisReport and finding semantics.
2. Complete Maven dependency and artifact evidence.
3. Implement the initial Native Image rules.
4. Implement static Engine and Context discovery.
5. Implement Context configuration extraction.
6. Produce concise terminal output.
7. Produce report.json and its schema.
8. Produce the static HTML report.
9. Validate against the project corpus and reduce false positives.
10. Add the Java Truffle instrument.
11. Implement verify and runtime correlation.
12. Add NDJSON, SARIF, and the GitHub Action.

## 25. Open decisions

These remain intentionally unfrozen:

- the exact list of initial Native Image rules;
- the exact supported Maven dependency-resolution strategy;
- how static Context sites are correlated with observed TruffleContext instances;
- whether Native runtime verification belongs in the first verify release;
- the final GitHub Action packaging;
- the optional future configuration-file format;
- the public website and documentation structure.

## 26. Reference APIs and projects

- GraalVM Polyglot Context API: https://www.graalvm.org/sdk/javadoc/org/graalvm/polyglot/Context.html
- GraalVM Polyglot Engine API: https://www.graalvm.org/sdk/javadoc/org/graalvm/polyglot/Engine.html
- Truffle ContextsListener: https://www.graalvm.org/truffle/javadoc/com/oracle/truffle/api/instrumentation/ContextsListener.html
- Truffle Instrumenter: https://www.graalvm.org/truffle/javadoc/com/oracle/truffle/api/instrumentation/Instrumenter.html
- GraalVM Native Build Tools: https://github.com/graalvm/native-build-tools
- GraalVM Reachability Metadata: https://github.com/oracle/graalvm-reachability-metadata
- GraalVM Native Image diagnostics: https://www.graalvm.org/latest/reference-manual/native-image/debugging-and-diagnostics/

