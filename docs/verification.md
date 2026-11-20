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

## Dependency and build-tool refresh — 14 November 2026

The crates.io and npm registries were queried for every direct dependency,
including platform-specific and test dependencies: **20 Rust crates and five
website packages**. After updating resvg/usvg from 0.47 to 0.50, all direct Rust
dependencies resolve to their latest stable releases checked that day. The
lockfile refresh also moves wgpu/naga to 30.0.2; 25 package names have changed
version sets or been added/removed. A final online `cargo update --dry-run
--verbose` proposes zero updates within the declared constraints and Rust 1.95
minimum. Local Rust checks used rustc 1.98.0; the minimum compiler itself was not
tested. No document schemas or historical fixtures changed.

| Component | Selected version | Result |
| --- | --- | --- |
| Astro | 7.4.12 | Updated |
| `@astrojs/sitemap` | 3.7.5 | Updated |
| `@astrojs/check` | 0.9.12 | Already latest |
| `@types/bun` | 1.4.4 | Already latest |
| TypeScript | 6.0.3 | Latest compatible 6.x; 7.0.2 excluded by checker peer range |
| Bun | 1.4.4 | Updated package manager and CI |
| Node.js | 26.13.1 LTS | Updated `.node-version`, documentation and explicit CI selection |
| Python | 3.15.2 | Updated CI/release packaging runtime |
| cargo-audit | 0.23.0 | Updated security workflow |
| Trunk / wasm-bindgen CLI | 0.22.1 / 0.2.130 | Already latest stable |

The checker declares `typescript: ^5.0.0 || ^6.0.0` in its
[published manifest](https://registry.npmjs.org/@astrojs/check/0.9.12).
`bun outdated --no-cache` reports only the unsupported TypeScript major after
the refresh. The Bun lockfile also updates compatible transitive packages.

GitHub action release tags were checked upstream and dereferenced to immutable
commit pins. Checkout, Python setup, artifact upload/download and release actions
were upgraded; Node setup now reads the declared version file. Workflow inputs
were checked against the pinned action metadata, with archived artifact uploads
explicitly retained. Bun dependencies join Cargo and actions in weekly Dependabot
checks. Cloudflare runtime recommendations were updated in the website guide;
remote Cloudflare environment variables were not inspected or changed.

The final checks pass: **184 sequential workspace tests** (84 application, 91
core, seven document integration and two generated-case tests), native and Wasm
`cargo check --locked`, strict Clippy, formatting, a Linux release build, the
complete Astro/Trunk release build and **nine Bun output tests**. The site checks
used Bun 1.4.4 and Node 26.13.1. **Two Python packaging tests**, Python compilation
of the browser helper and **three generated SQLite DDL cases** pass with Python
3.15.2. These runtime tools were installed in isolated validation directories;
the system-wide installations were not replaced.

Headless Firefox passes against the final `/app/` artifacts and shipped CSP:
automatic saving, deletion, quota failure/retry, drawing attachments and reload,
plus all four background-job types, cancellation and invalid requests. Local
Markdown targets, workflow YAML/action inputs and `git diff --check` pass.

A fresh online cargo-audit 0.23.0 run against RustSec commit `4ef2df8e`
(13 November 2026) reports **zero vulnerabilities and four unmaintained-package
warnings**: `ttf-parser` 0.25.1, `zbus_names` 4.3.5, `zvariant` 5.15.1 and
`zvariant_derive` 5.15.1. Updating resvg removes the previous rustybuzz warning.
Online Bun audit reports no advisories. These findings describe dependency
advisories, not a complete application security assessment. Windows/macOS,
the remote CI/release matrix and production deployment were not run locally.

## Home scrollbar placement — 14 November 2026

The home scroll area no longer shrinks horizontally to the occupied content
width. Its vertical scrollbar stays at the window's right edge while the cards
retain their centered, width-limited responsive layout. The existing home
navigation, document-card and responsive-grid checks pass; generated light and
dark home screenshots were visually inspected.

All **184 workspace tests** pass sequentially. Native and WebAssembly
`cargo check --locked -p op-app`, formatting, local Markdown targets and
`git diff --check` pass. Release builds, browser runs, Windows/macOS and
deployment were not repeated for this layout-only change.

## Silent routine automatic saving — 14 November 2026

The routine saving/saved status panel and its English, French and Spanish
messages were removed. The bottom panel now appears only for automatic-save
errors, with the storage location available on hover. Scheduling, write
acknowledgements, close/exit flushing and manual save behavior are unchanged.

All **184 workspace tests** pass sequentially, including automatic-save
failure/retry and translation inventory checks. Native and WebAssembly
`cargo check --locked -p op-app`, formatting, local Markdown targets and
`git diff --check` pass. Release builds, browser runs, Windows/macOS and
deployment were not repeated for this presentation change.

## Grid-aware BPMN automatic arrangement — 14 November 2026

The BPMN editor passes its active snapping setting and 10-unit step into the
domain layout. Mixed-size shapes share row centers. When snapping is enabled,
centers, pool origins, widths and lane heights align outward to the grid,
preserving shape sizes, clearance and original pool/lane membership. Disabling
snapping retains fractional placement. Flow geometry resolves from the placed
shapes using the shared preferred ports/circular boundaries. Invalid grid steps
are rejected before mutation; repeated arrangement is idempotent.

A new mixed-shape regression failed on the previous layout before the fix.
The final **188 sequential workspace tests** pass (84 application, 95 core,
seven document integration and two generated-case tests). Coverage includes
fractional sizes, several grid steps, multiple pools, complete shape containment,
nonoverlap, disabled snapping, unchanged business properties and endpoint
attachment for straight, orthogonal and curved flows. The existing UI regression
now exercises snapping both on and off through wide/compact controls and checks
undo/redo. Its wide snapped screenshot was visually inspected.

Native and WebAssembly checks, strict Clippy, formatting, local Markdown targets
and `git diff --check` pass. Document schemas and compatibility fixtures are
unchanged. Release builds, browser runs, Windows/macOS and production deployment
were not repeated for this change.

## Presentation website refresh — 14 November 2026

The English, French and Spanish presentation now covers the eleven document
types and BPMN evaluation workflow in home-screen order, with links to the
implemented-feature guides. Copy and search/social metadata include free
drawing, database design, UML and Mermaid alongside BPMN/Petri/discovery.
A local-saving section explains the desktop Documents/Open Process library,
browser local storage and separate manual `.opx` copies.

Eight WebP screenshots replace/extend the previous four, using actual application
UI captures from checkout `cd389d3`. The first preview is drawing; its counter
derives from the rendered tabs. Captions, alternative text, keyboard tabs and
the enlargement dialog follow the selected image. The website README records
asset provenance. The obsolete source-build README anchor was corrected.

`bun run build` passes with Bun 1.4.4 and Node 26.13.1: Astro/TypeScript checks,
localized static generation and the Trunk release application under `/app/`.
All **nine Bun tests** pass. A local Chromium run under the shipped presentation
CSP checks all three routes, twelve catalogue entries, eight previews/dialogs,
keyboard navigation and six widths from 320 to 1440 px for each language.
It also checks the static catalogue, initial screenshot and language navigation
with JavaScript disabled. No local asset failures, horizontal/tab overflow,
browser exceptions or CSP violations were observed. French desktop/mobile,
catalogue and storage screenshots were visually inspected.

Local Markdown/repository-guide targets and patch whitespace were checked.
GitHub release responses in browser checks were mocked; live release services,
production Cloudflare deployment, other browser engines and the remote CI matrix
were not exercised. No Rust models or application behavior are part of this
website change, and the Rust unit suite/native release matrix was not repeated.
Parallel application/interface edits in the working tree remain outside this
website change.

## Menu content padding — 14 November 2026

The shared popup style restores 10 px horizontal and 4 px vertical button
padding, replacing egui's compact menu defaults. It applies to the language
dropdown, main menus/submenus, editor and example menus, tab/canvas context
menus and database relationship picker. Inspectors, cards, dialogs and form
dropdowns already have their own content margins.

All **188 workspace tests** pass sequentially. Generated language, light/dark
BPMN context-menu, export submenu and compact-options screenshots were visually
inspected. Native and WebAssembly `cargo check --locked -p op-app`, formatting,
local links in `interface.md` and `git diff --check` pass. Release builds, browser
runs, Windows/macOS and deployment were not repeated for this spacing change.

## Saved diagram thumbnails — 14 November 2026

Closed **Saved diagrams** now reuse the rendered PNG previews already used for
open documents. The bounded preview cache associates acknowledged captures with
the automatic copy's stable UUID and revision. It restores closed cards without
an open workspace, retains compatibility with older UID-based cache entries and
also flushes previews through the browser lifecycle save path. `.opx`, automatic
document records and workspace schemas are unchanged. Missing or outdated images
fall back to notation icons until the document is displayed again.

All **194 workspace tests** pass with sequential execution. New coverage checks
distinct images for same-named documents after closing, cache persistence,
restart and reopening; it also rejects advanced/deleted library references and
previews mismatching a restored draft. The generated
`target/screenshots/automatic_library_thumbnails.png` was visually inspected.
Native and WebAssembly `cargo check --locked -p op-app`, formatting, local links
in the updated documentation and `git diff --check` pass. Release builds, actual
browser lifecycle/storage runs, Windows/macOS and the remote CI matrix were not
run for this change. Parallel BPMN/interface edits were preserved.

## Language flag spacing — 14 November 2026

The language button and its three options use the theme's horizontal item
spacing (8 logical pixels) between flags and text. The generated
`target/screenshots/language_dropdown.png` was visually inspected; the existing
home-navigation test passes, including accessible names, selection and closure.

Native and WebAssembly `cargo check --locked -p op-app` pass. The sequential
workspace test command reports 87 passing application tests and one failure in
`bpmn_pool_and_lane_resize_gestures_snap_undo_cancel_and_keep_elements_fixed`
at `app/tests.rs:948` (the unsnapped lane-height assertion). That command stops
before core tests; a separate `cargo test --locked -p op-core` passes all 108
tests. The failing BPMN test was not changed as part of the spacing correction.
UI-module formatting, local links in `traductions.md` and `git diff --check`
pass. Browser runs, release builds and the remote CI matrix were not repeated.

## BPMN pool and lane border resizing — 14 November 2026

Pool borders and all four corners can be dragged without selecting the pool
first. Directional system cursors and a highlighted border identify the target;
hit tolerances stay constant in screen pixels. Internal separators redistribute
two neighboring lane heights without changing total pool height or other lanes.
The domain operation preserves element positions, limits shrinking against their
bounds and normal minimum sizes, and honors optional grid snapping. Typed gesture
history provides a single undo step and Escape cancellation. No document schema
or dependencies changed.

Four domain regressions and two UI gesture/cursor tests pass. They cover all
edges/corners, single/multiple lanes, element constraints, fractional coordinates,
invalid requests, three actual camera zooms, both themes, grab offsets,
undo/redo/cancel, panning and creation-tool priority. The earlier unsnapped-height
failure recorded above was corrected by setting the test camera through the
saved view state, which otherwise overwrote the direct camera assignment.
Light/dark hover and resized-pool screenshots were visually inspected.

The complete `cargo test --locked -p op-core` passes **108 tests**. Native and
WebAssembly checks, formatting of the edited Rust files, local documentation
links and `git diff --check` pass. The sequential workspace run passed 87 of 88
application tests, then failed its translation inventory during concurrent exam
mode edits. Later application-test reruns could not compile because the existing
`manual_alpha_mode` test still referenced the removed `MiningEditor.manual`
field. These unrelated in-progress changes prevent claiming a full workspace
pass. Release builds, browser runs, Windows/macOS and remote CI were not run.


## Application-wide exam mode — 14 November 2026

**Tools → Exam mode** replaces Mining's manual-mode selector. It turns off
pedagogical assistance across open and newly created documents, while keeping
authored content editable and saveable. The Mining job only parses activity
headers in this mode; Alpha discovery and soundness are skipped. Answer fields
and matrix cells contain no model answers, corrections, hints, scores or generated
previews. The old log `manual` field and existing answers still round trip.

Session guards block assistance commands and generated-answer exports, and open
result panes show a disabled message. Notation UIs hide validation/evaluation,
conversions, automatic layout and SQL generation. Petri simulation and background
analysis are cancelled. Examples, templates and home preview images are hidden.
Mode changes leave document snapshots and stored view preferences intact.

Two UI regressions pass for menu activation/deactivation, answer preservation,
matrix editing/undo, serialization, export guards, existing result panes,
simulation shortcuts and documents created during an exam. A worker regression
checks that exam output contains activity headers without solutions or soundness,
and honours cancellation. The three application i18n checks pass after removing
36 retired manual-mode messages from each catalogue. The first full application
run passed 89 tests and failed only that retired-message inventory before the
catalogues were cleaned up. The exam screenshot was visually inspected.

The subsequent complete sequential workspace run passes **90 application tests**
and **110 core tests** (101 unit tests, 7 document regressions and 2 untrusted-input
checks). Native and WebAssembly compilation checks pass. The application test
build reported an unused import in the concurrently edited whiteboard module;
that warning was absent from the later native check. A minimal `f32` annotation
was added to the concurrently edited UML minimum-size helper after the compiler
reported an ambiguous float type. Local documentation links, formatting checks
for the exam-related Rust files and `git diff --check` pass. These are local Linux
checks; release builds, an actual browser session, Windows/macOS and remote CI
were not run for this change.

## Shared border/corner resizing — 14 November 2026

Resizable BPMN objects, UML class/use-case elements, EPC/activity nodes, database
tables and non-connector free-drawing elements share screen-space border picking,
grab offsets, directional cursors and highlights. The geometry operation in
`op-core::geom::ResizeEdge` keeps opposite edges fixed and applies the supplied
size limits and snapping. Pool/lane constraints remain specific to BPMN. Class
members and database columns determine minimum visible heights; free-drawing
connectors retain their endpoint editing and attachments. Existing English,
French and Spanish resize instructions now describe edge/corner gestures.
No dependencies or serialized document/view fields were added.

All **202 workspace tests** pass sequentially: 92 application tests, 101 core
unit tests and nine core integration tests. New coverage exercises eight resize
directions in all seven graphical editor types, both themes across the editor
set, different zooms, hover without mutation, offset grabs, fixed opposite edges,
minimum sizes, fractional resizing with snapping off, atomic undo/redo and Escape
cancellation. Shared picking tests cover fixed screen tolerance and overlapping
elements. Existing attached-connector and import/storage regressions pass.
Representative class, use-case, database, EPC and free-drawing captures were
visually inspected. The free-drawing test explicitly sets its snap preference
before testing on/off behavior; its first run had incorrectly assumed snapping
was enabled by default.

`cargo test --locked -p op-core`, native and WebAssembly compilation checks,
workspace formatting, local links in the updated documentation and
`git diff --check` pass. The former exam-related global test failures recorded
above are resolved in this checkout. Release builds, actual browser interaction,
Windows/macOS and remote CI were not repeated.

## Examples restricted to creation cards — 14 November 2026

The process-discovery input header no longer shows its separate **Examples**
dropdown. A source audit found no other editor-level **Examples** menus; home
and the new-document picker retain their shared card menu and both registered
sample logs. The two obsolete menu-only translation keys were removed from the
English, French and Spanish catalogues.

The updated process-discovery screenshot was visually inspected. Home/picker,
example registry, discovery and translation inventory checks pass. The complete
workspace command was interrupted by SIGTERM during the UML UI checks; the
remaining UML, free-drawing and recovery UI tests were rerun successfully in
short batches, followed by all 32 non-UI application checks and 110 core tests.
Together the completed runs cover all 202 workspace test cases. Native and
WebAssembly compilation, formatting, local documentation links and
`git diff --check` pass. Browser runs, release builds, Windows/macOS and the
remote CI matrix were not repeated.

## Standard drawing editing — 14 November 2026

Free drawing supports system clipboard copy/cut/paste across drawing documents,
plain-text paste, Alt-drag duplication (including pressing Alt during a move),
Shift-constrained movement/drawing, stacking order and F2/Enter text editing.
The Edit menu consumes registered commands and the canvas has context actions.
Focused text fields retain their own clipboard and undo. The shell consumes
Ctrl/Cmd+Shift+Z before Ctrl/Cmd+Z, and Shift+S before S, so redo and Save as are
not intercepted by the less specific shortcuts. Clipboard selections have a
separate versioned JSON format; no dependencies or document/session schemas
changed. Browser keyboard paste uses system clipboard events; browser menu paste
uses the last application-copied drawing selection.

Five new domain tests cover portable selections, fresh IDs, internal/external
attachments, malformed/future/over-budget input, atomic rejection, stable stacking
and constrained geometry. Four new egui interaction tests cover copy/cut/paste
between documents, native menu-paste requests, styles/freehand/connectors,
undo/redo, Alt/Shift dragging, cancellation and original selection restoration,
text-field focus/undo, context/Edit menus, stacking keys, F2 and free pencil
samples with snapping enabled. The clipboard, context menu and focused text
screenshots were visually inspected. The full core command passes 115 tests
(106 unit tests and nine integration tests). The complete sequential workspace
command passes **211 tests**: 96 application tests and those 115 core tests.
Native and WebAssembly compilation, workspace formatting, local documentation
links and patch whitespace checks pass. These are local Linux checks and egui
clipboard-event simulations; actual OS/browser clipboard integration, release
builds, Windows/macOS and remote CI were not run for this change.

## Combined pre-commit validation — 14 November 2026

The complete sequential workspace command passes all **211 tests**: 96
application tests and 115 core tests. The separate core test command, native and
WebAssembly `cargo check --locked -p op-app`, workspace formatting and
`git diff --check` also pass. All 110 local links in the changed Markdown files
resolve to existing files.

The full website `bun run build` passes, including Astro checks with no
diagnostics and the Trunk WebAssembly release build. All nine Bun tests pass
against that combined output. Generated HTML checks cover all six localized
terms/privacy pages, their metadata, canonical/alternate URLs, sitemap entries,
CSP routes, scripts, localized footer links and 198 local links/assets/anchors.
No new browser session, native release build, Windows/macOS run, deployment or
remote CI matrix was performed during this validation.


## Shared Open Process visual identity — 14 November 2026

The application and presentation site share a paper/graphite/vermilion identity,
Inter typography, small frame corners and ruled controls. The home/creation grid
uses neutral icons with a single action accent; small registry-color markers
retain notation identification. Canvas surfaces and selection colors follow the
shared theme, while authored drawing styles and semantic diagram colors remain
unchanged. The O → P SVG, generated native/web icons, web loading colors and all
eight real application captures in the website gallery were refreshed. The
[identity guide](identite-graphique.md) documents tokens and maintenance.

The complete sequential workspace suite passes **211 tests** (96 application and
115 core tests). Four home tests were rerun successfully after the final card
marker refinement. Eight existing gallery/UI tests were rerun after aligning the
shared floating palettes and tool buttons with the small frame corners. Native
and WebAssembly `cargo check --locked -p op-app` pass
without warnings. Two existing packaging tests pass. Workspace formatting, local
Markdown links and patch whitespace checks pass.

The full website build passes with no Astro diagnostics, including the Trunk
WebAssembly release build, followed by all nine Bun tests. Local Firefox checks
cover the three home pages and six legal pages at actual viewport widths of
320, 390, 768 and 1440 CSS pixels. They check horizontal overflow, loaded images,
language switching, Escape/outside-click closure, legal footer/section links and
gallery switching/enlargement. WebDriver BiDi set the smaller viewports because
ordinary Firefox window resizing stopped at 500 px. The initial mobile header
wrapping was corrected before the final browser run. Temporary browser profiles
and loopback servers were shut down by the check.

Desktop/mobile website captures, the catalogue, and light/dark application views
were visually inspected. Seven principal text/button color pairs have calculated
contrast ratios of at least 4.5:1; this is a palette check, not a complete
accessibility audit. Native release builds, Windows/macOS display, deployment
and the remote CI matrix were not run for this identity change.


## Selectable appearance families — 14 November 2026

Classic restores the previous default appearance. Cloud, Lilac, Blossom, Lagoon,
Midnight, Graphite and Workshop offer alternatives with distinct colors, corners,
shadows and website compositions. All families support light/dark colors. The
website's sticky selector, swatches and Reset use translated EN/FR/ES labels and
a browser-local preference. The app has a context-local typed selection and a
separate persisted setting. Explicit website app links pass the family and mode
to the local `/app/` route. Saved languages, documents, histories, cameras and
`.opx` schemas retain their previous behavior.

The complete sequential workspace command passes **215 tests**: 100 application
tests and 115 core tests. New tests cover preference restoration and malformed
values, isolated application contexts, explicit URL overrides, the real theme
menu and preservation of document snapshots, view state, history revisions and
language. The gallery test checks snapshots while generating 128 real captures
across eight views, eight families and two modes. The exporter validates and
converts all 128 images, updating the eight stable Classic fallback URLs.

Native and WebAssembly `cargo check --locked -p op-app` pass without warnings.
The full website build passes with no Astro diagnostics, including the Trunk
WebAssembly release build. All **12 Bun tests** pass, including preference/query
fallbacks, matching local app destinations, translated controls and the presence
of all 128 gallery images. Two packaging tests pass. Workspace formatting,
Markdown links and patch whitespace checks pass.

Firefox checks under the shipped CSP cover all eight families in both modes at
1440, 390 and 320 CSS pixels. They check actual gallery images, horizontal
overflow, matching app-link parameters, reload/language persistence, Reset,
explicit preview URLs, changing an existing preview URL, invalid browser storage
and gallery enlargement. The compiled WebAssembly application also starts from
an explicit Cloud/dark URL; its rendered palette matches the catalogue. Desktop,
mobile and application captures were visually inspected after transitions
settled. The existing Astro development server on `localhost:4321` was also
checked: its shared catalogue, selector, real images and reload behavior work.
That user-owned server was left running. Temporary test servers and browser
profiles were closed afterwards.

These are local Linux/Firefox checks. Native release builds, Windows/macOS
rendering, Safari/Chromium, deployment and the remote CI matrix were not run.


## Classic as the sole theme — 14 November 2026

Classic is now the only application and website identity. The alternative
families, palette catalogue, comparison bar, swatches, family preferences and
alternative preview images have been removed. Shared application radii remain
centralized in `theme.rs`. The existing application color-mode controls remain;
the website has a compact translated sun/moon button. Its former preference
migrates only the light/dark choice to a mode-only key. Retired `theme` parameters
cannot restore another family. Documents, histories, language preferences and
`.opx` schemas are unchanged.

The complete sequential workspace command passes **213 tests**: 98 application
and 115 core tests. A regression checks that old theme URLs leave Classic's
palette intact while valid color-mode parameters still work. The Classic gallery
test preserves document snapshots while rendering eight views in both modes.
The exporter refreshes 16 real Classic images and the eight stable fallback URLs.
Native and WebAssembly compilation checks, workspace formatting, local Markdown
links and patch whitespace checks pass.

The full website build passes with no Astro diagnostics, including the Trunk
WebAssembly release build. All **12 Bun tests** pass. They cover retained color
modes from old preference values, mode-only app links, ignored retired URL
parameters, removal of theme controls from all nine localized pages and the 16
remaining gallery images.

Firefox checks under the shipped CSP pass on the nine presentation/legal pages
in both modes at 1440, 390 and 320 CSS pixels. They cover overflow, header overlap,
a single-line mobile app button, preference migration, reload/language retention,
retired URL normalization, app links and gallery enlargement. The compiled web
app renders Classic dark colors even when opened with an obsolete Cloud link.
Desktop/mobile and application captures were visually inspected. The existing
Astro server on `localhost:4321` also passes a live selector-removal, gallery and
mode-persistence check, and was left running. Temporary test servers and browser
profiles were closed. Native release builds, other operating systems/browsers,
deployment and the remote CI matrix were not run.


## UML relation tools and contextual actions — 14 November 2026

Class and use-case builders no longer show a horizontal business action bar.
The floating Connect tool opens a relation picker on hover, click or keyboard
activation, with notation-specific types and matching UML symbols. The chosen
type becomes the palette icon and remains available through C. Switching types
clears an unfinished connection. Alignment/distribution moved to the selection
inspector and canvas context menu. Duplication remains in the context menu and
its shortcut; class-to-Mermaid conversion remains in Tools and is also available
in the canvas context menu. Exam mode hides the conversion. Document formats,
saved view preferences and domain relation rules are unchanged.

The complete sequential workspace suite passes **216 tests**: 101 application
and 115 core tests. Three new UI regressions cover all seven class relation types
with click/drag creation and atomic undo/redo, tool selection without document
edits, remembered types through C, valid use-case choices, keyboard activation,
Escape, hover dismissal, pending-source cancellation and arrangement access.
The existing mouse and independent-document conversion tests pass with the new
controls, including conversion from a 420 px view's context menu.

Native and WebAssembly compilation checks, workspace formatting, local Markdown
links, the annotated SVG's XML syntax and patch whitespace checks pass. The full
website build and all **12 Bun tests** pass. The Classic gallery test renders
16 current views without changing document snapshots; the exporter refreshes
the two UML images and their stable fallback.

Firefox runs the compiled application under the shipped CSP with a temporary
profile/server. Real pointer interactions open the relation picker, choose
Generalization, create two classes and connect them. Captures at 1400 and 420 CSS
pixels were visually inspected, alongside light/dark UI-test captures and the
selection context menu. Temporary browser/server processes were closed. Native
release builds, Windows/macOS display, other browsers, deployment and the remote
CI matrix were not run.


## Website system color mode — 15 November 2026

Without an explicit choice, presentation and legal pages now read
`prefers-color-scheme` on load and listen for changes while open. Automatic mode
does not write a browser preference. A sun/moon button choice or valid `mode` URL
remains an explicit override, stored under `open-process-color-mode-v2`.
Ambiguous v1 color/appearance preferences are cleared, including old manual
choices, because previous visits saved defaults as though they were explicit.
Language preferences and application/document storage are unchanged.

The full website build passes with no Astro diagnostics, including assembly of
the unchanged WebAssembly app. All **18 Bun tests** pass. Six new tests execute
the bundled browser event handlers and cover initial/live system modes without
automatic persistence, old pinned defaults, explicit choices/reload, cross-tab
updates/removal/clear, URL priority, malformed values and unavailable storage.

Firefox checks under the shipped CSP use real changes to the browser's system
color-scheme preference and confirm both initial dark and live light/dark modes
on all nine EN/FR/ES presentation/legal pages. Logos, theme-color metadata, app
links and the selected gallery view update together. Checks also pass for manual
choice persistence across reload/language navigation, storage events between
tabs, clearing an override and explicit URL modes. Desktop and 320 px mobile
captures in both modes were visually inspected. The existing Astro server at
`localhost:4321` passes the same checks and was left running. Temporary test
profiles and servers were closed. Local documentation links and patch whitespace
checks pass.

Rust sources and models were not changed; the Rust test suites were not rerun.
Other browsers/operating systems, deployment and the remote CI matrix were not
run for this website correction.


## Mermaid and Markdown exports — 15 November 2026

File → Export now offers `.mmd` and `.mermaid.md` for BPMN, Petri, EPC, UML
activities, classes and use cases, alongside existing formats. The Mermaid editor
also exports Markdown without requiring a valid preview. Typed projections retain
graph identities/connections and notation labels, with loss notes in both file
forms; [the format guide](export-formats.md) describes exact limits. No toolbar,
dependency, model version or document/session schema is added.

The Mermaid module imports one complete fenced block, including invalid drafts.
Multiple/unfinished Mermaid blocks produce a targeted error. The event-log
fallback excludes explicit Mermaid fences, so Markdown is not mistaken for trace
data. The session's per-format assistance hook hides generated projections in
exam mode without removing native exports; authored Mermaid text remains exportable.

`cargo test --locked -p op-core` and the complete sequential workspace suite pass.
The workspace contains **225 passing tests**: 103 application and 122 core tests.
Seven new core tests parse/render projections, check BPMN pool/lane membership
and arrow kinds, initial Petri markings/capacities/inhibitors/weights, empty
diagrams, escaping, deterministic output, exact Markdown source round trips and
fence rejection. Two new adapter tests cover all six notation families, registry
reopening, unchanged snapshots/revisions/views/native exports and exam filtering.
The existing File/Export UI test now covers the new formats and all relevant types.

Native and WebAssembly compilation checks pass without warnings. The full website
build, including the Trunk release app, passes with no Astro diagnostics; all
**18 Bun tests** pass. Formatting, local Markdown links and patch whitespace
checks pass.

Firefox runs the rebuilt app under the shipped CSP with an isolated profile and
loopback server. It creates a three-node/two-flow BPMN, invokes both real export
menu actions, and checks the generated Blob contents and `.mmd`/`.mermaid.md`
download filenames. Native `.opx` snapshots before/after export are equal.
Dropping the exported Markdown into the app opens a Mermaid document with the
exact exported source; saving it confirms the registered document type. The
export menu and rendered Mermaid preview were visually inspected. The test
captures download bytes instead of writing through the browser's download UI.
Temporary browser/server processes were closed and existing servers left running.

Native save-dialog interaction, other operating systems/browsers, official
JavaScript Mermaid rendering, deployment and the remote CI matrix were not run.


## Strict Clippy CI correction — 15 November 2026

The [reported Ubuntu job](https://github.com/linventif/open-process/actions/runs/43717166198/job/132821075395)
completed its tests but failed `cargo clippy --locked --workspace --all-targets
-- -D warnings` under Rust 1.100.0. `Footprint::pairs` now enumerates matrix rows
and cells instead of indexing range loops. Its ordering, directed relations,
single symmetric pairs, parallel self-loops and omitted independent diagonals
retain their behavior. A new regression checks all four relations and empty logs.

The full lint command also exposed an existing nested thumbnail-acknowledgement
condition and the size imbalance between mining result variants. The condition
uses a let chain, retaining its short-circuit behavior. The discovery variant
boxes its `AlphaResult`; a worker transport regression confirms the JSON remains
readable as the previous unboxed type. Documents and session schemas are unchanged.

The exact strict Clippy command passes on Linux with **Rust 1.98.0 and 1.100.0**.
The 1.100.0 compiler and build artifacts use temporary/ignored directories;
the default Rust toolchain remains unchanged. Formatting checks pass under both
versions. With the default Rust 1.98.0, core tests and the complete sequential
workspace suite pass **226 tests**: 103 application and 123 core tests. Native
and WebAssembly compilation checks also pass. Documentation links and patch
whitespace checks pass.

These are local checks. The failed GitHub run was inspected but not rerun;
native release builds, the website build, other operating systems and the remote
CI matrix were not rerun for this correction.

## Integrated website documentation — 16 November 2026

The website now publishes 40 maintained documentation sources under `/docs/`,
with equivalent navigation routes under `/fr/docs/` and `/es/docs/`. The landing
catalogue's twelve tool/workflow links open dedicated internal guides. Ten new
English guides cover the previously missing tool workflows and troubleshooting;
existing Markdown guides and project documents are rendered directly.

The documentation includes grouped navigation, accent-insensitive title/summary/
keyword filtering, section contents, group-local previous/next links and active
page state. Language switching keeps the selected guide; French and Spanish
navigation explicitly identifies the English article content. Relative guide
links keep the route language; source links remain repository links. Interface
artwork and SQL examples are available locally. Presentation CSP rules also
cover the documentation routes, without inline scripts or syntax-highlighter
styles.

Local checks performed:

- `bun run build:site`: Astro/TypeScript reports zero errors, warnings and hints;
  the static build generates 133 HTML pages, including 123 documentation pages.
- `bun test --test-name-pattern '^(?!the shared Cloudflare output|the standalone app)'`:
  22 tests pass. The two output tests that require a freshly built Wasm application
  are deliberately excluded from this presentation-only check.
- Documentation tests cover every maintained `docs/*.md` source, all twelve tool
  destinations, local guide/asset links and anchors, labelled heading targets,
  language alternates, canonical URLs, sitemap entries and the documentation CSP.
- A temporary headless Firefox harness exercises the built site's presentation
  script/style restrictions. All 21 browser assertions pass: internal guide links, search
  normalization/multiple terms/empty/reset states, article rendering, language
  switching, desktop contents, saved color preference, compact menu interaction,
  viewport resizing, local image loading and absence of page overflow on the
  index/database/interface pages at 375 px.
- Desktop index and mobile dark-mode BPMN screenshots were visually inspected.
  Temporary Firefox profiles and the owned localhost server were closed.
- `git diff --check` passes.

The combined Rust/Wasm build, the full Rust suite, other browsers/operating
systems, remote CI and production deployment were not run for this website work.

## Mermaid catalogue, forms and initial native flowcharts — 16 November 2026

**The request for complete native graphical editors for every Mermaid family is
not complete.** Basic source forms/templates are not counted as dedicated native
editors. The [Mermaid guide](mermaid.md#native-graphical-editor-work) identifies
the implemented subset and remaining native work.

The Mermaid module declares 35 templates covering the reviewed catalogue, with
five C4 levels, and 75 basic statement forms. Twenty-seven templates have local
Rust previews. Swimlane, Mermaid use case, event modeling, Venn, Ishikawa,
Wardley, Cynefin and TreeView retain source/forms, storage and text exports, with
an explicit unavailable-preview message. Switching Source/Builder alone retains
all source bytes, saved state and revisions. Applied edits guard the selected
statement and preserve surrounding unknown syntax. Builder input is bounded;
statement source lines and virtualized rows avoid repeated prefix scans and
drawing every row of a long source.

An initial native `flowchart` module now registers its own typed model,
descriptor and example through composition. It reuses FlowEditor and
DiagramLayout for creation, movement, resizing, selection, properties, labelled
connections, history and SVG/DOT/Mermaid exports. The five supported kinds are
process, decision, terminator, circle connector and preparation. Basic native
flowchart ↔ Mermaid conversion preserves these kinds, names and connections;
layout is regenerated with a diagnostic. Rounded rectangles map to terminators
with a presentation diagnostic. Groups, unsupported shapes, decorated/dashed
links, self-links and parallel links fail without opening a partial destination.

Native flowchart import runs in the existing native pool/Web Worker. Changed
source/name/language and exam mode cancel pending imports; stale results cannot
open a destination. The common flow storage validator indexes node kinds rather
than repeatedly scanning nodes for each edge. Existing EPC/activity invariants
and payloads retain their behavior. The new kind uses model version 1; existing
document versions, .opx envelopes and dependencies are unchanged.

Tools commands also open projections directly from the six existing BPMN, Petri,
EPC, activity and UML builders. Source documents/histories/views stay unchanged;
conversion notes accompany the new source as comments and a message.

Local checks performed:

- `cargo test --locked -p op-core` passes **130 tests**.
- `cargo test --locked --workspace -- --test-threads=1` passes **241 tests**:
  111 application and 130 core tests.
- New regressions cover family forms/storage, rendering of preview-capable
  patterns, delimiter-containing labels, guarded edits, atomic errors, bounds,
  dates/packet ranges, Unicode, preambles, unknown syntax, original encodings and
  CRLF. UI checks cover switching, apply/remove, undo/redo, reopening, all
  templates, direct conversions, the last example in the scrollable home menu
  and native graphical create/connect/move gestures. Worker tests cover stale
  source and exam cancellation.
- Native and `wasm32-unknown-unknown` compilation, strict workspace Clippy,
  formatting and patch whitespace checks pass.
- Local links in the Mermaid documentation updates were checked. Native
  flowchart and builder-inspector captures at 1,400, 420 and 280 px were visually
  inspected. The inspector captures show a rendering-in-progress state; SVG and
  rasterizer behavior is covered by automated tests.

The official JavaScript Mermaid parser was not executed: a temporary validation
installation failed because npm registry DNS resolution was unavailable. Newer
starter forms follow the official syntax documentation without claiming full
grammar validation. Native release builds, the website/Trunk build, browser
interaction, other operating systems and remote CI were not run for this Mermaid
work. The separate website verification above describes independent website work.

## Shared graphical selection editing (2026-11-16)

BPMN, Petri, UML class/use-case, EPC, activity, native flowchart and database
builders use the shared typed selection clipboard controller. It supplies
copy/cut/paste, duplicate/select-all commands, canvas context actions and native
paste requests/browser menu fallback. The drawing editor retains its existing
portable format, plain-text paste and gestures and shares clipboard event
ownership with the controller. No document or workspace storage schema changed.

Domain regression tests cover BPMN pools/lanes, documentation and default flows;
Petri markings, capacities, silent transitions and inhibitor weights; flow labels
and omitted external edges; relational column/index/foreign-key remapping and
SQL name uniqueness; and atomic rejection of malformed clipboard references.
Four UI scenarios cover all eight newly connected builders' selection commands,
copy/cut/paste across documents, text-field focus, Edit/context menus and Alt-drag
with one-step undo/Escape cancellation, plus a fifth BPMN pool regression preserving
original nodes while moving only the copy.

Checks performed on the final checkout:

- `cargo test --locked -p op-core`: passed (135 tests).
- `cargo test --locked --workspace -- --test-threads=1`: passed (251 tests,
  including 116 application tests).
- `cargo check --locked -p op-app`: passed.
- `cargo check --locked -p op-app --target wasm32-unknown-unknown`: passed.
- Local links in the six affected documentation pages: 96 checked, none missing.
- `git diff --check`: passed.

UI coverage uses egui clipboard/pointer event simulations. Physical OS/browser
clipboard integration, native release builds and the full three-OS CI matrix
were not exercised by these local checks.

## Combined commit preparation checks — 16 November 2026

The remaining selection-editing and website documentation changes were checked
together before committing them. The complete sequential workspace suite passed
251 tests (116 application and 135 core). Native and WebAssembly checks passed.
Four nested conditions flagged by strict Clippy were simplified without changing
short-circuit behavior; the five selection-editing UI tests were rerun afterward,
and strict Clippy plus formatting checks pass.

The site and application builds were both run with `bun run build:site` and
`bun run build:app`; the latter rebuilt the release WebAssembly application.
The complete `bun test` suite passes all 24 tests, including the previously
excluded output checks and the documentation link/anchor/CSP checks. No new
interactive browser run or remote CI matrix was performed for commit preparation.

## Simple drawing sizes — 17 November 2026

Free drawing uses XS–XL presets for stroke width and font size. A collapsed
Advanced section exposes exact stroke/font values and single-element dimensions.
Two egui interaction tests cover preserving custom precision when opening or
focusing fields, exact edits, undo, mixed selections and styles for new elements.
The simple and advanced inspector screenshots were visually inspected.

The complete sequential workspace suite passes **253 tests** (118 application
and 135 core). Native and WebAssembly compilation, workspace formatting, the 12
local links in the drawing guide and `git diff --check` pass. These are local
Linux and egui checks; browser interaction, release builds and the remote CI
matrix were not run for this change.

## macOS application bundle signing — 17 November 2026

The packaging script now ad-hoc signs the complete macOS `.app` after adding
metadata, icons and licences, then requires strict signature verification before
archiving. The release workflow also verifies the bundle extracted from each
macOS distribution archive before uploading it. This does not provide Developer
ID signing or Apple notarization, and does not establish the exact cause of a
particular user's Gatekeeper alert.

On Linux with Python 3.14.4, four packaging tests pass, covering archive contents,
Linux installation, complete-bundle signing order and failure propagation. The
fifth test is macOS-only and was skipped locally: it compiles a real executable,
packages and extracts it, verifies and executes it, then checks that modifying a
bundled resource invalidates the signature. Existing macOS CI will run that test;
no remote CI result is claimed here.

`bun run build:site` passes Astro checks and the static site build. Ten targeted
Bun tests pass, including generated documentation links/anchors and release
download selection (local Bun 1.4.0). Release workflow YAML, Bash syntax and
embedded Python syntax checks pass, as does `git diff --check`.

No Rust source changed, so workspace tests and native/WebAssembly builds were
not repeated. No actual macOS signing, quarantined-download launch, full CI
matrix, Developer ID/notarization operation or publication was performed locally.

## Desktop release checks — 17 November 2026

Official native builds embed the release tag computed once by the release
workflow. A native platform service checks GitHub metadata on a background thread
and the shell shows translated update, download and release-note links. Stable
and automatic `main-*` builds use separate channels. Startup failures stay quiet;
manual checks report the result. No document or workspace schema changed.

The complete sequential workspace suite passed. The final nine focused updater
tests also pass with `OP_RELEASE_TAG=main-123-abcdef0`, covering displayed build
identity, semantic and automatic-build comparisons, identical commits, incomplete
uploads, platform downloads, untrusted metadata URLs, malformed responses,
nonblocking polling, popup dismissal, keyboard ownership, links and three
interface languages. Tests inject metadata/results without making live update
requests. The update popup screenshot was visually inspected.

Native and WebAssembly `cargo check --locked -p op-app`, strict workspace Clippy,
formatting and `git diff --check` pass. The site build and ten targeted Bun tests
pass, including documentation links/anchors and release download selection.
Release workflow YAML/script syntax, stable/preview/automatic release-tag cases,
and the shared compilation/publication tag were checked locally.

`cargo audit --no-fetch --no-yanked` passed against the locally available RustSec
database, with four warnings about existing unmaintained transitive dependencies. The new HTTPS
and semantic-version dependencies are native-only. These local Linux checks do
not establish an actual GitHub-to-desktop download or Windows/macOS behavior.
No native release build, combined site/WebAssembly release build, remote CI run,
publication or deployment was performed for this change.

## Website dependency advisory gate — 17 November 2026

The [desktop update commit's website audit](https://github.com/linventif/open-process/actions/runs/44139862108/job/134239381062)
failed on `picomatch` advisory `GHSA-fc5q-39xc-vjpf`. The same online audit was
reproduced locally. The reviewed advisory lists no patched version; the npm
registry reports 4.0.7 as latest. The dependency remains unpatched. A single-GHSA,
time-limited exception for repository-controlled build globs is documented in
[SECURITY.md](../SECURITY.md#dependency-audit-exception), with automatic expiry
at 00:00 UTC on 1 December 2026. The complete advisory report remains in CI logs;
other advisories and registry failures still block the gate.

Online `bun run audit` passes with the exception banner and the unfiltered
`picomatch` report. Running it without network access returns failure rather
than a clean audit. Two policy tests pass, covering the exact advisory scope,
UTC expiry boundary and invalid-date behavior. Frozen installation with ignored
scripts passes without dependency or lockfile changes. Security workflow YAML
and the website/Rust audit commands were checked.

Astro checks and the static build pass; after permitting Cargo cache writes, the
WebAssembly release build completes the combined output. All **26 Bun tests**
pass, including output assets, documentation links/anchors and the new audit
policy tests. Local checks used Bun 1.4.0; CI selects 1.4.4. Local documentation
file links and `git diff --check` pass. No Rust source changed, so native builds
and the Rust workspace suite were not repeated for this audit-policy change.

## Writer version in `.opx` and multiple file opening — 19 November 2026

Saved envelopes now carry an optional `generator` (application, release tag and
`CONTENT_REVISION`); the envelope version is unchanged. Opening a native file
from a higher revision, or one whose content this version would drop, shows a
prompt listing unsupported items before anything opens. **File → Open** accepts
several files on native and web.
## Whole-diagram PNG export — 19 November 2026

PNG export from the BPMN, Petri, UML, EPC/activity/flowchart, database and free
drawing editors previously saved one screenshot of the visible canvas. It now
visits the diagram's bounds tile by tile at a fixed scale, copies each capture
into one image and restores the previous view (`canvas/export.rs`).

`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`,
native/WebAssembly checks and `git diff --check` pass; **266 tests passed**
(131 app, 121 core, 14 integration), none ignored.

Unit tests cover tile coverage, the size limits, and a simulated nine-tile
export: every output pixel comes from the expected tile, the view is restored and
the PNG reaches the save queue. A UI test zooms a BPMN view to 300% on one
corner, exports and checks that the PNG is wider than the window and than twice
the diagram width, that drawn content reaches both ends and that the view is
restored. `bpmn_full_export.png` (2440 × 1000 px) was inspected: no seams and no
selection. Native and web save dialogs were not exercised interactively.
## Shared auto arrange, bulk tab closing and matrix headers — 19 November 2026

Auto arrange now uses one layered engine for BPMN, Petri nets, EPC, UML activity,
flowcharts, UML class/use-case diagrams and database schemas. BPMN loop flows in
the orthogonal style are routed around intermediate elements. Tab context menus
add left/right/all bulk closing, and relation-matrix headers fit activity names.

```sh
cargo fmt --all -- --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace --no-fail-fast -- --test-threads=1
cargo check --locked -p op-app
cargo check --locked -p op-app --target wasm32-unknown-unknown
git diff --check
```

**264 tests passed** (128 app, 121 core, 15 integration), none ignored; clippy
and both compilation checks reported no warnings.

An integration test checks that new files record the current writer, historical
v1 fixtures have none, and a future envelope with unknown envelope and generator
fields still reads. It also checks that dropped fields are listed once per path
and that `null` placeholders are ignored. A UI test loads a future BPMN file with
an extra node field: nothing opens until a choice is made, the prompt names
`v9.0.0` and `nodes › halo`, **Cancel** opens nothing, two queued files open one
after the other, and a current file opens directly. `newer_file_prompt.png` was
inspected. The native multiple-selection dialog and the web reload were not
exercised interactively; the web path was only compiled.
**273 tests passed** (130 app, 129 core, 14 integration), none ignored; clippy
and both compilation checks reported no warnings. Tests cover layering direction, crossing reduction, cycles, isolated nodes, grid
alignment and invalid geometry; the existing BPMN layout tests pass unchanged.
UML tests check parents above children and use cases kept in their systems.
A UI test clicks **Auto arrange** for one example of each graph notation and
checks a single undo/redo step with an unchanged semantic revision. Another
opens four tabs, closes to the right, cancels an unsaved-changes prompt (the
queue stops), then answers it (the queue resumes) and closes all tabs.

The `auto_arranged_*.png` and `tab_close_menu.png` screenshots were inspected,
as were renders of a discovered 8-activity takeaway log (BPMN, Petri net and both
matrices, including an elided name). These local Linux checks do not replace the
CI matrix; no browser run, release build or deployment occurred.
## System language and theme — 19 November 2026

Until a language is picked in the dropdown, the app now follows the first
supported system or browser language, and English otherwise. The sun/moon button
returns to **System** when it selects the system's own theme. On Linux, the
desktop's light/dark preference is read from the freedesktop settings portal and
followed live. `sys-locale` and `zbus` were already compiled through eframe and
accessibility; declaring them adds no package to `Cargo.lock`.

`cargo fmt --check` and `cargo clippy --workspace --all-targets -D warnings`
(Rust 1.100, as in CI), native/WebAssembly checks and `git diff --check` pass;
**265 tests passed** (130 app, 121 core, 14 integration), none ignored.

Unit tests cover locale matching, an explicit choice outranking the system,
older storages (a saved non-English language counts as a choice, a saved English
does not), the explicit flag including an explicit English, portal value mapping
(including the deprecated nested variant) and the theme toggle returning to
**System**. Tests do not read the machine's locale, so English test labels stay
stable. A temporary live check on this Linux (Wayland) desktop read the portal
through the new thread and got light (value 0); following a live change was not
exercised.
## CSV/XLSX import and Petri discovery — 19 November 2026

The first TakeAway increment imports event logs into the existing discovery and
Petri editors. A module-owned binary importer decodes a workbook's sole matching
event-log worksheet with `calamine` 0.38; other sheets, business statistics and
Excel formula execution are outside this increment. The dependency is justified
by XLSX decoding, uses no optional features and accepts in-memory bytes on both
native and WebAssembly targets. CSV preparation removes only wholly empty
trailing columns. Source files remain unchanged; nonempty event columns remain
in the working source. Sparse parsing avoids materializing padded sheet
dimensions. Calamine still buffers workbook metadata and shared strings; the
cell limits are not a guarantee of constant memory or a decompressed ZIP-byte
limit.

Optional common boundaries produce short, silent `[start]`/`[end]` transitions.
Structured CSV/XLSX imports enable the visible option; historical documents and
teaching examples retain classic Alpha. Optional settings and typed import notes
round-trip without changing the `.opx` envelope or event-log model version.
Replaying silent routes and formatting the write-up now run in the existing
worker, with cached results on the UI. Changed options invalidate those results.
Replay is bounded to 5,000 markings per silent closure and 1,000,000 work steps;
a limit is inconclusive. Recorded `Vide*` activities remain unchanged. Existing
Petri-to-BPMN conversion can still show silent transitions as unnamed tasks;
that conversion was not extended in this Petri-focused increment.

The corrected CSV files in `tp/clean` were checked with actual Rust import,
discovery and replay: Counter 33 cases/6 variants, Phone 109/12 and Internet
185/13. All 327 cases replay with common boundaries; classic Alpha accepts only
the three Internet cancellation cases, reporting two final tokens for the other
cases. The clean CSV text is retained byte-for-byte. This establishes observed
trace replay, not general process correctness or conformance metrics.

The three XLSX files in `tp/clean` are byte-identical to the earlier workbooks.
Counter and Phone event worksheets differ from the corrected CSVs, so those CSVs
were the reference for this check. Three event-log and three Petri `.opx` files
were generated under `tp/results/import-petri`, checked by typed round-trip,
Petri storage validation and replay; originals were not rewritten.

Regression fixtures are small synthetic workbooks and logs, independent of the
user-supplied `tp` directory. Checks cover quoted CSV values, sparse padding,
ambiguous sheets/headers, cached formula values, missing activities, silent
routing, parallel starts/ends, naming collisions, old files, worker cancellation,
exam-mode suppression, stale exports and import-setting undo. The UI test opens
a padded CSV, discovers and edits its net, then saves/reopens both document
types. Its Petri screenshot was visually inspected, including the shorter
generated labels that avoid covering adjacent places.

Final checks passed: `cargo test --locked -p op-core` and the complete sequential
`cargo test --locked --workspace -- --test-threads=1` run (131 application tests
and 153 core/integration tests, 284 total). Native and WebAssembly
`cargo check --locked -p op-app` passed, as did a native debug
`cargo build --locked -p op-app`. Formatting, local links in the four changed
documentation pages and `git diff --check` passed. An initial translation-source
inventory failure was corrected and its targeted test passed before the final
complete run.

These are local Linux checks. No native release build, Trunk/site assembly,
interactive browser import, remote CI matrix, publication or deployment was
performed for this increment.

## Sales imports and quantitative analysis — 19 November 2026

The next increment adds the registered `process.sales` document. Its core model
owns recorded rows, source provenance, grouping, exact monetary sums and checked
persistence; the canvas-free editor owns filters, source import, tables and bar
charts. Calculations run through the existing native/Web Worker transport. No
dependency or crate was added. Filename context is now available to byte
importers without changing text importer callbacks or introducing shell cases
for the new document kind.

Workbooks containing a sales table open the sales editor, while event-only
workbooks retain event-log import. An explicit **Open workbook event log…**
action in discovery also opens the event worksheet of a mixed workbook as a
separate log, preserving the current source and authored answers. Missing
amounts, missing formula caches, ambiguous sheets/columns and repeated sources
are errors. Recorded zero ratings remain distinct from missing ratings. Study
files retain integer hundredths; ratios and averages are derived. No currency,
customer identity, transaction identity or dated event series is inferred.

The three clean workbooks contain 463 recorded rows: Counter 67, Phone 132 and
Internet 264. Actual Rust import/analysis and an independent Decimal-based read
of the stored worksheet values agree on revenue 6,795.00, costs 5,420.00 and
profit 1,375.00 in source units. There are 460 recorded ratings totalling 1,258,
plus three missing ratings. This gives mean satisfaction 2.7347826, revenue per
record 14.6760259 and profit/revenue 20.235467%. Profit uses the recorded costs
only. All seven groupings reconcile with the global counts and monetary totals.

The typed study and six grouped CSVs were generated under `tp/results/sales`.
Their codec round-trip, CSV headers and source preservation were checked. The
workbooks' event worksheets remain independent of the corrected process CSVs;
sale IDs were not joined to event-log case IDs.

The complete sequential workspace run passed 304 tests (137 application tests,
152 core unit tests and 15 integration tests). Targeted sales tests also cover
filtered worker results, stale exports, duplicate import, undo/redo, source
provenance, exam-mode suppression and typing a multiword location across
separate UI frames. The summary and grouped-table screenshots were inspected.
After that complete run, the explicit mixed-workbook log action was verified
with six targeted discovery-module tests and the translation inventory; the
complete suite was not repeated for that final isolated addition.

Trunk debug builds and headless Firefox worker checks passed at both `/` and
`/app/`. The smoke check covers sales sums, filtered records, zero/missing rating
handling, existing mining/Petri/BPMN/Mermaid jobs, cancellation and invalid input.
Its discovery assertion was updated to the current typed result wire shape.

The final native `cargo build --locked --workspace --all-targets` and `/app/`
Trunk debug build also passed after the explicit event-sheet action was added.
Formatting, whitespace checks and local links in all five changed documentation
pages passed. These checks used local Linux and Firefox; no Astro site assembly,
release build, remote CI matrix, publication or deployment was performed.

## Routing-marker projection in Petri discovery — 19 November 2026

Application discovery now projects whole-name `Vide`/`Empty` routing markers,
including numeric suffixes, to unnamed silent transitions after Alpha
construction. Source text, trace order and the footprint remain unchanged.
Names such as `Empty basket` remain ordinary activities. Generated Petri place
labels replace only marker components with `τ`. Raw teaching `alpha` and strict
manual-net replay retain their previous behavior; application replay explicitly
projects the recorded traces using the result's marker metadata. That metadata
defaults to empty when reading older derived results and survives net rebuilds.
No native document schema changed.

All seven markers in the three existing generated TakeAway nets require silent
routing under conservative local contraction rules: they connect choices to
merges or synchronize parallel branches. The saved files were updated in place
without regenerating their layout or identifiers. Two Counter, three Phone and
two Internet transitions became unnamed/silent; twelve matching generated place
labels were updated. An explicit old/new isomorphism handled boundary labels
that had previously been shortened after generation. JSON comparison confirmed
unchanged positions, IDs, arc endpoints/weights, styles and other properties.
Backups were retained in a temporary directory. All 25 other TP files, including
the CSVs, log documents and sales outputs, remained byte-identical.

All 327 corrected cases replay against the visible projections on the updated
saved nets (Counter 33, Phone 109, Internet 185). Focused regressions cover marker
matching, meaningful similarly named tasks, preparation loops, source/topology
preservation, metadata round-trips and cached worker replay. The complete
sequential workspace suite passed 308 tests: 139 application, 154 core unit and
15 integration tests. Native all-target compilation, the installed WebAssembly
target check, a Trunk `/app/` build and the Firefox worker smoke check passed.
The browser check also verifies source preservation, invisible markers, cleaned
place labels and successful projected replay. Formatting, local documentation
links and `git diff --check` passed. No remote CI or publication was performed.

Pre-commit cleanup removed redundant validation after the checked sales codec,
added TP and synthetic-fixture provenance notes, and preserved supplied CSV
bytes with directory-local Git attributes. The repeated sequential workspace
run passed all 308 unit/integration tests. Its rustdoc step reported unresolved
`calamine` and `quick_xml` crates with reused build outputs; repeating
`cargo test --locked -p op-core --doc -- --test-threads=1` with a fresh
`CARGO_TARGET_DIR` passed (zero collected doctests), without source changes.
Formatting, 165 local file links across nine documentation pages and the staged
patch whitespace check passed.

## TP integration with merged main — 19 November 2026

The TP increment was rebased onto `7da37f7`, retaining the merged writer-version,
multiple-file opening, whole-diagram PNG, shared arrangement, matrix-header and
system-preference changes. Conflict resolution keeps named imports together with
the newer-file compatibility dialog, all test modules and both verification
histories. Language documentation now follows the merged system/browser policy.

Content revision 2 identifies the sales document kind and optional event-log
fields. The seven generated TP `.opx` files now carry a local-build writer header;
their document payload bytes are unchanged from the pre-rebase commit. The other
23 TP files are byte-identical. Historical fixtures remain unmodified.

The explicit workbook-log importer queues every valid selected file and retains
batch errors when a later file succeeds. Its new regression covers two valid
workbooks with an invalid one between them, unchanged authored source/answers
and no repeated open requests on the next frame.

The complete sequential workspace run passed 329 tests: 151 application,
162 core unit and 16 integration tests; the rustdoc step also passed with no
collected doctests. Native and WebAssembly `cargo check --locked -p op-app`
passed after the integration changes. Formatting, 168 local file links across
nine changed Markdown pages and the whitespace check passed. These are local
Linux checks; no remote CI, release build or publication was performed.

## Trash for saved diagrams — 19 November 2026

Saved-diagram cards on the home screen get a bin button and a **Move to trash**
context-menu entry. A **Trash** section lists trashed copies with the days left,
**Restore** and **Delete permanently**. Copies trashed 15 days ago or more are
erased when the library opens. Only the entry metadata changes when trashing,
after pending automatic writes have completed.

`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings` (Rust
1.100), native/WebAssembly checks and `git diff --check` pass; **284 tests passed**
(140 app, 129 core, 15 integration), none ignored.

A native library test writes two copies to a temporary folder. It then checks
that trashing survives a reopening, that the content stays loadable, the
day count (15 → 1 → 0), that restoring persists, that the purge keeps a copy
one second before the deadline and erases both files at the deadline without
touching the other copy, plus permanent deletion and errors for unknown copies.
A UI test, with an isolated library injected into the app, trashes a card
without opening it, then checks the notice and day count, restoring and
permanent deletion of the files. `saved_diagram_card_trash_button.png` and
`saved_diagram_trash.png` were inspected. The browser storage path was
compiled only.

After merging `main` on 20 November 2026, three clippy 1.100 findings in merged
code were fixed. Formatting, clippy, the WebAssembly check and `git diff --check`
passed again; the sequential workspace run passed **353 tests** (156 app,
181 core, 16 integration), none ignored.
The site test rejects a JavaScript placeholder word in rendered pages, so two
sentences merged into `process-discovery.md` were reworded; `bun run build` and
`bun test` (26 tests) then passed locally.

## Moving connection ends and merging flows on deletion — 19 November 2026

Selected arrows show end handles in the BPMN, Petri, EPC/activity/flowchart and
UML editors; dragging one calls the notation's `reconnect` in `op-core`.
Deleting BPMN or flow-diagram elements merges the flows around each deleted
block through the shared `flow::bridges` helper.

`cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings` (Rust
1.100), native/WebAssembly checks and `git diff --check` pass; **289 tests passed**
(140 app, 129 core, 20 integration), none ignored.

Integration tests (`op-core/tests/connections.rs`) cover:

- BPMN: merging a single in/out pair while keeping the name and flow order;
  no merge with two incoming flows or when the merge would duplicate a flow;
  block merges (A→B→C→D without B and C).
- BPMN reconnect: refused start-event targets and self connections leave the
  flow unchanged; moving across pools turns it into a message flow without
  the default marker, and back into a sequence flow.
- EPC: event → event merges are refused, while a function plus the next event
  merge as a block; reconnect follows the same rules.
- Petri: bipartite arcs, inhibitors leaving places and duplicate arcs.
- UML: inheritance cycles are refused.

UI tests drag a selected BPMN flow's end onto another task (one undo step) and
delete a middle task with the Delete key, which leaves a single merged flow. They
also drag a Petri arc's end onto a place (refused, with a status message) and
then onto a transition. `reconnect_handles.png` was inspected.

## TakeAway completion — 20 November 2026

Application discovery now converts silent routing to BPMN gateways with
conservative contractions, retains silent generated boundaries on reverse
conversion and reports omitted place capacities. Bounded observable-language
regressions cover preparation loops, parallel completion, repeated routes and
non-free-choice examples. The source Petri models are not mutated.

The shared BPMN SVG renderer exports complete model geometry, lanes, information
artifacts, routed loops and wrapped escaped labels. Orthogonal routing keeps
clear preferred paths and uses bounded obstacle search for obstructed routes:
256 nodes, 32,768 grid points, 65,536 states and two million intersection checks
are upper limits. Larger/overlapping drafts can retain the preferred fallback;
explicit straight/curved styles are unchanged. A UI regression exposed flow
insertion querying the route after its new shape became an obstacle. Capturing
the target before insertion preserves that editing behavior.

Official OMG BPMN 2.0.2 XSD validation identified process artifacts being emitted
before later sequence flows. Export now writes flow elements before annotations
and associations, preserving identities and diagram interchange. The four final
XML models pass all five normative schemas with libxml2 2.15.2; schema URLs,
SHA-256 fingerprints, file fingerprints and scope are recorded in
[`bpmn-xsd.json`](../tp/results/checks/bpmn-xsd.json). XSD grammar validation does
not certify all BPMN semantics or every external editor.

Sales analysis now derives weekday/customer-type profiles in the existing
worker, displays count/revenue/profit/observed-score curves and comparisons, and
exports series and two explained KPIs. An exact `StructureId` filter can be
scoped by channel; it does not interpret a sales ID as a case/customer ID.
Historical view preferences remain readable and no stored model schema changed.

The reproducible TakeAway example produces 17 outputs: four enriched native,
BPMN XML and SVG models, complementary evidence, case validation, weekday/KPI
CSVs and a filtered scenario export. All 327 observed cases replay through their
channel BPMN, the connected global BPMN and XML round-trips. All four converted
workflow networks return `Sound` within the 20,000-state analysis bound; 15
selected invalid control/cross-channel traces are rejected. Reproducible geometry
checks find no node-body overlaps or unrelated-node crossings for sequence,
association or message routes, using a 3 px inset and excluding endpoints.
Visual review separately checks labels and channel corridors. Inferred role
titles, representative information associations and unmeasured task defaults
are documented; these are not resource-availability or data-execution proofs.

Independent integer/Decimal analysis reconciles eight native summary/series/KPI
CSVs plus the separate scenario subset. It verifies all 463 stored rows against
their own workbooks and resolves local template references. Internet rows 18,
61 and 135 reference cancellation template 7 but retain cached revenue 36 and
cost 28; they also account for all three missing scores. Recorded totals remain
463 / 6,795 / 5,420 / 1,375. A clearly conditional exclusion yields
460 / 6,759 / 5,392 / 1,367; neither actual cash flows nor automatic exclusions
are asserted. All 31 independent analysis outputs reproduce byte-for-byte,
including seven SVG/PNG figure pairs and sixteen CSV tables.

The complete sequential workspace suite passes 351 tests: 154 application,
181 core unit and 16 integration tests; the zero-doctest rustdoc step passes.
An initial unused translation key and a duplicate JavaScript smoke-page binding
were corrected before final verification. The installed WebAssembly target
check, Trunk `/app/` debug build and Firefox worker smoke pass, including seven
weekday points, two customer-type series, exact scenario filtering, zero/missing
scores, existing jobs, cancellation and invalid input.

Final publication checks native/source fingerprints, recorded software results,
all model/CSV checks and the official-XSD file fingerprints before producing the
French PDF/HTML report and ZIP. PDF text extraction, required sections, vector
appendices, ZIP integrity, contained hashes and relative HTML asset paths are
checked. Optional pinned Python publication packages are isolated from Rust and
WebAssembly runtime dependencies. The supplied data and existing log/Petri/sales
study files retain their original bytes. These are local Linux/Firefox checks;
no remote multi-system CI, release build, upload or deployment is claimed.
