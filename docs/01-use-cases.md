# JMend --- Use Cases

This document describes intended workflows. Currently, only project and JDK
detection through `jmend check` are implemented; the remaining analyzers,
interactive TUI, JSON output, and specialized commands are planned.

## UC-01 --- Inspect Project

A developer runs JMend from a JVM project directory.

``` bash
jmend
```

JMend detects the project and presents basic information:

-   project root;
-   Maven or Gradle;
-   Java target version;
-   modules;
-   packaging;
-   available diagnostics.

The interactive TUI becomes the main entry point for further
investigation.

## UC-02 --- Run Project Health Check

A developer wants a quick diagnostic scan without opening the TUI.

``` bash
jmend check
```

JMend analyzes the project and prints findings grouped by severity and
category.

Example:

``` text
ERROR
  1 incompatible bytecode finding

WARNING
  3 dependency conflicts
  7 duplicate classes

INFO
  2 observations
```

The command should return a meaningful exit code so it can later be used
in CI.

## UC-03 --- Find Duplicate Classes

A developer suspects that several JARs contain the same class.

JMend scans the effective classpath and reports duplicate class
definitions.

Example:

``` text
Duplicate class:
com.example.internal.Utils

Found in:
foo-core-1.4.jar
legacy-sdk-3.2.jar
```

The developer can inspect the affected JARs and dependency paths.

## UC-04 --- Find Duplicate Resources

JMend detects resources with the same path in multiple JARs.

Examples:

-   configuration resources;
-   metadata;
-   service descriptors;
-   framework resources.

The finding should show every artifact containing the resource.

## UC-05 --- Inspect ServiceLoader Providers

JMend scans `META-INF/services`.

It reports providers and detects suspicious collisions or duplicated
provider definitions.

The developer can inspect:

-   service interface;
-   provider implementations;
-   containing artifacts.

## UC-06 --- Explain Dependency Origin

A developer sees an unexpected dependency.

``` bash
jmend why jackson-databind
```

JMend shows why it exists in the resolved dependency graph.

Example:

``` text
payment-service
└── legacy-payment-sdk
    └── jackson-databind
```

If multiple paths exist, JMend shows each relevant path.

## UC-07 --- Diagnose Version Conflicts

JMend detects multiple requested versions of the same artifact and
explains which version is effectively resolved.

Example:

``` text
jackson-core

requested:
  2.18.4 via legacy-sdk
  2.20.0 via platform

resolved:
  2.20.0
```

The developer should be able to navigate from the finding to dependency
paths.

## UC-08 --- Inspect a JAR

A developer wants to inspect a JAR without extracting it manually.

``` bash
jmend inspect library.jar
```

JMend exposes:

-   manifest;
-   packages;
-   classes;
-   resources;
-   service providers;
-   multi-release entries;
-   class-file metadata.

The TUI may provide navigation through the JAR structure.

## UC-09 --- Detect Java Version Incompatibility

JMend compares:

-   project Java target;
-   local JDK;
-   build tool JDK;
-   class-file versions found in dependencies.

Example:

``` text
Project target: Java 21
Dependency foo-sdk contains Java 25 bytecode.

Potential UnsupportedClassVersionError.
```

## UC-10 --- Detect Potential Linkage Problems

JMend analyzes bytecode references against the effective runtime
classpath.

Example:

``` text
legacy-client.jar invokes:

Foo.connect(String)

Resolved Foo.class provides:

Foo.connect(String, Duration)

Potential NoSuchMethodError.
```

The finding includes:

-   caller;
-   expected symbol;
-   resolved class;
-   dependency origin;
-   evidence.

This is a post-MVP capability.

## UC-11 --- Explain a JVM Error

Future workflow:

``` bash
jmend explain stacktrace.txt
```

JMend recognizes supported JVM errors and correlates them with the
analyzed classpath.

Initial candidates:

-   `NoSuchMethodError`;
-   `NoSuchFieldError`;
-   `ClassNotFoundException`;
-   `NoClassDefFoundError`;
-   `UnsupportedClassVersionError`;
-   `ServiceConfigurationError`.

## UC-12 --- CI Check

A CI pipeline runs:

``` bash
jmend check
```

Future options:

``` bash
jmend check --format json
jmend check --fail-on error
```

The same analyzer results used by the TUI are exposed in
machine-readable form.

## UC-13 --- Spring Diagnostics

Future optional analyzer for Spring projects.

Potential capabilities:

-   detect Spring Boot project;
-   inspect configuration metadata;
-   detect unknown configuration properties;
-   inspect profiles;
-   identify suspicious configuration overrides.

Spring support is not required for the initial MVP.

## UC-14 --- GraalVM Diagnostics

Future optional analyzer for GraalVM projects.

Potential capabilities:

-   detect GraalVM Polyglot dependencies;
-   detect inconsistent GraalVM artifact versions;
-   inspect language dependencies;
-   identify common project configuration problems.

GraalVM support is not required for the initial MVP.

## UC-15 --- Compare Builds

Future workflow:

``` bash
jmend compare old.jar new.jar
```

Potential comparison:

-   dependencies;
-   classes;
-   resources;
-   public binary API;
-   Java version;
-   newly introduced findings.

## User Experience Principle

Every diagnostic should aim to provide three levels:

``` text
WHAT
WHY
EVIDENCE
```

Where useful, a fourth level can be provided:

``` text
NEXT STEP
```

JMend should prefer actionable diagnostics over raw data dumps.
