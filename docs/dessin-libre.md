# Native and web free drawing

**Implemented: a Rust/egui whiteboard inspired by Excalidraw workflows, within the
common shell and layout.** It runs natively and in WebAssembly; the official
Excalidraw React component is not embedded.

## Usage

Open **File → New…**, the **+** button or **Ctrl/Cmd+N**, search for “whiteboard”
and choose **New free drawing**. **Draw freely** on the home screen creates the
same document. The card's **Examples** button provides a starter drawing.

The vertical palette on the left offers select, hand, rectangle, ellipse, diamond,
line, arrow, freehand, text and sticky note tools. Drag to draw an element. The
palette reuses the floating panel from other graphical editors and scrolls
vertically when needed. Icon buttons have accessible names/tooltips and share the
BPMN selection cursor. A click with a shape tool creates a default-sized element.
The pencil remains active after a stroke; other drawing tools return to selection.

- Click to select; Shift+click adds/removes elements.
- Drag empty space for rubber-band selection; Shift preserves prior selection.
- Drag a selection to move it. A bottom-right handle resizes a single element,
  including strokes and arrows.
- The inspector edits multiline text, dimensions, stroke/fill colors, thickness,
  font size and sketch mode. Double-click opens it and focuses text editing.
- Style changes affect selected elements and future creations. Without selection,
  they configure the next element.
- **Ctrl/Cmd+D** duplicates, Delete removes, arrow keys nudge by one unit (ten
  with Shift), and **Ctrl/Cmd+A** selects all. Drawing shortcuts apply over the
  canvas when no text field is being edited.
- **Escape** cancels the active gesture; completed changes use shared undo/redo.
- Hand, Space+drag, middle/right drag and scrolling navigate the view. Zoom/fit
  use the common controls.

Tool shortcuts: **V** select, **H** hand, **R** rectangle, **E** ellipse,
**D** diamond, **L** line, **A** arrow, **P** freehand, **T** text, **N** note.
**F** fits the drawing.

The shared configuration bar contains **Show grid**, **Snap to grid**, zoom and
inspector controls. Snapping is off by default; pencil points remain free even
when it is enabled. Options move into the compact menu in narrow views.

## Documents and exports

Drawings save as `.opx` v2 with type `whiteboard`, model version 1. The model
contains elements, IDs, geometry, text, colors and styles without changing other
notations' schemas.

Sessions separately store camera, snapping, grid, inspector and next-element style.
Multiple panels share one document/history with independent cameras/selections.
For free drawing, geometry is semantic content: moving a shape also changes the
semantic revision.

**File → Export** provides:

- **SVG:** the whole drawing as vector geometry/editable text, without selection
  or grid. Regular outlines and a sans-serif font do not reproduce the exact
  canvas sketch irregularities or handwriting font.
- **PNG:** the visible canvas with its theme/grid settings, without selection handles.

## Boundaries and limits

| Responsibility | Implementation |
| --- | --- |
| Model, geometry, selection, validation | [whiteboard.rs](../crates/op-core/src/whiteboard.rs) |
| SVG export | [whiteboard/svg.rs](../crates/op-core/src/whiteboard/svg.rs) |
| Tools and inspector | [whiteboard_editor.rs](../crates/op-app/src/whiteboard_editor.rs) |
| Gestures and keyboard | [interaction.rs](../crates/op-app/src/whiteboard_editor/interaction.rs) |
| Rendering | [render.rs](../crates/op-app/src/whiteboard_editor/render.rs) |
| Sessions, commands, registration | [session.rs](../crates/op-app/src/whiteboard_editor/session.rs) |
| Layout | [editor/layout.rs](../crates/op-app/src/editor/layout.rs) |

The module registers in `composition.rs` without additional dependencies, a web
layer or changes to the shell's catalogue.

This initial version has no network collaboration, images, symbol libraries,
persistent groups, rotation, cross-document element clipboard, shape-attached
arrows or `.excalidraw` exchange. Arrows are independent strokes. Text content is
retained but may be hidden by an undersized box.

Limits are 20,000 elements and 200,000 points per document, with at most 20,000
sampled points per pencil gesture. Validation rejects invalid geometry/styles.
Creation/duplication exceeding limits is rejected before replacing the document.
See [recorded checks](verification.md#native-and-web-free-drawing).
