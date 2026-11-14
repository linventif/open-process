# Verification record

**Status: historical local validation records.** Results below describe the
checkout at each change, not a claim that all checks pass today. Test counts can
change between sections. Local Linux/browser checks do not replace the complete
Linux/Windows/macOS CI matrix. No deployment/publication is implied by a build.

## Initial architecture migration

The baseline had 71 passing tests. After migration, **90 tests passed**: 36 app,
49 core and five historical-document integration tests; none ignored.

### Commands executed

From the repository root:

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
git diff --check
```

The web artifact was also built from `crates/op-app/`:

```sh
CARGO_NET_OFFLINE=true trunk build --dist /tmp/open-process-migration-web --public-url /
```

It contained JavaScript glue/Wasm, job-transport snippets and `analysis-worker.js`.
The final native check had no warnings. Rust formatting was limited to migration
files, without formatting unrelated modules.

### Additional scenarios

- Five `.opx` v1 fixtures and migration/v2 round trips.
- Future versions, invalid envelopes and a codec outside the built-in catalogue.
- RON v1/v2 sessions, historical views and JSON v3 migration without deleting old keys.
- Two UML views sharing one model/history with independent cameras.
- Common BPMN/Petri/UML configuration in light/dark at 1,400, 420 and 280 px.
- Zoom, grid, issues, inspector alignment and compact menus.
- Independent grid/issue preferences across document views.
- Petri graph camera restoration and cache rebuilding.
- Undo to saved state, Escape cancellation and cancelled/failed saves.
- Byte-for-byte unknown-file recovery, including session serialization.
- A canvas-free table extension with multiple exports and no shell branch.
- Namespace-prefix-independent XML detection, ambiguous imports and duplicate IDs.
- Petri analysis replacement after semantic changes and reuse after movement.

Existing interaction, translation and native rendering tests were retained. UI
source inventories followed extracted submodules to retain translation/font coverage.

### Actual Web Worker test

[worker-smoke.html](../crates/op-app/tests/worker-smoke.html) exercises the Trunk
JavaScript transport and the app's Wasm in a worker. It checks a two-state workflow,
soundness, cancellation and a decoding error.

Headless Firefox ran the local build at `/` and `/app/`; both returned `ok: true`
with expected computation, cancellation and errors. A root run counted 63 main-thread
timer ticks during 658 ms of worker startup/processing. That includes Wasm loading
and initialization; it is not a large-model throughput benchmark. The `/app/` run
also checked resource-path resolution.

To reproduce, build with Trunk, copy the test page into the output, serve it over
HTTP and open `worker-smoke.html`. Results appear in the page and
`document.documentElement.dataset.result`. `?report=1` also sends to local `/result`
if supplied by the test server. The page is not automatically shipped in the app.

These checks covered local Linux/Firefox. Release builds, other browsers and
Windows/macOS remained CI/publication checks; no deployment or publication occurred.

<a id="zones-et-style-de-linterface"></a>

## Interface zones and style

After adding the [annotated SVG](diagrams/interface-zones.svg) and shared bar style,
the four root migration commands were rerun: **90 tests passed**, plus native and
WebAssembly checks. Common layout tests covered BPMN, Petri and both UML editors
at 1,400, 420 and 280 px. Light/dark Home/File/Style/Inspector hover screenshots
were generated under `target/screenshots/toolbar_hover_*.png`. Light File and dark
Style captures were visually inspected.

The SVG was parsed, its twelve zone groups and unique IDs checked, then rendered
with Rsvg and inspected. Local documentation links were checked. The website and
worker transport were unchanged and not revalidated for that visual change.

<a id="notations-dmn-epc-et-activite"></a>

## DMN, EPC and activity notations

The [initial integrations](notations-ajoutees.md) used the same four root migration
commands: **100 passing tests** (39 app, 56 core, five integration), none ignored;
native/WebAssembly checks passed without warnings. After the final palette icon
adjustment, the multi-view session test and both compilation checks were rerun.

Added/extended coverage included:

- EPC/activity connection/degree rules, deletion and duplication with remapped IDs.
- DMN UNIQUE/FIRST/ANY/COLLECT, interval boundaries, alternatives with commas in
  strings and unknown-expression rejection.
- JSON/DMN XML round trips, multi-model/aggregation rejection and SVG/DOT label escaping.
- Home creation of all three types and scrolling to their cards.
- Mouse creation/connections/movement, undo, reopening and exports.
- Table editing/evaluation/invalidation, column undo and registry reopening of DMN XML.
- Independent inspector preferences across views.
- Shared light/dark graphical configuration for EPC/activity at 1,400, 420 and 280 px.
- Translations and bundled-font glyph coverage.

Local `example_epc.png`, `example_uml.activity.png` and `example_dmn.table.png`
screenshots were inspected. SVG/local links were checked. This was neither OMG
certification nor exhaustive external-tool interoperability. No Looping/Bizagi
parity test, release build, publication or deployment ran. Competitor comparison
used the public sources in [the product analysis](parite-looping-bizagi.md).

<a id="base-de-donnees-et-ddl"></a>

## Database and DDL

The [relational module](base-de-donnees-ddl.md) was checked with:

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cargo run --locked --offline -p op-core --example database-ddl -- /tmp/open-process-ddl
python3 scripts/check-database-ddl.py /tmp/open-process-ddl
git diff --check
```

**107 tests passed** (41 app, 61 core, five integration), none ignored, and both
compilation checks passed without warnings. Tests covered dialects, identities,
SQL names, simple/composite keys, unique indexes, cycles, referential actions,
defaults, decimal precision and inconsistent-schema rejection. UI scenarios created
tables/links, moved without changing SQL, undid changes, switched dialects, reopened
models and displayed/copied SQL in a second panel. Layout coverage included
1,400, 420 and 280 px.

Python executed **three generated SQLite scripts** in memory: shop, composite
keys/cycles and literals. It checked table/key metadata, orphan/duplicate rejection,
NOT NULL, defaults and cascades. Quotes, backslashes and SQL characters in text
remained data.

Local `database_example_light.png`, `database_example_dark.png` and
`database_ddl_split.png` captures showed light/dark and split SQL views; light/split
were inspected. Documentation links/SVG were checked and [SQL examples](base-de-donnees-ddl.md)
regenerated. PostgreSQL/MySQL were not run on servers. No browser test, release
build, deployment or SQL-import trial was performed. Existing `.opx` compatibility
tests stayed green; no historical schema changed.

## Creation grid, exports and BPMN layout

Commands executed:

```sh
cargo fmt --all -- --check
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
git diff --check
```

**115 tests passed** (46 app, 64 core, five integration), none ignored; compilation
checks passed without warnings. Tests covered:

- File → New…, **+**, Ctrl/Cmd+N, search/no results, cancellation and originating
  tab groups, with the grid at 1,400, 420 and 280 px.
- File → Export differences for BPMN, DMN, EPC and databases.
- Wide/compact BPMN auto-arrange, preserved domain content/semantic revision and
  exact undo/redo snapshots.
- Layout non-overlap, pool/lane membership, cycles, disconnected components,
  annotations, determinism and atomic rejection of pool-header containment.

`new_document_grid_1400.png`, `new_document_grid_280.png`, `file_export_submenu.png`
and `bpmn_auto_arranged_1400.png` were inspected. Native interaction checks were
not browser tests; no new browser run, release build or deployment occurred.

## Petri place size

At this earlier revision the canvas radius changed from 11 to **5.5 model units**,
a 50% diameter reduction. Tokens followed the radius, and anchors, hit testing
and creation previews reused `PLACE_R`; labels kept their size. A later change
increased the radius to 8.25, as described in the [current interface guide](interface.md).

Formatting, local links, `git diff --check`, sequential workspace tests and
native/WebAssembly checks with `--locked --offline` passed: **123 tests** (50 app,
68 core, five integration), none ignored, no compilation warnings.
`mining_petri.png` was inspected. No new browser/release/deployment check; schemas
were unchanged.

## Spanish application and website

Commands executed:

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cd website
bun run build:site
CARGO_NET_OFFLINE=true bun run build:app
bun test
```

**123 Rust tests passed** (50 app, 68 core, five integration), none ignored; native,
Wasm and Trunk release web builds succeeded. Astro/TypeScript had no errors or
warnings and **nine Bun tests** passed on combined output.

At that revision, 860 Spanish translations were checked for coverage, uniqueness,
placeholders, line breaks and glyphs. Tests covered EN → FR → ES → EN, persisted
choices, historical serialized values and English fallback. `home_spanish.png`
was inspected. Site tests covered `/es/`, language links, canonical URLs,
`hreflang`, Open Graph, structured data and sitemap. Local links, touched Rust
formatting and diff checks passed. No new interactive browser run, native release
build, Windows/macOS validation, publication or deployment occurred.

<a id="dessin-libre"></a>

## Native and web free drawing

The [whiteboard](dessin-libre.md) used the five commands from the creation-grid
section: **123 tests passed** (50 app, 68 core, five integration), none ignored;
native/WebAssembly checks had no warnings.

Scenarios covered eight drawing tools via mouse, pencil points, multiline text,
escaped SVG and `.opx` reopening, movement/resizing/duplication/deletion, exact
gesture cancellation and history. Rubber-band selection/shared style editing
preserved distinct fills; a pencil click produced a point. Core tests checked
path direction, resizing, IDs, hit testing, invalid geometry and SVG XML validity.

Shared home/export/view-restoration/configuration tests included whiteboard at
1,400, 420 and 280 px, plus French/Spanish and glyph coverage.
`whiteboard_all_tools_light.png` and `whiteboard_example_dark.png` were inspected;
links/SVG were checked. The official Excalidraw component/format was not integrated.
Web compiled, but no new interactive browser run, release build or deployment occurred.

## Notation icons and shared selection

Modules declared card icons through `DiagramDescriptor::icon` and
`WorkflowDescriptor::icon`, independently of ordering/filtering. Whiteboard used
icon tools, and BPMN, Petri, UML, EPC/activity, databases and whiteboard shared
`icons.rs`'s BPMN selection cursor.

The five creation-grid commands passed: **123 tests** (50 app, 68 core, five
integration), none ignored; native/Wasm checks without warnings. Existing tests
covered tools, accessible names, selections and common controls.
`home_narrow.png`, `new_document_grid_1400.png` and `whiteboard_all_tools_light.png`
were inspected. The documentary SVG used the same selection shape; XML/links
passed. No new browser test, release build or deployment occurred.

## Icons in BPMN type menus

Task/Gateway type context menus, corresponding inspector lists and empty-space
creation used standard buttons with decorative icons and active highlighting.
Task markers/gateway vector shapes matched the canvas.

`bpmn_editing_by_mouse` was extended for both submenus, all labels, type changes
and exact undo/redo in light/dark. `bpmn_task_menu_dark.png` and
`bpmn_gateway_menu_dark.png` were inspected. The preceding five commands passed:
**123 tests** (50 app, 68 core, five integration), no ignored tests or compilation
warnings; formatting/links/diff passed. No schema change, browser run, release
build or deployment occurred for this change.

## Language dropdowns

The app button/site links became direct-choice menus for English, Français and
Español. Existing app preferences remained valid; HTML links worked without
JavaScript and identified the current language.

The Spanish-addition command set was rerun: **123 Rust tests** (50 app, 68 core,
five integration), **nine Bun tests**, native/Wasm checks and web release build
passed without Astro errors/warnings. Home tests checked opening without changing
language, three visible choices, direct selection and closure.
`language_dropdown.png` was inspected. HTML tests checked links/unique active
language on all three pages. Local links, UI-module formatting and diff passed.
Site Escape/outside-click closure was implemented but not exercised in a browser.
No deployment or Windows/macOS check occurred.

## Stable translation keys

At that revision, **859 messages** used stable IDs. English/French/Spanish
catalogues shared keys. A one-off comparison preserved **2,577 values** exactly,
including arguments/line breaks. A lexical inventory checked **1,057 literal calls**.

Commands executed:

```sh
cargo test --locked --offline -p op-core
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cargo fmt --all -- --check
git diff --check
cd website
CARGO_NET_OFFLINE=true bun run build:app
bun test
```

**124 Rust tests passed** (50 app, 69 core, five integration), none ignored;
native/Wasm checks, web release build and **nine Bun tests** succeeded. Touched
Rust files and workspace formatting passed.

A new core test verified that changing English copy preserves its key/French
translation and updates fallback. Catalogue checks covered identical/valid keys,
placeholders and line breaks. Usage/glyph checks included English and all three
languages. UI tests translated registry/palette keys before looking up labels.
Historical-document/session tests and local links passed.

Astro already used stable keys, so its catalogue/pages were unchanged; combined
output received the rebuilt app. No new browser run, native release build,
Windows/macOS validation, publication or deployment occurred.

## Vertical whiteboard palette

The palette moved to the left of the canvas, reusing `canvas::floating` with
vertical scrolling. Configuration, zoom and inspector remained in shared layout.
Three whiteboard UI scenarios passed, including rubber-band selection beside the
palette, creation, edits, movement/resizing, styles and undo. The completed-feature
snapshot passed **123 Rust tests** during commit preparation, independently of
the then-ongoing stable-key migration.

## Flags in language menus

English/Français/Español choices and the active button shared flags in app/site.
`language_dropdown.png` was inspected and home tests retained accessible-name and
language-selection checks. **124 Rust tests** passed (50 app, 69 core, five
integration), along with native/Wasm, formatting, Astro/web release and **nine Bun
tests**. HTML tests checked decorative flags/SVGs on all pages. Three SVGs were
parsed; 156 local documentation links and diff checks passed. No new interactive
browser run or Windows/macOS validation occurred.

## Examples on diagram cards

The card **Examples** menu in home/picker filtered by `ExampleDescriptor::kind`.
The registry rejected unknown types; a new test checked factories produced the
declared type and an openable document.

Interaction tests checked button placement at 1,400, 420 and 280 px, filtering,
no blank document on opening and Escape closing only the menu. Examples opened
from home and search in a split tab group, preserving the existing document.
`new_document_grid_1400.png`, `new_document_examples_280.png` and `home_french.png`
were inspected. The sequential offline suite passed **125 tests** (51 app, 69 core,
five integration); native/Wasm, workspace formatting, 156 links and diff passed.
Site unchanged; no new browser run or release build occurred.

## Resume cards and PNG previews

Four new scenarios covered:

- Distinct 320 × 180 previews for BPMN, Petri, DMN, database, whiteboard and logs,
  with unchanged models, updates after editing and resumption without duplication.
- Pixel-density-aware cropping and out-of-bounds regions.
- Rejection of delayed captures after screen changes.
- UID remapping, stale/corrupt cache rejection and session-schema independence.

Commands executed:

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cargo fmt --all -- --check
git diff --check
cd website
CARGO_NET_OFFLINE=true bun run build:app
bun test
```

Rust suites, **nine Bun tests**, native/Wasm checks and the web release build
passed. Cards were exercised at 1,400, 720 and 280 px; `home_resume_cards_1400.png`,
`home_resume_cards_280.png` and French rendering were inspected.

A one-off headless Firefox check of local `/app/` opened the BPMN example, displayed
its resume card and checked the stored `localStorage` entry's PNG signature.
After reload, the preview restored on home before reopening the document.
Browser screenshots before/after reload were inspected. Local links passed;
Windows/macOS and the complete CI matrix were not run for that change.

## English documentation and project language — 14 November 2026

The README was rewritten from the registered editor modules. Architecture,
notation, compatibility, website and repository-rule documentation now uses
English, as do interface-diagram labels and icon provenance. Existing document
paths and explicit legacy verification anchors were retained. The website's
README links to process discovery and running remain supported.

Inspection confirmed that `Lang::default()` and the initial atomic language are
English, that loading without a persisted preference uses that default, and that
the website's `/` route is English. No runtime language behavior or translation
catalogue was changed. Existing saved French/Spanish preferences remain supported.

Commands actually executed for the language check:

```sh
cargo test --locked --offline -p op-core i18n
cargo test --locked --offline -p op-app i18n -- --test-threads=1
git diff --check
```

All **five core i18n tests and three application i18n tests passed**, including
catalogue consistency, English fallback, saved-language restoration and startup
without a saved preference. Local Markdown links/anchors and issue-form YAML were
checked. The translated SVG was parsed for XML validity and twelve unique zone
IDs, rasterized with Rsvg and visually inspected; one label was spaced farther
from its neighbor after inspection.

This documentation change did not run the full Rust suite, native/WebAssembly
compilation checks, website builds, interactive browser checks or the CI matrix.
Historical results above were translated, not rerun or reclassified as current results.

## Mermaid editor and preview

The `mermaid` document module adds source editing, five templates, background
preview rendering and Mermaid/SVG exports through the notation registry.

Eight new tests cover SVG rendering, header detection, Mermaid/log import
separation, invalid drafts, `.opx` compatibility, history, independent views,
stale jobs, error recovery and responsive creation at 1,400, 420 and 280 pixels.
The native screenshots at 1,400 and 280 pixels were inspected.

Checks executed:

```sh
cargo test --locked --offline --workspace -- --test-threads=1
cargo test --locked --offline -p op-core mermaid
cargo test --locked --offline -p op-app mermaid_editor -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cargo fmt --all -- --check
git diff --check
cd website
CARGO_NET_OFFLINE=true bun run build:app
cd ../crates/op-app
trunk build --release --locked --public-url / --dist /tmp/open-process-mermaid-root
```

The workspace suite passes: **61 application tests, 72 core tests and five document
integration tests**. Native/Wasm checks, formatting and patch checks pass. The
Mermaid-specific tests also pass after the final header-detection additions.

The actual Trunk release builds were served locally under `/app/` and at `/`.
Headless Firefox checks exercised the shared worker for flowchart, sequence,
class, state, ER and Gantt inputs on both paths, checking SVG output, PNG
signatures and syntax-error responses. The final `mermaid-svg` engine passes;
the first engine candidate compiled but failed at runtime because it called
`std::time::Instant` during layout. Gantt's today marker is disabled in derived
rendering to avoid a system-clock call; source documents remain intact.

Local documentation links passed. Windows/macOS and the complete CI matrix were
not run for this extension; no website-content build or new Bun test run is
claimed. See [the Mermaid guide](mermaid.md) for rendering limitations.

## UML class ↔ Mermaid conversions

The UML and Mermaid modules declare conversion commands in their own adapters.
Toolbar buttons and the existing Tools menu open a separate editable destination;
no shell catalogue, dependency, file-schema change or new worker was added.

Six core tests cover the built-in class example, all seven relation kinds,
classifier kinds, duplicate/escaped names, Unicode and reserved text, reversed
markers and endpoint multiplicities, interface inference, loss diagnostics,
invalid/cyclic relations, incomplete sources, deterministic layout and empty
models. Two application tests exercise both directions through the real buttons,
Tools commands, independent history, dirty state, source preservation, unsupported
source kinds and rejection without opening a partial document. Wide and narrow
screenshots at 1,400 and 420 pixels were inspected.

Commands executed:

```sh
cargo test --locked --offline -p op-core mermaid::conversion -- --test-threads=1
cargo test --locked --offline -p op-app conversion -- --test-threads=1
cargo test --locked --offline --workspace -- --test-threads=1
cargo check --locked --offline -p op-app
cargo check --locked --offline -p op-app --target wasm32-unknown-unknown
cargo fmt --all -- --check
git diff --check
cd website
CARGO_NET_OFFLINE=true bun run build:app
```

The workspace suite passes: **63 application tests, 78 core tests and five document
integration tests**. Native/WebAssembly checks, the Trunk release build, formatting,
patch checks and local documentation links pass. This change did not run a new
interactive browser test, a website-content build, Bun tests, Windows/macOS or the
complete CI matrix. See [conversion behavior and limits](mermaid.md#converting-uml-class-diagrams).


## Reliability and security hardening — 14 November 2026

The local patch based on `96c86bb` strengthens existing imports, analysis and
saving without changing the `.opx` v2 envelope, model versions or v1 fixtures.
BPMN/Petri/UML checked codecs validate storage before editor creation. Saving
rechecks storage and encoded size; unsupported originals remain recoverable.
Native saving uses the native-only `tempfile` dependency (already present in the
lockfile transitively) for portable staging/replacement, with sync and failure
regressions rather than a handwritten Windows replacement implementation.

Targeted regressions cover deep/multiple-root XML, unsafe identities/geometry/
endpoints, near-exhausted counters, combined PNML arcs, truncated Petri deadlocks,
large invariant coefficients, cancellation, interrupted saves, unconfirmed
browser downloads and preserved original bytes. Generated tests exercise 600 CSV
round trips with quoting/Unicode/control characters and 1,000 bounded mutated
inputs through XML, document and log parsers. These complement the unit suite;
they are not an exhaustive fuzzing campaign or a standards-conformance audit.

Final local Rust results: **73 application tests, 87 core tests, seven document
integration tests and two generated-case tests passed (169 total)**. Historical
fixtures are unchanged. Formatting, strict Clippy, native and Wasm compilation,
and Linux/native and Trunk/Wasm release builds pass.

Commands executed for these checks:

```sh
cargo test --locked --workspace -- --test-threads=1
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo check --locked -p op-app --target wasm32-unknown-unknown
cargo build --locked --release -p op-app
cd website
bun run build
bun run build:app
bun test
cd ..
python3 -m unittest discover -s scripts -p 'test_*.py'
cargo run --locked -p op-core --example database-ddl -- /tmp/open-process-hardening-ddl
python3 scripts/check-database-ddl.py /tmp/open-process-hardening-ddl
python3 scripts/check-web-worker.py
```

The site builds and **nine Bun tests** pass. The **two Python packaging tests**
pass, and three generated SQLite scripts execute with metadata/constraint checks
(`shop`, `literal-defaults`, `composite-cycle`). PostgreSQL/MySQL server execution
was not performed. Native screenshots for discovery and BPMN evaluation were
inspected after the asynchronous changes.

Headless Firefox exercises all four job types (Petri, Mermaid, mining and BPMN
evaluation), invalid inputs/tasks and cancellation under the shipped application
CSP. A standalone root Trunk build also exercises the same checks:

```sh
cd crates/op-app
trunk build --release --locked --public-url / --dist /tmp/open-process-hardening-root
cd ../..
python3 scripts/check-web-worker.py --dist /tmp/open-process-hardening-root --app-prefix /
```

Both `/app/` and `/` checks pass. The local Snap launcher required using its
direct Firefox executable and a temporary profile under ignored `target/`;
the helper now detects that launcher. Earlier failed browser attempts are not
counted as successful runs. The final runs observed 17–19 main-thread timer
ticks during the small Petri request (174–194 ms); these are observations, not
latency budgets or mobile/browser-matrix guarantees.

Online `cargo audit --no-yanked` used the RustSec database at commit `4ef2df8e`
(13 November 2026): **zero reported vulnerabilities and five unmaintained-package
warnings** (`rustybuzz`, `ttf-parser`, `zbus_names`, `zvariant`, `zvariant_derive`).
Online `bun audit --json` returned no advisories. Neither result certifies that
application code is vulnerability-free or resolves the maintenance warnings.

Workflow YAML and immutable action references were checked. The release-tag
script was exercised for `main`, a stable tag and a prerelease tag, and generated
`SHA256SUMS` was verified with `sha256sum --check`. CI now runs dependency audits,
format/lint checks and generated SQLite execution; dependency audits also run
weekly. The new workflows were not executed on GitHub or used to publish a
release during this change. Local Markdown targets and `git diff --check` pass.
Windows/macOS builds, the complete CI matrix and production hosting settings
were not verified locally.

## Cloudflare-only deployment clarification — 14 November 2026

The repository has no GitHub Pages deployment workflow, Pages write permission
or Pages deployment action. The website CI job retains combined-build checks and
an ordinary artifact upload; its display name now states that purpose. The old
GitHub Pages instruction in `Trunk.toml` was replaced with the Cloudflare assembly
entry point, and the website guide documents disabling the former publishing
source in repository settings.

Workflow YAML, Trunk TOML, local documentation links and patch whitespace were
checked. No runtime behavior changed, so builds and application tests were not
repeated. The connected GitHub tool does not expose Pages administration; the
remote publishing setting was not changed or verified during this clarification.

## Creation-card order — 14 November 2026

Module-owned `home_order` values now place free drawing, relational database
design, UML classes, BPMN and UML use cases first on the home screen and in the
new-document picker. Remaining cards preserve their relative order. Existing
responsive UI checks were adjusted to the new first cards and scroll to BPMN
before testing its example menu on narrow screens.

All 169 workspace tests passed with `--test-threads=1`. Native and WebAssembly
`cargo check --locked -p op-app` checks, formatting, local documentation links
and `git diff --check` passed. The generated home-screen screenshot was inspected.
No new release build, deployment or Windows/macOS run was performed for this
presentation change.

## Closing empty documents — 14 November 2026

The tab close button and File menu now share the same close policy. Each typed
backend reports whether content remains; empty diagrams, whitespace-only source
or logs and default decision tables close directly, including renamed documents
and saved diagrams cleared by deletion. Authored configuration, exercise answers
and recoverable originals retain the conservative confirmation behavior when
unsaved. The dirty indicator and saved-file reference are unchanged.

All **172 workspace tests** passed with `--test-threads=1` (76 application, 87 core,
seven document and two generated-case tests). New regressions cover all empty
editors through both close entry points, nonempty examples, customized clauses,
pool-only BPMN, invalid source, original data and deletion with undo available.
Native/WebAssembly checks, strict Clippy, formatting, local documentation links
and `git diff --check` passed. Release builds, new browser runs, Windows/macOS
and deployment were not repeated for this change.

## Automatic local document library — 14 November 2026

Desktop builds resolve the OS Documents folder, create Open Process and write
UUID-named `.opx` files with metadata sidecars. Browser builds use one
`open-process-document-<uuid>` local-storage record per diagram. The home screen
reopens closed saved diagrams. Manual destinations/downloads remain explicit;
automatic saving never writes an imported or manually chosen source path.
The document envelope/model versions and historical `.opx` fixtures are unchanged.

The native single-writer queue acknowledges exact submitted snapshots and close/
exit barriers finish pending writes. Browser page-hide/unload handlers flush
automatic records and workspace state. Failed or stale writes retain pending
content. Workspace v4 reads v1/v2/v3 without deleting old keys and references
confirmed copies instead of duplicating browser payloads. Pending workspace edits
whose record advanced elsewhere are retained as a separate automatic copy.

The final workspace suite passes: **82 application tests, 87 core tests, seven
document integration tests and two generated-case tests (178 total)**. New
regressions cover in-flight edits followed by close, manual-file isolation,
home-library reopening/restart, failed destination/retry, same-name identities,
stale revisions, original JSON bytes, compact/mixed workspace ordering, missing
references and pending-edit preservation after another writer advances. Storage
tests use temporary directories and do not create a library in the tester's real
Documents folder. The library screenshot was inspected during development.

Native Linux and Trunk/WebAssembly release builds, native/Wasm checks, strict
Clippy, formatting, local documentation links and `git diff --check` pass. Nine
Bun build/output tests and two Python packaging tests pass; the browser helper
also passes Python compilation. The website's content sources did not change,
so a new Astro content build was not needed.

Headless Firefox checks the final `/app/` artifacts under the shipped CSP:

```sh
python3 scripts/check-web-worker.py --smoke autosave --timeout 70
python3 scripts/check-web-worker.py
```

The autosave scenario opens a library diagram in the real app, deletes its
content, flushes on page hiding, simulates `QuotaExceededError`, checks the prior
record and pending payload, retries successfully and reopens after reload. The
worker scenario checks all four tasks, cancellation and malformed inputs.

An offline Rust dependency audit against the previously fetched RustSec commit
`4ef2df8e` reports zero vulnerabilities and the same five maintenance warnings;
this is not a fresh online database check. Windows/macOS, abrupt power-loss
durability, simultaneous-writer transactional merging, the remote CI matrix and
production deployment were not validated locally. Browser quota/clearing and
unconfirmed changes during abrupt termination remain practical storage limits.

## Attached connectors and preferred ports — 14 November 2026

Free-drawing lines/arrows now have typed start/end attachments to shape IDs and
preferred ports. Moving/resizing shapes updates resolved paths; endpoint handles
retarget or detach ends. Group movement preserves attachments, duplication
remaps copied shape IDs, and deletion freezes detached ends at their last
positions. Canvas rendering, hit/selection bounds, fit and SVG share indexed
geometry. Unused identities remain allocatable when drawing counters approach
their limit.

Whiteboard writes model v2 and reads v1 with original independent path geometry.
The new `whiteboard-v1.opx` compatibility fixture complements unchanged historical
fixtures. Invalid/dangling/path-target attachments and future model versions are
rejected. Other model/envelope schemas are unchanged. Shared `geom::Port` supplies
rectangle side midpoints and diamond vertices to graphical editors; Petri
reciprocal paths retain distinct routes with centered transition endpoints.

The complete workspace suite passes: **84 application tests, 91 core tests,
seven document integration tests and two generated-case tests (184 total)**.
The new mouse regression creates a bound arrow, moves/resizes its source,
undoes/redoes, retargets/detaches its endpoint and saves/reopens the drawing.
Domain regressions cover resolved hits/SVG/bounds, duplication/deletion/group
movement, identity exhaustion, legacy/future codecs and preferred BPMN ports.
Petri checks cover reciprocal straight/orthogonal/curved arcs after movement.
The generated attached-drawing screenshot was visually inspected.

Strict Clippy, formatting, native/Wasm compilation checks, Linux native and
Trunk/Wasm release builds, nine Bun output tests, local documentation links and
`git diff --check` pass. Firefox exercises the shipped `/app/` artifacts under
the app CSP: a v2 drawing retains both arrow attachments through deletion,
undo, a simulated quota failure, retry and reload. All four background-job
smoke checks also pass. The translation inventory check initially caught an
out-of-line test declaration hiding production text from its scanner; placing
the declaration after production code restored full coverage.

Windows/macOS, mobile/touch interaction, the remote CI matrix and deployment
were not run locally. Legacy v1 paths remain independent until their endpoint
handles are used to attach them. Diagram routing is not a general obstacle-
avoidance or automatic diagram-layout guarantee.
