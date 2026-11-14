# Open Process

<img src="crates/op-app/assets/icons/open-process.svg" alt="Open Process" width="96" height="96">

**Model processes, explore their behavior and design diagrams in one free,
open-source application.** Open Process combines BPMN, Petri nets, process
mining, UML, decision tables, database design, free drawing and Mermaid.

Built with Rust and egui/eframe, it runs on **Windows, macOS and Linux**, and in
modern browsers through **WebAssembly**. English is the default language;
French and Spanish are also available.

[Open the browser app](https://open-process.linv.dev/app/) ·
[Download desktop builds](https://github.com/linventif/open-process/releases) ·
[Getting started](docs/getting-started.md) · [Contribute](CONTRIBUTING.md)

## What you can do

| Workspace | Available features | Scope and details |
| --- | --- | --- |
| **BPMN 2.0** | Tasks, events, gateways, pools/lanes, validation, automatic layout, BPMN XML with diagram interchange, conversion to Petri nets, cost/duration evaluation. | Evaluation covers structured, acyclic XOR/AND models. This is a supported subset, not complete BPMN execution or simulation. |
| **Petri nets** | Weighted and inhibitor arcs, capacities, token game, matrices, invariants, reachability/coverability, structural properties, workflow-net soundness, PNML exchange. | Exploration limits and diagnostics affect which conclusions can be drawn. |
| **Process discovery** | Text/CSV/XES logs, footprint matrix, step-by-step Alpha algorithm, trace replay, exercises, conversion of discovered nets to BPMN. | Educational and analysis workflows; additional miners and conformance metrics remain planned. |
| **UML classes and use cases** | Classifiers and relationships, actors and system boundaries, properties, diagnostics, selection, alignment/distribution, copy/paste, PlantUML export. | PlantUML export does not imply PlantUML import. |
| **UML activities and EPC** | Typed graphical editors, connection rules, structural diagnostics, SVG and Graphviz DOT exports. | Initial implementations; see [supported features and limits](docs/notations-ajoutees.md). |
| **DMN decision tables** | Typed inputs/outputs, rules, UNIQUE/FIRST/ANY/COLLECT policies, evaluation, DMN XML and CSV exports, single-table XML import. | A limited FEEL test subset; no full FEEL interpreter or decision requirements graph. [Details](docs/notations-ajoutees.md). |
| **Relational databases** | Tables, columns, primary/foreign keys, indexes, validation, shared SQL preview and DDL export for PostgreSQL, MySQL/MariaDB and SQLite. | Physical schema design; Merise conceptual/logical transformations and SQL reverse engineering remain planned. [Details](docs/base-de-donnees-ddl.md). |
| **Free drawing** | Shapes, attached lines/arrows, endpoint snapping, sticky notes, text, freehand strokes, colors, sketch mode, SVG export. | A native/web whiteboard; no Excalidraw format exchange or network collaboration. [Details](docs/dessin-libre.md). |
| **Mermaid** | Source editor, live preview, flow/sequence/class/state/ER templates, source import/export and SVG export. | Uses a Rust renderer with its own syntax coverage, rather than the official JavaScript engine. [Details](docs/mermaid.md). |

Graphical editors share navigation, grid and zoom controls, an inspector and
responsive layout. Documents support undo/redo, automatic local saving, editable
`.opx` copies, session restoration and multiple docked views where supported. The home screen
and **File → New…** provide searchable document cards and examples by type.
**File → Export** lists the active document's formats, including PNG capture
where available. PNG captures the visible diagram region.

The project is under active development. It does not claim complete standards
conformance or feature parity with other modelling products. Check each guide's
limits and any diagnostics or truncated-analysis notices before using results.

### Process discovery (event logs)

Import text, CSV or XES event logs to inspect direct succession, causality,
parallelism and independence in a footprint matrix. The Alpha walkthrough shows
each construction step, previews the discovered Petri net and replays traces.
Manual exercises let you fill relations and construction sets, check answers
or request a completed step. You can open a discovered net in the Petri editor
or derive a BPMN diagram. A command-line example prints the walkthrough:

```sh
cargo run --locked -p op-core --example alpha -- log.txt
```

Petri matrices also have an exercise mode and support CSV/LaTeX copying. BPMN
process evaluation separately computes trace costs and expected duration for
supported structured models, using equiprobable XOR choices and no waiting.
Sequences add durations and parallel branches use their maximum. Evaluation is
capped at 256 choice scenarios; displayed traces are limited to 512 words, with
an explicit notice when shortened.

## Get started

Use the [hosted app](https://open-process.linv.dev/app/) or a build from
[GitHub Releases](https://github.com/linventif/open-process/releases).
Open an example from a document card, explore its editor, then save your work as
`.opx`. The [getting started guide](docs/getting-started.md) covers the basic workflow.

Desktop distributions include:

- **Windows x64:** an executable with the application icon.
- **macOS Apple Silicon and Intel:** archives containing **Open Process.app**;
  extract and move the app to Applications.
- **Linux x64:** an archive with the binary, icon, desktop launcher and
  `install.sh`; run `sh install.sh` from the extracted directory for a user installation.

Standalone Linux/macOS binaries are also published. See the
[icon and packaging guide](docs/icone.md) for package details.

Desktop versions automatically create **Documents/Open Process** and keep editable
copies there. The web app automatically keeps diagrams in the browser's local
storage. Closed diagrams remain available under **Saved diagrams** on the home
screen. **Save/Save as** still writes a chosen file on desktop or downloads an
`.opx` copy in the browser. Keep separate copies for sharing and backup; see
[data and storage](docs/data-and-storage.md).

<a id="running"></a>

## Build and run

### Desktop

Install a stable Rust toolchain meeting the minimum version in
[Cargo.toml](Cargo.toml) (currently Rust 1.95), plus your platform's native build
tools. Linux also needs windowing development libraries; CI installs
`libxkbcommon-dev` and `libgtk-3-dev` on Ubuntu.

```sh
git clone https://github.com/linventif/open-process.git
cd open-process
cargo run --locked --release -p op-app
```

You can pass files on the command line:

```sh
cargo run --locked --release -p op-app -- my-process.bpmn my-net.pnml
```

### Browser application

Install Trunk and the WebAssembly target, then serve the Rust application:

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk --version 0.22.1
cd crates/op-app
trunk serve --release
```

Open the URL printed by Trunk. `trunk build --release` produces the standalone
application in `crates/op-app/dist/`.

### Website and combined web build

The Astro/TypeScript website uses Bun 1.4.0 and Node.js 24:

```sh
cd website
bun install --frozen-lockfile
bun run dev
```

`bun run build` builds the website and compiles the application into
`website/dist/app/`. `bun run preview` serves the combined output.
English is served at `/`, French at `/fr/`, Spanish at `/es/`, and the app at
`/app/`. See [website/README.md](website/README.md) for tooling and hosting details.

## Development and checks

```sh
cargo test --locked --workspace -- --test-threads=1
cargo check --locked -p op-app
cargo check --locked -p op-app --target wasm32-unknown-unknown
git diff --check
```

Run application/UI tests sequentially, as in CI. For focused checks, website
validation and contribution requirements, use [CONTRIBUTING.md](CONTRIBUTING.md).
Historical local validation records are in [docs/verification.md](docs/verification.md);
they are not a claim that every check was rerun against the current checkout.

```text
crates/op-core/   Typed models, algorithms, geometry, conversions and formats
crates/op-app/    Native/web egui application, editors and platform services
website/         Astro presentation site and combined WebAssembly build
scripts/         Packaging, icons and independent validation helpers
docs/            User guides, architecture, compatibility and verification records
```

Notations register their document factories, commands, views and formats through
[composition.rs](crates/op-app/src/composition.rs). The shell consumes that registry;
models and histories remain typed. Start with the [documentation index](docs/README.md)
and [current architecture](docs/architecture-actuelle.md) to explore the implementation.

## Automatic releases

Pushes to `main` run the shared CI workflow: tests and native release builds on
Linux, Windows and macOS, plus the website/WebAssembly build and Bun tests.
After validation, the release workflow builds four native targets and publishes
one GitHub Release tagged `main-<run number>-<short commit SHA>`. `v*` tags are
also supported. `main-*` builds and prerelease tags are published as prereleases.
Only a `vMAJOR.MINOR.PATCH` tag becomes the latest stable release. Each release
includes `SHA256SUMS`; the application's About dialog displays a build identifier
from CI/Cloudflare, or `local` for an unidentified local build.

CI also audits Rust/Bun dependencies and executes generated SQLite schemas.
Dependency audits run weekly, and action revisions are pinned to commit hashes.

Website download links use the latest stable GitHub release metadata. Cloudflare Pages
builds and deploys the combined site through its own Git integration.
See [the release workflow](.github/workflows/release.yml),
[website deployment](website/README.md) and [CHANGELOG.md](CHANGELOG.md).

## Roadmap

The product direction prioritizes data modelling inspired by Looping and process
modelling/documentation/simulation inspired by Bizagi Modeler, followed by broader
modelling tools. These are development goals, not delivery commitments.

Remaining areas include:

- Merise conceptual and logical models, transformations to SQL and SQL reverse engineering.
- Broader BPMN coverage, subprocess navigation, documentation and resource modelling.
- Process simulation with durations, resources, calendars and scenario comparisons.
- Extended DMN/FEEL, EPC and UML activity support.
- More process miners, conformance metrics and advanced Petri-net analysis.
- Consistent editing/export coverage across notations and, later, shared repositories.

The [coverage matrix](docs/parite-looping-bizagi.md) distinguishes implemented,
partial and proposed features. The [diagram catalogue](docs/catalogue-diagrammes.md)
describes possible future document families.

## Community

English is the project's default language for documentation and source text.
Issues and pull requests in French are also welcome. Contributions can include
translations, examples, documentation, bug reports, accessibility improvements and code.

- [Contributing](CONTRIBUTING.md): setup, conventions, checks and pull requests.
- [Support](SUPPORT.md): questions, bugs and feature suggestions.
- [Code of Conduct](CODE_OF_CONDUCT.md): community expectations and private contact.
- [Security](SECURITY.md): private vulnerability reporting and update scope.
- [Governance](GOVERNANCE.md): decisions, reviews and releases.
- [Changelog](CHANGELOG.md): curated changes and published build history.

## Credits and license

The UI uses [Inter](https://rsms.me/inter/) by Rasmus Andersson and
[Patrick Hand](https://fonts.google.com/specimen/Patrick+Hand) by Patrick Wagesreiter,
under the SIL Open Font License 1.1. See their bundled
[Inter](crates/op-app/assets/fonts/Inter-LICENSE.txt) and
[Patrick Hand](crates/op-app/assets/fonts/PatrickHand-OFL.txt) notices.

Open Process is dual-licensed under [MIT](LICENSE-MIT) OR
[Apache-2.0](LICENSE-APACHE), at your option. Third-party assets retain their own licenses.
