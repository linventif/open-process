# Modular architecture migration

**Status: approved plan completed. This is a historical migration record.**

The [target](architecture-cible.md) is the design reference; actual paths and
interfaces are in the [current architecture](architecture-actuelle.md).

## Completed steps

| Step | Delivered result |
| --- | --- |
| 0 — Approve direction | Direction accepted after documentation review and authorization to execute the plan. |
| 1 — Baseline and separation | Initial tests recorded; five `.opx` v1 fixtures preserved; rendering, interaction and panels extracted. |
| 2 — UML pilot | Common typed session, one model/history and independent cameras for multiple views. |
| 3 — Catalogue | Injected static registry; module declarations drive home, menus, imports, exports and views. BPMN, Petri, UML and Mining use the contract. |
| 4 — Storage | Open v2 envelope, v1 reading, RON v1/v2 → JSON v3 session migration, unknown-file preservation and compatibility tests. |
| 5 — Transactions | Snapshot history, gesture cancellation, Mining history, content-based saved state, separate semantic/graphical revisions, shared layout. |
| 6 — Jobs | Native-thread/Web Worker Petri analysis, stale-input rejection, cancellation and cache reuse after geometry changes. |

The migration kept two crates and public `op-core` paths. Target directory names
were design guides; extraction preserved historical modules and existing local work.

## Extensibility proof

A test-only decision-table module registers its codec and canvas-free session
without shell changes. It exercises creation, factories, common documents and
saving. Duplicate IDs and ambiguous imports are also checked.

That proof itself did not add a visible notation. Subsequent product integrations,
including actual DMN tables, are documented in [the notation guide](notations-ajoutees.md).

## Verified scenarios

- BPMN, Petri and UML interactions, home, manual exercises and existing translations.
- Multiple views, independent cameras, common history and single-view closure.
- Escape cancellation of a drag without an extra history step.
- Undo to saved state and byte-for-byte recovery of unknown files.
- Migration and re-encoding of five v1 document families without domain data loss.
- RON v1/v2 sessions, historical Petri views, JSON sessions and malformed-session rejection.
- Analysis replacement after semantic changes and cache reuse after moves.
- Firefox test of the compiled worker: computation, cancellation, errors and
  main-thread activity during processing.

## Checks and limits

Commands and outcomes for that migration are in [the verification record](verification.md).
Local checks do not replace the Linux/Windows/macOS CI matrix.

Mining and process evaluation were not moved to background jobs; future migration
requires measurements and an identified need. `.opx` v2 is not promised to be
readable by older applications. The compatibility API `to_json(&Model)` intentionally
keeps v1 output.

Future extensions follow [the extension guide](ajouter-une-notation.md). Format
changes and removal of compatibility facades still require historical fixtures
and checks of their consumers.
