# Working on Open Process

## Documentation and architecture status

- Start with [docs/README.md](docs/README.md).
- The [current architecture](docs/architecture-actuelle.md) describes the implementation.
- The [target architecture](docs/architecture-cible.md) and [decisions](docs/decisions.md)
  are approved. The [migration record](docs/migration.md) lists completed steps.
- Follow the [extension guide](docs/ajouter-une-notation.md) when adding a notation.
- New notations declare their factories, commands, views, imports and exports in
  their own module, then register in `composition.rs`. The shell consumes the
  injected registry; do not reintroduce an enum-based editor catalogue.
- Graphical editors reuse `op-app/src/editor/layout.rs` for the configuration bar,
  inspector, inspector tabs and canvas. Do not copy margins, heights, responsive
  thresholds or zoom widgets into each notation. Modules provide domain options
  and content through the available slots.
- Keep models and histories typed. View state must not duplicate the document.
  Existing `Any` probes are reserved for UI tests.
- Distinguish implemented behavior, proposals and checks actually performed in
  documentation. Update the relevant pages after a change.

## Project language

- English is the default language for documentation, new source comments and
  original interface text. Write new documentation and updates in English.
- English is also the application's first-run and fallback language and the
  website's root language. Preserve explicit saved language preferences.
- Keep French and Spanish translations and localized website routes. Use the
  existing translation mechanisms rather than hardcoding interface text.
- Community issues and pull requests may also be written in French.
- Existing documentation filenames are retained to preserve links.

## Current structure

- `crates/op-core/`: typed models, domain rules, analyses, conversions,
  import/export and geometry; independent of egui and eframe.
- `crates/op-app/`: the egui/eframe application, notation editors, home screen,
  docking, history, files and native/web persistence.
- `website/`: Astro/TypeScript/Bun presentation site and assembly with the
  WebAssembly application under `/app/`. See [its README](website/README.md).
- Rust versions and dependencies: workspace and crate `Cargo.toml` files.

## Contribution rules

- Inspect `git status` before editing and preserve existing work, including
  untracked files. Limit changes to the requested scope.
- Keep domain rules and algorithms in `op-core`, without dependencies on the UI,
  system dialogs or browser storage.
- Preserve notation-specific types and invariants. Share a mechanism when
  multiple concrete uses justify its interface.
- Preserve `.opx` data, existing imports/exports and session restoration.
  Document and test schema changes against older files.
- Consider both native and WebAssembly effects; isolate platform-specific APIs.
  Any future long browser computation must yield control to the UI.
- Do not add a dependency or extract a crate just to move files. Explain the
  boundary or need that the change addresses.

## Checks by change type

Run relevant checks and report any that could not be run. For documentation-only
changes, check local links and diffs; a full build is unnecessary.

```sh
# Models, rules, formats and algorithms
cargo test --locked -p op-core

# Application and UI tests: sequential execution, as in CI
cargo test --locked --workspace -- --test-threads=1

# Native compilation and web target check, if installed
cargo check --locked -p op-app
cargo check --locked -p op-app --target wasm32-unknown-unknown

# Whitespace and patch errors
git diff --check
```

For the website, use the commands in [website/README.md](website/README.md).
[CI](.github/workflows/ci.yml) runs tests on three operating systems, native
release builds and the site build with the web application. Do not present a
partial local check as success across that entire matrix.
