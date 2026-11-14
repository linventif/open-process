# Interface zones and components

**Status: implemented interface references and separate component-library proposals.**

Open the [annotated SVG](diagrams/interface-zones.svg) to refer to a zone by name
or ID, e.g. “Z10: change the Style button hover.” The editable vector document
has an identified group for each of twelve zones.

![Annotated Open Process layout](diagrams/interface-zones.svg)

The diagram collects optional slots to give them names; a notation need not show
every component. Proportions/colors explain structure rather than propose moving
panels or adopting a new product theme.

## Shared vocabulary

| ID | Name | Role and owner |
| --- | --- | --- |
| Z01 | Main navigation / menu bar | Home, File, Edit, View, Tools, Help, language and theme. Shell: `app.rs::menu_bar`. |
| Z02 | Document tabs / docking | Open documents/views, closure and workspace splitting. `DockBehavior` in `app.rs`, tree in `dock.rs`. |
| Z03 | Document view / dockable panel | Container in the docking layout. Multiple views may share a model/history. |
| Z04 | Business action bar | Notation-specific actions such as UML relations/alignment. `DiagramLayout::tools`. |
| Z05 | Tool palette | Active tool: select, connect, create. Notation content with `canvas.rs` primitives. |
| Z06 | Canvas / drawing area | Model rendering, background grid, selection, movement and camera. `DiagramLayout::canvas`. |
| Z07 | Inspector | Properties, validation or analysis. Frame/responsiveness: `DiagramLayout::inspector`. |
| Z08 | Inspector tabs | Switch Z07 content, e.g. Properties / Validation / Evaluation. `DiagramLayout::inspector_tabs`. |
| Z09 | Status / summary | Counts, diagnostics, simulation state. `FooterStatus` in `DiagramLayout::configuration`. |
| Z10 | View configuration | Style, Show issues, Snap to grid, zoom, inspector visibility. `DiagramLayout::configuration`. |
| Z11 | Zoom controls | Part of Z10: decrease, percentage/reset, increase, fit. `canvas::zoom_controls` and `zoom_buttons`. |
| Z12 | Compact Options menu | Groups Z10 options in narrow views; the same configuration component. |

Z09 and Z10 share **one bottom bar**, with Z11 inside Z10. Z02 changes the document
or view; Z08 changes inspector content. The Z06 grid is the canvas background,
while **Snap to grid** controls movement snapping.

Petri places use a model-space radius of 8.25 units, a diameter of 16.5 before
zoom, following the later 50% increase. Tokens scale with the radius; arc anchors,
creation previews and hit testing use it too. Labels retain their text size.
Historical radius changes in the verification record refer to earlier revisions.

The Z02 **+**, **File → New…** and **Ctrl/Cmd+N** open the same searchable,
scrollable, responsive grid of registry document cards. It reuses home components.
Choosing a card creates a document; cancellation/Escape closes the picker.
Creation preserves the originating tab group in split layouts.

Each document card has an **Examples** button at the bottom right with only that
type's examples. Home and picker use the same component. Cards without examples
and workflow launchers retain their primary action. Choosing an example closes
the picker and preserves the tab group; opening the menu creates no blank document.

On home, **Resume your work** precedes **New document** when documents are open.
Cards show a PNG preview, a one-line name with full hover text, type/icon and
unsaved status. Click/keyboard activation resumes the same document/history.
The responsive grid uses one to four columns.

Home content stays centered with a maximum width of 1120 logical pixels. Its
vertical scroll area spans the available window width, keeping the scrollbar
at the right edge rather than beside the centered cards.

Previews are local captures of displayed content, reduced to 320 × 180 pixels
with aspect ratio preserved. They do not modify models/cameras and restore from
a separate bounded cache. Missing previews fall back to notation icons.

Notation/workflow icons come from descriptor functions rather than card position.
The free-drawing palette uses accessible icon buttons, floating left of the canvas
like BPMN/Petri, with vertical scrolling when needed. Graphical editors reuse the
shared selection cursor from `icons.rs`.

BPMN **Task type**/**Gateway type** context menus and corresponding inspector lists
use standard buttons with decorative icons and highlight the active type. Task
markers and gateway drawings match the canvas. Empty-space creation menu actions
also have icons. Full buttons remain clickable, with accessible labels; icons
follow text color for theme/selection.

Z01 groups exports in **File → Export**, populated by the active session, with
PNG where capture is supported. Exports are disabled on home.

For BPMN, Z10 offers **Auto arrange** alongside style; it moves into Z12 in compact
views and is also available under Tools. It lays out left to right, preserves
pool/lane membership and fits the active view. Rows align node centers across
different shape sizes. When **Snap to grid** is enabled, those centers and the
pool/lane geometry use the editor's 10-unit grid; disabling it keeps free
coordinates. Node sizes are preserved. Flow endpoints are recalculated from the
placed shapes: side midpoints on rectangles, vertices on gateways and the
boundary of circular events, rather than rounded away from their shapes.
Geometry is persistent/undoable; pool widths/lane heights may grow to fit
content and snapping clearance. Invalid geometry and ambiguous header
containment are diagnosed before mutation.

These references cover BPMN, Petri, UML, EPC, database and [free drawing](dessin-libre.md).
Text/table views such as Mining must not simulate a canvas to fit the layout.
DMN shares actions/inspector with scrollable table content and no artificial zoom.
Database SQL is a secondary selectable-text view sharing the relational document.
[Mermaid](mermaid.md) uses an inspector for source/templates and a graphical preview.

## Reusable layout and appearance

[editor/layout.rs](../crates/op-app/src/editor/layout.rs) owns editor arrangement.
Notations provide palettes, business actions, properties and specific options
rather than copying configuration bars, zoom, spacing or breakpoints.

Thresholds depend on **available view width**, including docking splits:

- Inspector on the right above 760 px; hidden by default below it. Explicitly
  opening it in a narrow view places a bounded panel above the canvas.
- Options grouped in Z12 below 720 px of bar width.
- Inspector button also grouped in Z12 below 360 px.

[theme.rs](../crates/op-app/src/theme.rs) owns reusable appearance.
`toolbar_style` shares transparent idle backgrounds, neutral hover, accent-tinted
selection, corner/glyph consistency and keyboard focus outlines across navigation,
business actions, inspector tabs and configuration. Form fields retain their
usual style. `DiagramLayout` determines placement; `theme.rs` determines appearance.
A future component kit would integrate into that common layer.

## Rust component libraries: historical investigation

The app uses egui/eframe 0.37; see [Cargo.toml](../crates/op-app/Cargo.toml).
Controls are drawn by egui, not HTML elements accepting Tailwind/DaisyUI classes.
The following was a **13 November 2026 investigation**, not a current compatibility
certification. None of these kits was integrated or tested in the repository.

| Library | Potential use | Compatibility recorded at the time |
| --- | --- | --- |
| [egui-shadcn](https://docs.rs/crate/egui-shadcn/latest) | shadcn/ui-inspired buttons, fields, tabs, cards and variants. | Documented 0.5.0 depended on egui 0.33; adaptation needed for 0.37. |
| [egui-material3](https://docs.rs/crate/egui-material3/latest) | Material Design 3 components/themes. | Investigated 0.0.9 also depended on egui 0.33. |
| [shadcn-egui](https://docs.rs/shadcn-egui/latest/shadcn_egui/) | A separate shadcn/ui-inspired theme/component/font/icon project. | The [inspected manifest](https://github.com/oetiker/shadcn-egui/blob/main/Cargo.toml) used egui 0.34; 0.37 compatibility was not validated. |

Before selecting a kit, pin a version and exercise menus, docking, keyboard focus,
translations and native/WebAssembly builds. Types from different egui versions
are not directly interchangeable. Adoption remains a separate proposal to compare
with maintaining the small shared theme.

## Historical checks for the layout work

The SVG was parsed as XML, its twelve zone groups/unique IDs checked, then
rasterized and inspected. `app::tests::screenshots` generates local light/dark
hover captures for Home, File, Style and Inspector in `target/screenshots/`.
Recorded Rust/document checks are in
[verification](verification.md#interface-zones-and-style). See its later English
documentation section for checks of the translated diagram.
