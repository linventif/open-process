# Architecture decisions

**Status: decisions accepted after approval of the architecture plan.**

See the [target design](architecture-cible.md) for the rationale and
[current architecture](architecture-actuelle.md) for implementation paths.

## D1 — Keep one application and two crates initially

The existing domain/UI split supports incremental migration. Internal modules
provide initial boundaries without multiplying dependencies or file moves.
Notation-specific crates can follow an independent reuse or isolation need.
The tradeoff is relying on Rust visibility, reviews and tests for internal boundaries.

## D2 — Register modules statically

One composition point knows all shipped modules; the registry exposes their
catalogue. Adding a notation requires recompilation, acceptable for in-repository
extensions. Dynamic libraries would require an ABI, versioning and an additional
web strategy; that need has not been established.

## D3 — Keep notation-specific typed models

A universal node/edge model would move invariants into informal properties and
poorly represent logs or tables. Concrete models stay typed while geometry and
editing mechanisms can be shared. Specific validation, codecs and adapters are
an accepted maintenance cost.

## D4 — Use a common editor boundary

The central editor enum was replaced by a registry and trait-object sessions.
The shell no longer knows every variant. Models remain concrete and trait objects
are not serialized. Local enums, such as BPMN tools or UML relations, remain
appropriate for their domains.

## D5 — Declare commands/views and evaluate availability

A static capability list is insufficient: undo depends on history and simulation
can prohibit mutations. Descriptors announce commands/views; sessions compute
current availability, while the shell controls presentation. Explicit host
requests keep behavior easier to follow than an untyped global event bus.

## D6 — Keep snapshots and introduce transactions

Existing history provides a simple foundation. The first improvement is defining
action boundaries and tracking mutations rather than inverse commands per
operation. Snapshots use memory for large documents; optimize from representative
measurements.

## D7 — Separate the document envelope from domain content

Each document type has stable identity and a model version. Its codec owns its
migrations. The cost is compatibility adapters for older `.opx` files and sessions.
Compatibility tests must precede changing the writer. See
[document formats](documents-et-compatibilite.md).

## D8 — Decouple heavy computation from the UI after the pilot

Domain functions stay synchronous and testable; platform-specific services manage
execution. This avoids spreading async code throughout models. The Petri pilot
uses a native thread and a Web Worker loading the app's Wasm, without an additional
job library.

## Follow-up considerations

- Contracts are exercised by UML, BPMN, Petri and Mining.
- Refine text and matrix-cell transaction granularity as needed.
- View states are typed and separately persisted; command IDs are explicit.
- Extend browser background jobs to other analyses when justified.
- Measure history memory budgets on representative documents.

Further optimizations, including history deltas, remain measurement-driven.
No additional crate split is required by these decisions.
