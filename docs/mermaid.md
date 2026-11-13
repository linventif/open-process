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
