# Mermaid editor and preview

**Status: implemented in the editor registry.**

The **Create with Mermaid** card is available on the home screen and in the new
document picker. It opens a text document with a graphical preview. The inspector
has **Source** and **Templates** tabs; in narrow views, **Inspector** opens it
above the preview.

## Features

- Monospaced Mermaid source editing, with automatic preview after a 300 ms pause.
- Five templates, also available as card examples: flowchart, sequence, class,
  state and entity-relationship. Selecting a template replaces the source;
  undo restores the previous code.
- Shared preview pan, zoom and fit controls.
- `.mmd` and `.mermaid` source import, detected by Mermaid headers. Drafts with
  recognized headers can open even if they contain syntax errors.
- `.mmd` source export even when invalid; SVG export when the preview for the
  current source is ready.
- `.opx` saves, session restoration and history shared across views. Name/source
  persist; each view keeps its camera, inspector visibility and tab. Computed
  errors/images are not saved.
- English, French and Spanish interface text, with English as the default.

## Converting UML class diagrams

In the UML class builder, select **Convert to Mermaid** in the toolbar or the
**Tools** menu. A new Mermaid document opens with editable `classDiagram` source
and its preview. Its source can then be exported as `.mmd` with the existing
Mermaid export command. The original UML document remains open and unchanged.

For the reverse direction, open or import a Mermaid `classDiagram` and select
**Convert to UML classes**, again in the toolbar or **Tools**. This creates a
separate editable UML document with its own history and save state. This command
is disabled for other Mermaid families; UML use-case diagrams are not supported
by this conversion either. This is an explicit class-diagram mapping, not a
general conversion between every registered notation.

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
five template families; complete syntax, directive, theme and interaction parity
is not guaranteed. Raw engine diagnostics may be in English. Diagrams are edited
through source; preview nodes cannot be edited graphically.
The Gantt today marker is disabled in derived rendering to avoid depending on
the system clock and keep the same behavior on the web. Exported source retains
its original directives.

Checks actually performed are recorded in [verification](verification.md).
