# JMend contributor guide

## Purpose and sources of truth

JMend is a local, evidence-based JVM and GraalVM readiness analyzer. The current implementation is a project-fingerprint and artifact-evidence foundation; Native Image readiness, Polyglot Context analysis, canonical reports, and runtime verification are planned unless code and tests prove otherwise.

Use these sources in order:

1. `docs/MVP.md` is the authoritative product specification and delivery order.
2. The implementation and tests define what works today.
3. `docs/03-architecture.md` defines current boundaries and evidence semantics.
4. `README.md`, `docs/02-mvp.md`, and `docs/04-roadmap.md` summarize implemented and planned scope.
5. `docs/05-classfile-parser-evaluation.md` records the class-file parser decision.

Do not turn roadmap text or representable model variants into support claims. Do not change the package version unless a version change is explicitly requested.

## Repository map

- `src/cli.rs`, `src/commands/`: CLI syntax and application operations. Implemented product commands are `check` and `inspect`; `verify` is planned.
- `src/project/`: project/build-unit evidence, Maven model inspection, Maven/Gradle detection and wrapper-first runtime metadata. Maven module and direct target inspection is implemented; Gradle multi-project parity is not.
- `src/runtime/`, `src/process.rs`: host/JDK evidence and injectable external-command execution.
- `src/artifact/`: scoped artifact discovery, bounded JAR/class-file parsing, normalized bytecode evidence, and declarative semantic catalogs.
- `src/analysis/`: analyzer interface and the current finding foundation. No production analyzer emits findings yet.
- `src/output/`: terminal presentation for complete command results.
- `tests/fixtures/`: checked-in Java sources and deterministic Java 8/17/21/25 class fixtures; ordinary tests do not compile fixtures or require a JDK.
- `docs/`: product, architecture, roadmap, use cases, and decisions.
- `.github/workflows/ci.yml`: repository CI and cross-platform snapshot packaging, not user-facing JMend analysis.
- `Cargo.toml`, `Taskfile.yml`: Rust package/dependencies and supported local tasks.

## Architecture boundaries

Keep the flow explicit:

`collectors/inspectors -> normalized evidence -> analyzers -> canonical results/findings -> renderers`

- Collectors gather facts and provenance. Build-tool adapters, runtime probes, artifact parsers, and semantic catalogs must not decide compatibility, severity, readiness, or remediation.
- Analyzers correlate evidence and produce conclusions. A finding must be traceable to collected evidence.
- Analyzer errors mean analysis could not complete and remain separate from findings. Findings are successful diagnostic conclusions about the inspected target; expected project defects are not application failures.
- Renderers display canonical results only. They must not detect conditions, assign diagnostic categories or severity, synthesize findings, fingerprints, readiness, or exit policy. Core analysis must never depend on presentation.
- Keep parser-specific and build-tool-specific types behind adapters; JMend-owned evidence crosses domain boundaries.
- Preserve project -> module -> artifact -> class/member/instruction provenance. Do not create competing flattened sources of truth.

## Product invariants

- Analysis is evidence-first and static inspection is read-only: do not build, test, or modify the inspected project implicitly.
- Prefer explicit `unknown`, `unresolved`, `not observed`, or partial-support states over guesses.
- Empty findings do not automatically mean success; absence of implemented analyzer coverage is not a passing readiness conclusion.
- Never produce readiness percentages.
- Rule IDs are stable. Finding fingerprints are deterministic and derived from stable evidence, never human-readable messages.
- Ordering and serialization are deterministic; use ordered collections or explicit stable sorting where output is observable.
- Canonical/user-facing paths are repository-relative where possible and retain module/artifact provenance.
- GraalVM Polyglot analysis is generic and centered on Engine/Context configuration and lifecycle. Do not add language-specific advisers such as Python, JavaScript, or Wasm advisers.
- No network, cloud service, account, telemetry, or source/guest-source upload is required or performed by default. Reports must avoid credentials, environment secrets, and source contents by default.
- AI may assist implementation, but must not generate canonical findings, readiness decisions, rule fingerprints, or exit codes. Those must come from deterministic, reviewed code and evidence.

## Development workflow

1. Start with `git status --short --branch`; inspect existing changes and do not overwrite unrelated work.
2. Read the relevant specification, architecture section, module, and adjacent tests before editing.
3. State whether the work implements current behavior or planned MVP behavior. Keep collection, analysis, and rendering changes separated by responsibility.
4. Make the smallest coherent change. Do not add dependencies or public/schema fields without demonstrated need and tests.
5. Add focused tests beside the module. Reuse checked-in fixtures; do not make normal tests depend on an installed JDK, Maven/Gradle, network, timing, or global machine state.
6. Run focused tests while iterating, then the full verification suite below.
7. Review the final diff for scope, deterministic output, repository-relative paths, secrets, and accidental support claims.

## Commands

Rust 2024 and a compatible stable toolchain are required. The repository supports these exact checks:

```bash
cargo fmt
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
task verify
```

`task verify` runs formatting, `cargo check`, Clippy with warnings denied, and tests. CI additionally exercises all targets/features, Cargo metadata, documentation, docs tests, builds, duplicate-dependency reporting, and release snapshot builds:

```bash
cargo check --all-targets --all-features
cargo metadata --no-deps --format-version 1
cargo test --all-targets --all-features
cargo test --doc --all-features
cargo build --all-targets --all-features
cargo doc --no-deps --all-features
cargo tree --duplicates
```

Build and run commands defined by `Taskfile.yml`:

```bash
task build
task build-release
task run -- [CLI_ARGS]
task run-check
cargo run --bin jmend -- --help
cargo run --bin jmend -- check [PROJECT]
cargo run --bin jmend -- inspect <PATH>
```

`check` defaults to the current directory and walks upward to the nearest Maven/Gradle root. `inspect` accepts an exact project/module directory, JAR, or class file. Do not document planned MVP flags or `verify` as implemented CLI behavior.

## Rust and test conventions

- Follow `rustfmt`; Clippy warnings are errors. Prefer standard-library types and small explicit modules.
- Domain values derive `Debug`, `Clone`, `PartialEq`, and `Eq` where useful. Keep visibility narrow (`pub(crate)` unless a public boundary is required).
- Use structured errors implementing `Display` and `Error`; preserve sources. Do not panic on user-controlled input.
- Inject `CommandExecutor` for command-dependent code. Unit tests use fakes rather than invoking real tools.
- Keep unit tests in `#[cfg(test)]` modules near the code. Test positive, missing, malformed, unsupported, and precedence cases.
- Temporary test directories must be uniquely named and cleaned up with RAII/`Drop`. Checked-in binary fixtures must retain matching source and documented provenance.
- Test observable output for deterministic ordering, bounded detail, absence of misleading placeholders, and ANSI-independent alignment.

## Definition of done

- Behavior is supported by evidence and does not overstate implemented coverage.
- Collection, analysis, errors, findings, exit policy, and rendering remain correctly separated.
- New conclusions have reviewed deterministic rules, stable IDs/fingerprints, provenance, and unknown handling.
- Observable paths, ordering, and serialization are stable and repository-relative where applicable.
- Focused tests cover success and relevant failure/unknown cases; fixtures remain reproducible and offline.
- `git diff --check`, `cargo fmt --check`, `cargo check`, Clippy with `-D warnings`, `cargo test`, and `task verify` pass.
- Documentation/help reflects only implemented CLI behavior; no secrets or unrelated changes are present.

## Review rules

Block changes that:

- mix collection policy with analysis, turn analyzer failures into findings, or put diagnostic logic in renderers;
- fabricate compatibility/readiness conclusions, infer certainty from missing evidence, or treat zero findings as proof of success;
- introduce unstable rule IDs, message-derived fingerprints, nondeterministic ordering/serialization, absolute machine paths, or an unversioned/breaking canonical schema change;
- create language-specific Polyglot advisers, implicit project execution, default network/cloud/upload behavior, or AI-authored canonical decisions;
- leak parser/build-tool types across domain boundaries, duplicate sources of truth, or claim planned support as implemented;
- change dependencies, versioning, generated fixtures, public contracts, or unrelated files without explicit scope and justification.
