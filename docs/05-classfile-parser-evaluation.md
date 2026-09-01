# Class-file parser evaluation

This decision record supports issue #11. JMend needs full structural and JVM
instruction parsing for Java 17 through Java 25, while keeping parser-specific
types behind a JMend-owned adapter.

## Decision

Use `ristretto_classfile` 0.33.0. A small JMend header reader validates
`CAFEBABE` and preserves raw minor/major versions before delegation. Full
custom ClassFile parsing was rejected.

The selected crate explicitly accepts major versions 45 through 69, models all
constant-pool tags, fields, methods, decoded `Code` instructions, annotations,
signatures, exceptions, inner/nest metadata, records, permitted subclasses,
bootstrap methods, invokedynamic, method handles/types, and module attributes.
Unknown attributes are retained as opaque data. Its repository is active, has
fuzzing infrastructure, and the crate is part of a JVM implementation rather
than an isolated header reader.

JMend verified actual classes emitted by Oracle JDK 17, 21, and 25. For every
version the parser resolved class identity, superclass, fields, methods, Code,
invocation instructions, record/sealed metadata, annotations, lambdas and
`module-info`. Deterministic outputs from those compilers are checked in as
test fixtures, so normal tests require no installed JDK.

## Candidates

| Candidate | Result | Relevant evidence and limitation |
|---|---|---|
| `ristretto_classfile` 0.33.0 | Selected | Explicit historical versions from major 45 through Java 25/major 69; decoded instructions and modern typed attributes; unknown attributes preserved. Parser caps structural parsing at major 69, handled by JMend's header/partial state. |
| `cafebabe` 0.9.0 | Rejected | Rich resolved API, instructions, and Chapter 4 attributes; it has no upper-major rejection and many 22-25 classes may parse. Its documented/tested contract stops at Java 21, so Java 25 correctness is not assured. |
| `jclassfile` 0.6.1 | Rejected | Active Java 25 Chapter 4 parser with modern attributes, but `Code` remains raw bytes and constant-pool indexes require another layer; insufficient alone for invocation/access evidence. |
| `jvmti-bindings` 3.0.2 | Rejected | Defensive limits, unknown attributes, and broad modern attributes, but its class parser retains raw `Code` bytes and the enclosing JVMTI/JNI crate is broader than JMend needs. |
| `lvm_class` 0.7.2 | Rejected | Decoded instructions and Java 25 claim, but inspected code still contains incomplete constant-pool/module paths and rejects unknown attributes; unsafe for forward-compatible evidence collection. |

## Boundaries and limitations

The adapter converts parser values to JMend domain types in `artifact::classfile`.
No parser type appears in `Project`, `ProjectModule`, `ArtifactEvidence`, CLI,
or findings. This preserves replaceability. The adapter emits generic member,
instruction, constant, reference, and provenance facts only. JVM/JDK API and
namespace recognition belongs to a separate declarative semantic catalog, so
new GraalVM, Native Image, Polyglot, or framework knowledge does not modify the
low-level parser.

For major versions above 69, JMend records the raw header and marks structural
analysis unsupported. It does not fabricate Java versions or semantic
evidence. Input and archive-entry limits supplement the parser because its API
does not expose configurable cumulative-allocation limits. Unknown attributes
are not interpreted until they have diagnostic value.
