# Changelog

This file records curated changes for users. Published builds and their generated
release notes are available on [GitHub Releases](https://github.com/linventif/open-process/releases).
Use the release tag or commit to identify a build; Cargo's package version alone
does not distinguish the automatic `main-*` releases.

## Unreleased

### Editing

- Attach free-drawing lines/arrows to shapes, keep endpoints following movement
  and resizing, and retarget them with endpoint handles. Whiteboard model v2
  reads v1 drawings and saves typed attachments with undo/redo and SVG support.
- Prefer rectangle side midpoints and diamond vertices across graphical editors;
  show shared connection guides and snapped previews when linking elements.
- Automatically save desktop documents in Documents/Open Process and browser
  documents in local storage; reopen closed diagrams from the home library.
- Keep manual Save/Save as copies independent, report automatic-save failures,
  and migrate older workspaces without removing their original storage keys.
- Close empty documents directly from their tabs or the File menu while retaining
  save confirmation for unsaved content and recoverable originals.

### Reliability and security

- Bound XML imports and reject unsafe BPMN/Petri/UML payloads while preserving
  `.opx` recovery and historical fixtures.
- Combine repeated PNML arcs, preserve quoted CSV data, and report malformed rows.
- Correct Petri deadlocks at exploration limits and protect invariant arithmetic.
- Stage native saves before replacement; distinguish browser downloads from
  confirmed writes and retain their unsaved indicator.
- Run discovery and BPMN evaluation outside the UI; bound worker concurrency,
  transfers, exploration and history, and avoid idle-frame JSON reconstruction.
- Add generated import regressions, SQLite execution and dependency audits;
  pin CI actions and publish development builds as prereleases with checksums.
- Show build identity in About and retain calculation-specific language settings.

### Documentation and community

- Rewrote the README around registered editors, current formats and known limits.
- Made English the default project documentation language, translated the
  architecture/notation/website guides and interface diagram, and documented
  the existing English first-run/fallback behavior while keeping French/Spanish.

- Added contribution and support guides, project governance, a code of conduct
  and a security reporting policy.
- Added GitHub bug and feature request forms and a pull request template.
- Added getting started and data/storage guides, and linked community resources
  from the README and documentation index.
- Clarified the README roadmap to distinguish existing features from remaining work.

## Earlier builds

See [GitHub Releases](https://github.com/linventif/open-process/releases) for the
existing release history. This changelog starts with the community documentation
changes; it does not reconstruct past release dates or validation results.

For future releases, record user-visible changes, known limitations and any
file/session compatibility changes here. Keep unreleased work under **Unreleased**
until the corresponding release is published.
