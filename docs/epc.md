# EPC diagrams

The EPC editor represents event-driven process chains using events, functions
and AND/OR/XOR connectors. Its current validation is structural; it does not
simulate an EPC or implement the complete eEPC notation.

## Create a chain

1. Choose **EPC** in **File → New…**, or open an example from the card.
2. Place an event such as **Order received**, followed by a function such as
   **Check order**, then a resulting event.
3. Connect nodes by dragging a connection or selecting its two endpoints.
4. Add an AND, OR or XOR connector where the chain branches or joins.
5. Select nodes and flows to edit names and labels in the inspector.
6. Review diagnostics and save the diagram as `.opx`.

Direct event-to-event and function-to-function connections are rejected.
Use the appropriate intermediate element rather than adding a connector solely
to bypass a diagnostic. Branch labels document the process; they are not
executable expressions.

## Edit and validate

Use single selection, Shift selection or rubber-band selection to organize the
model. Selected elements can move, resize, duplicate with **Ctrl/Cmd+D** or be
deleted. The selection tool can grab bounding-box edges/corners without a prior
selection. Escape cancels an active gesture; undo/redo records completed edits.

The shared configuration bar controls zoom, fit, snapping and inspector visibility.
Compact windows group these options into a menu. See [interface controls](interface.md).

Diagnostics check supported node input/output degrees and connection rules.
They do not establish transitive event/function alternation through connectors
or the full choice rules after events. A diagram with no local errors is not a
certification of complete EPC semantics.

## Export and share

**File → Export** provides vector SVG, Graphviz DOT, Mermaid source, Mermaid
Markdown and visible canvas PNG. **Tools → Convert to Mermaid** opens a separate
flowchart document. Conversion retains nodes, type labels and directed branches
while approximating notation shapes.

Keep `.opx` for continued native editing. SVG/DOT/Mermaid exports do not imply
that arbitrary files in these formats can be imported as EPC models. EPML
import/export, organizational extensions and execution simulation are not
implemented. Read [coverage](notations-ajoutees.md#epc) and
[export limits](export-formats.md) for details.
