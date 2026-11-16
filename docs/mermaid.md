# Mermaid source, builder and preview

**Status: implemented in the editor registry.**

The **Create with Mermaid** card is available on the home screen and in the new
document picker. It opens a text document with a graphical preview. The inspector
has **Source**, **Builder** and **Templates** tabs; in narrow views, **Inspector**
opens it above the preview.

## Features

- Monospaced Mermaid source editing, with automatic preview after a 300 ms pause.
- 35 templates, also available as card examples, covering the 31 families in the
  [Mermaid syntax catalogue](https://mermaid.js.org/intro/) reviewed on
  16 November 2026, with five separate C4 levels. Selecting a template replaces
  the source; undo restores the previous code.
- A form builder with 75 basic statement patterns across the listed families. It adds, updates
  or removes a selected statement without regenerating the rest of the source.
- 27 templates have local previews. Eight newer families have source/form editing
  and text exports, with an explicit unavailable-preview message.
- Shared preview pan, zoom and fit controls.
- `.mmd` and `.mermaid` source import, detected by Mermaid headers. Drafts with
  recognized headers can open even if they contain syntax errors.
- `.mmd` and `.mermaid.md` exports even when invalid; SVG export when the preview for the
  current source is ready.
- Markdown import with exactly one complete Mermaid fence. Incomplete or multiple
  Mermaid blocks are rejected; invalid source inside a complete fence remains editable.
- `.opx` saves, session restoration and history shared across views. Name/source
  persist; each view keeps its camera, inspector visibility and tab. Computed
  errors/images are not saved.
- English, French and Spanish interface text, with English as the default.

## Native graphical editor work

The requested objective is dedicated graphical editors for all Mermaid families,
with movable elements, domain properties and relations. This objective is **not
complete**. The form builder below is useful source tooling, not completion of
that request.

A first native flowchart module is available through **Draw a flowchart** and
**Convert to flowchart builder** from Mermaid source. It reuses the shared
canvas/inspector, with graphical creation, movement, connection, resizing,
selection, properties, undo/redo, `.opx` saves and SVG/DOT/Mermaid exports.
Its initial typed model contains process rectangles, decisions, terminators,
circle connectors and preparation hexagons with labelled directed arrows.

Flowchart ↔ Mermaid preserves these node kinds, names and labelled connections.
Positions/sizes are regenerated with a diagnostic. Import runs in the existing
native worker/Web Worker; edits or exam mode cancel an outdated request. Round Mermaid rectangles map
to terminators with a presentation diagnostic. Styles/annotations are diagnosed;
groups, advanced shapes, decorated/dashed links, self-links and parallel links
are rejected without opening a partial destination. The original source/native
document remains unchanged. This is an initial integration, not full flowchart
syntax coverage.

UML class conversion remains available separately. Other native families,
including sequence timelines, state hierarchy, ER semantics, chart manipulation,
Gantt dependencies and Kanban card moves, still require dedicated implementation.
The existing database editor does not substitute for a generic Mermaid ER model,
and free drawing does not substitute for typed notation editors.

Remaining native work is tracked by interaction/model needs, not by template count:

| Workstream | Families still needing native behavior |
| --- | --- |
| Typed graphs and containers | State hierarchy, ER entities/cardinalities, requirements, C4 levels, architecture, blocks and Mermaid use-case syntax. |
| Ordered interactions | Sequence, ZenUML, event modeling and Git graph operations. |
| Hierarchy editing | Mind map, Ishikawa, TreeView and treemap parent/child/value rules. |
| Time and dependencies | Gantt, journey and timeline. |
| Numeric/chart manipulation | Pie, XY, radar, quadrant, Sankey, packet ranges, Venn and Wardley. |
| Boards/domains | Kanban column/card moves and Cynefin domain assignment. |

Each family requires a typed model, storage validation, canvas interactions,
history, examples and explicit conversions with diagnosed unsupported syntax.
The [notation extension guide](ajouter-une-notation.md) remains the implementation
contract; this table records unfinished work rather than available editors.

## Switching source and builder

Select **Builder** in the inspector to choose an element type and fill in its
fields. **Add element** inserts the generated statement. The default insertion
point is the end of the source; a line number and indentation level allow
insertion into a group, lane, tree branch or Kanban column. Insertion before the
Mermaid header is rejected. **Editable statements** lists recognized statements
by source line. Select one, edit its fields, then choose **Apply changes** or
**Remove statement**. Form input stays unapplied until that action. Each applied
addition, change or removal is a single document undo step.

**Source** and **Builder** operate on the same Mermaid source and history.
Switching tabs alone leaves every source byte, the saved state and the document
revision unchanged. Comments, YAML frontmatter, directives, styles, Unicode,
unknown syntax and surrounding line endings are preserved. Applying unchanged
values also preserves the original encoding. A selected statement carries an
exact source guard: if another view edits it, the old form cannot overwrite the
new source. Select the element again to refresh its fields.

This is a form builder with an automatically laid-out preview. It does not add
draggable native graphical editors for every Mermaid family. Its forms recognize
a conservative set of statement patterns, including their generated syntax;
other valid/imported syntax remains available in **Source**. Removing a statement
removes that declaration only: remaining Mermaid references can still imply its
nodes. The builder does not claim complete Mermaid grammar or semantic
validation. Invalid drafts remain saveable and editable.

| Mermaid family | Basic form content | Local preview |
| --- | --- | --- |
| Flowchart | Nodes, decisions, labelled arrows. | Yes |
| Sequence | Participants, messages, replies. | Yes |
| Classes | Classes, attributes, operations, relations. | Yes |
| State | States, transitions, initial/final transitions. | Yes |
| Entity relationship | Entities with an attribute, relationships. | Yes |
| Pie | Labels and values. | Yes |
| Gantt | Sections, dated/dependent tasks and durations. | Yes |
| User journey | Sections, tasks, scores and actors. | Yes |
| Timeline | Sections, periods/events. | Yes |
| Sankey | Weighted links. | Yes |
| Quadrant | Labelled points and coordinates. | Yes |
| XY | Numeric X axis, bar/line series. | Yes |
| Radar | Labelled curves and numeric values. | Yes |
| Packet | Integer bit ranges and labels. | Yes |
| Mind map | Indented ideas. | Yes |
| Git graph | Commits, branches, checkout and merge. | Yes |
| Requirements | Requirements, elements and typed relations. | Yes |
| C4 context/container/component/dynamic/deployment | Appropriate nodes and relations, deployment nodes. | Yes, all five templates |
| Block | Blocks and arrows. | Yes |
| Architecture | Groups, services, icons and port connections. | Yes |
| Kanban | Columns/cards, positioned through indentation. | Yes |
| Treemap | Groups and weighted leaves. | Yes |
| ZenUML | Actors and messages. | Yes |
| Swimlanes | Lanes, nodes, arrows. | Unavailable |
| Use cases (`usecase-beta`) | Actors, use cases, arrows. | Unavailable |
| Event modeling | Numbered, typed frames. | Unavailable |
| Venn | Sets and intersections. | Unavailable |
| Ishikawa | Indented causes. | Unavailable |
| Wardley | Components, coordinates, dependencies. | Unavailable |
| Cynefin | Domains and items. | Unavailable |
| TreeView | Indented file/folder labels. | Unavailable |

The eight unavailable previews reflect the bundled Rust renderer's coverage,
not an invalid-source diagnosis. These sources still open as Mermaid documents,
can use forms, save as `.opx` and export as `.mmd`/Markdown. Their starter syntax
follows the official documentation; it has not been executed against the official
JavaScript parser in the local verification recorded for this change. Source and
form availability does not imply native-builder conversion support.

## Exporting other diagram builders

**Tools → Convert to Mermaid** in BPMN, Petri, EPC, UML activity, UML class and
UML use-case builders opens a separate Mermaid document directly. Conversion
notes are shown and stored as comments in its source. The native document stays
open and unchanged; edits to either copy are independent.

**File → Export** in these builders also offers Mermaid source and Mermaid
Markdown. Classes use `classDiagram`; the process/control-flow and
use-case builders use flowchart projections. The Markdown file contains a title,
conversion notes and a fenced Mermaid block. Original native formats remain
available; `.opx` remains the editable save format. See the
[format catalogue and projection limits](export-formats.md).

## Converting UML class diagrams

In the UML class builder, select **Convert to Mermaid** in the **Tools** menu or
the canvas context menu. A new Mermaid document opens with editable
`classDiagram` source and its preview. Its source can then be exported as `.mmd`
or `.mermaid.md`. Direct exports are also available in the UML builder's File
menu. The original UML document remains open and unchanged.

For the reverse direction, open or import a Mermaid `classDiagram` and select
**Convert to UML classes** in the Mermaid toolbar or **Tools**. This creates a
separate editable UML document with its own history and save state. This command
is disabled for other Mermaid families; UML use-case diagrams are not supported
by the reverse conversion. Their forward command instead creates a flowchart
projection. This reverse command is an explicit class-diagram mapping. Basic flowcharts have
a separate native conversion described above; other families remain unsupported.

The conversion preserves class names, duplicate display names, interfaces,
enumerations, abstract classes, attributes, operations, relation labels and UML
multiplicities. It maps association, directed association, aggregation,
composition, generalization, realization and dependency in both directions.
Reversed Mermaid arrows keep their semantic direction and swap endpoint
multiplicities as needed. Unannotated realization targets are inferred as
interfaces, with a diagnostic.

Mermaid's method/attribute distinction uses parentheses. Builder operations must
contain `(`, and attribute lines must not; ambiguous member compartments are
rejected rather than silently moved. Member whitespace is normalized. Stable
aliases and Mermaid entity escapes allow duplicate names, Unicode, quotes and
reserved syntax without treating user text as new declarations.

Positions and sizes are regenerated, using a deterministic nonoverlapping grid
when opening a UML document. Separate UML endpoint roles have no dedicated
mapping in the emitted Mermaid syntax and are omitted with a warning. In the
reverse direction, endpoint text that is not a valid UML multiplicity is kept
as a role, also with a diagnostic.

Namespace frames, notes, custom stereotypes, themes, styles, links, callbacks and
document annotations have no equivalent in this UML builder. The conversion
reports these omissions or approximations. Inheritance cycles, invalid builder
relations, two-ended decorated links, lollipop interfaces, undirected dashed
links and incomplete class/namespace blocks are rejected. A failed conversion
opens no partial destination and leaves the source intact.

The typed conversion functions live in
[`mermaid/conversion.rs`](../crates/op-core/src/mermaid/conversion.rs).
Each editor declares its own command and emits the existing `HostEvent::Open`
and diagnostic messages. No editor catalogue, document schema, dependency or
shell-specific notation branch is added. Existing BPMN/Petri conversions remain
available through their own modules.

## Implementation and limits

The [typed model and SVG rendering](../crates/op-core/src/mermaid.rs) stay in
`op-core`. The `mermaid` JSON codec, model version 1, uses the existing `.opx` v2
envelope with `name` and `source`. It changes no historical type or session schema.
Source is a draft: Mermaid validity is not required to save or reopen it.

The [form operations](../crates/op-core/src/mermaid/builder.rs) and
[family catalogue](../crates/op-core/src/mermaid/catalogue.rs) live in `op-core`.
Form drafts and source guards are transient per-view interactions; no second
document model is stored in a view or workspace. The persisted builder tab is an
optional view preference, defaulting to Source when older workspaces lack it.

The [editor](../crates/op-app/src/mermaid_editor.rs) declares factories, commands,
examples, imports and exports, then registers in `composition.rs`. It reuses
`DiagramLayout`; snapping is an optional common configuration capability, absent
from this preview because it has no draggable elements.

[mermaid-svg 0.7](https://docs.rs/mermaid-svg/0.7.0/mermaid_svg/)
provides Rust parsing, layout and SVG generation.
The `resvg` rasterizer is limited to `op-app` and uses bundled Inter fonts. These
dependencies support native/WebAssembly rendering without an embedded browser or
external service.

The [preview job](../crates/op-app/src/mermaid_editor/preview.rs) runs in a native
thread or the existing Web Worker. Composition distinguishes `mermaid.preview`
from historical Petri requests. Editing source abandons the preceding job, and a
stale image/SVG cannot replace the current result. Rasterization caps the longest
side at 2,048 pixels; exported SVG remains vector. Native cancellation ignores
the result but cannot interrupt a call already inside the library; web cancellation
terminates the worker.

This is a Rust implementation of Mermaid with its own rendering and syntax
coverage, rather than the official JavaScript engine. Repository tests cover the
27 preview-capable templates; the eight remaining families explicitly report
that preview is unavailable. Complete syntax, directive, theme and interaction
parity is not guaranteed. Raw engine diagnostics may be in English. Diagrams are
edited through source or basic statement forms; preview nodes cannot be dragged.
The builder and preview each accept up to 256 KiB of source. Larger drafts remain
saveable/exportable and editable in Source.
The Gantt today marker is disabled in derived rendering to avoid depending on
the system clock and keep the same behavior on the web. Exported source retains
its original directives.

Checks actually performed are recorded in [verification](verification.md).
