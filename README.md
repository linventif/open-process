# Open Process

**One free, open-source tool for process modelling and analysis** — BPMN 2.0 diagrams,
Petri nets, incidence matrices, invariants, reachability graphs and workflow-net soundness —
instead of juggling Bizagi, Visual Paradigm, WoPeD, PIPE and friends.

Written in Rust with [egui](https://github.com/emilk/egui): a single code base that runs
**natively on Windows, macOS and Linux** and **in any modern browser** (WebAssembly).

> 🇫🇷 **En bref** — Open Process remplace la collection d'outils utilisés en cours de
> *process intelligence* : modélisation BPMN (pools, lanes, passerelles, événements…),
> réseaux de Petri (jeu de jetons, matrices Pre/Post/C, invariants P/T, graphe
> d'accessibilité/couverture, bornitude, vivacité, réversibilité, blocages, soundness),
> conversion BPMN ↔ réseau de Petri, découverte de processus (matrice des relations →, ||, #,
> algorithme Alpha pas à pas, vérification des mots), import/export BPMN 2.0, PNML et XES.
> Natif + navigateur.

## Features

### BPMN 2.0 editor
- Start / intermediate (catch & throw) / end events with triggers (message, timer, error, signal, conditional, terminate)
- Tasks (user, manual, service, script, send, receive, business rule), sub-processes
- Gateways: exclusive (XOR), parallel (AND), inclusive (OR), event-based, complex
- Pools & lanes, sequence flows (conditions, default flows), message flows, data objects, data stores, annotations
- Quick-append pad (Bizagi / bpmn.io style), drop an element on a flow to insert it, orthogonal routing
- Live model validation (missing start/end, dangling elements, implicit splits/joins, unlabeled XOR branches…)
- **Import / export BPMN 2.0 XML with diagram interchange** (Bizagi, Camunda, bpmn.io, Signavio, Visual Paradigm…)
- **Convert to a Petri net** (workflow net) to check soundness
- **Process evaluation**: enter each activity's cost CAi and duration TAi in the inspector's
  **Evaluation** tab. See possible traces, their cost equations and values, and the mean
  execution-time equation for equiprobable XOR choices with no waiting. Sequences add
  durations; parallel AND branches use their maximum. Choices inside parallel branches
  are expanded before averaging. Includes the course's **Process P** under Examples.
  Supports acyclic, structured XOR/AND diagrams (sub-processes are atomic activities).
  Costs and durations default to zero and are saved in `.opx` and BPMN XML extensions.
  At most 256 choice scenarios are evaluated; trace display is limited to 512 words,
  with an explicit notice when shortened, while the calculated values remain exact.

### Petri net editor & analysis
- Places, transitions, weighted arcs, inhibitor arcs, place capacities, silent (τ) transitions
- **Token game**: click enabled transitions, random / automatic play, step back, firing sequence σ
- **Matrices**: Pre (W⁻), Post (W⁺), incidence C, markings — copy as **LaTeX** or CSV
- **Reachability / coverability graph** (Karp–Miller, ω), interactive graph view, state table
- Properties: bounded (k-bound per place), safe, live (L4), quasi-live, reversible, deadlock-free (+ shortest firing sequence to each deadlock)
- **P- and T-invariants** (minimal semi-positive, Farkas algorithm), with conservation equations `M(p1) + M(p2) = k`
- Structural classes: ordinary, pure, state machine, marked graph, free-choice, extended free-choice, workflow net
- **Workflow-net soundness** (van der Aalst)
- **PNML import / export** (WoPeD, PIPE, ProM, TINA…)

### Process discovery (event logs)
- Paste or open an event log: `Case 1, Task A` lines, words like `ABCD ACBD EF`, one trace per line, CSV or **XES** (ProM)
- Direct successions `>`, causality `→`, parallelism `||`, independence `#`
- **Relation (footprint) matrix**, as drawn in class, copyable as LaTeX
- **Alpha algorithm step by step**: TW, TI, TO, XW, YW, PW (places `P(A,BE)`, iw, ow), FW, α(W)
- Discovered Petri net preview, **word check** (replay of every trace), soundness verdict
- One click to open α(W) as a Petri net or to **deduce the BPMN diagram** (Petri net → BPMN, with AND/XOR gateways)
- **Step-by-step mode**: write the relations, click the matrix cells and type TW…FW yourself; every step has *Check* (missing / extra items, hints) and *Fill for me*; your net is built from your own answers and replayed
- Petri matrices **exercise mode**: fill Pre / Post / C by hand and check them
- Command line: `cargo run -p op-core --example alpha -- log.txt` prints the full write-up

### Everywhere
- **Split views**: drag a tab onto the edge of any pane to split it (e.g. BPMN on the left, Petri matrices on the right); matrices, analysis, states and the reachability graph can each get their own pane
- **Rubber-band selection** (drag on empty space), Shift+click, Ctrl+A, move / align / distribute / delete a group; Space+drag or right/middle drag to pan
- **Drag & drop** elements from the floating palette onto the canvas (or onto a flow to insert them)
- **Link styles**: straight, right angles (90°) or curved; **hand-drawn "sketch" mode** (Excalidraw-like)
- **Export as image (PNG)** of diagrams and matrices; matrices also as **CSV** and LaTeX
- Tabs, undo/redo, zoom/pan, snap to grid, light & dark themes, drag-and-drop files
- Native `.opx` (JSON) format; session automatically restored (local storage on the web)
- Built-in examples: order process, mutual exclusion, producer/consumer, dining philosophers

## Running

### Desktop

```sh
cargo run --release
# or open files directly
cargo run --release -- my-process.bpmn my-net.pnml
```

Linux needs the usual windowing libraries (`libxkbcommon`, X11 or Wayland).

### Browser

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk
cd crates/op-app
trunk serve --release      # http://127.0.0.1:8080
```

`trunk build --release` produces a static site in `dist/` that can be hosted anywhere
(the included GitHub Actions workflow publishes it to GitHub Pages).

## Project layout

The product website lives in [`website/`](website/). It is a standalone static
site with English/French content, application screenshots and GitHub release
downloads. See [`website/README.md`](website/README.md) for local preview and
Cloudflare Pages deployment settings.

```
crates/
  op-core/   GUI-independent library (models, algorithms, file formats) — native & wasm
    petri.rs      Petri net model, firing rule, Pre/Post/C matrices
    analysis.rs   reachability/coverability graph, properties, invariants, soundness
    bpmn.rs       BPMN model, validation, flow routing
    bpmn_xml.rs   BPMN 2.0 XML + DI import/export
    pnml.rs       PNML import/export
    convert.rs    BPMN ↔ Petri net translations
    mining.rs     event logs, relation matrix, Alpha algorithm, trace replay
    evaluation.rs traces, activity costs and expected process execution time
  op-app/    egui/eframe application (desktop + web)
```

Run the tests with `cargo test --workspace`.

## Roadmap

- [ ] SVG / PNG export of diagrams
- [ ] French / English UI
- [ ] Multi-selection, copy/paste, alignment guides
- [ ] Siphons & traps, home markings, coverability-based liveness hints
- [ ] Coloured / timed / stochastic Petri nets
- [ ] More process mining: directly-follows graph with frequencies, Alpha+ (short loops), inductive / heuristic miner, conformance metrics (fitness, precision)
- [x] Process evaluation: enumerate the traces of an acyclic structured BPMN model, cost and duration formulas
- [ ] BPMN simulation (token flow, resources, durations) and KPIs
- [ ] DMN decision tables, EPC, UML activity diagrams

Contributions are welcome!

## Credits

The UI uses the [Inter](https://rsms.me/inter/) typeface by Rasmus Andersson,
licensed under the SIL Open Font License 1.1 (see `crates/op-app/assets/fonts/Inter-LICENSE.txt`).
The sketch mode uses [Patrick Hand](https://fonts.google.com/specimen/Patrick+Hand) by Patrick Wagesreiter,
also under the SIL OFL 1.1 (see `crates/op-app/assets/fonts/PatrickHand-OFL.txt`).

## License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE), at your option.
