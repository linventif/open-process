# BPMN processes

The BPMN editor models processes using tasks, events, gateways, pools and lanes.
It provides graphical editing, validation, automatic layout, BPMN XML exchange
and conversions. It does not implement every BPMN construct or a full execution
engine. See [costs and durations](bpmn-evaluation.md) for the evaluation subset.

## Create a process

1. Open **File → New…** or the tab bar's **+**, then choose **BPMN**. Use the
   card's **Examples** button to start with a working model.
2. Place a start event, tasks and an end event using the palette.
3. Connect the elements in execution order. Select a connection to edit its
   label and properties in the inspector.
4. Add gateways when execution branches or joins. Choose the appropriate task,
   event or gateway type from its inspector or context menu.
5. Add pools and lanes when the model needs participants or responsibilities.
   Keep sequence flows within their participant; use message flows for messages
   between participants.
6. Read the validation inspector and resolve reported issues before interpreting
   a conversion or evaluation.

For a first example, create **Start → Receive order → Check order → End**.
Then insert an exclusive gateway after the check and label its outgoing branches
**Accepted** and **Rejected**. Connect both branches to suitable end paths.

## Edit and arrange

Select an element to edit its name and available properties. Use the selection
tool to move elements or resize their bounding-box edges and corners. Connections
follow their attached shapes. Pool headers move their contents together.

Dragging a pool's outer edges resizes it; an internal lane separator redistributes
height between adjacent lanes. Shrinking stops at contained elements and minimum
sizes. Escape cancels an active gesture; completed edits use shared undo/redo.

**Auto arrange** in the configuration bar or **Tools** lays out the process from
left to right and fits the view. It preserves node sizes and pool/lane membership;
pool and lane bounds may grow to accommodate content. With **Snap to grid**, node
centres and container geometry use the editor's grid. Arrangement is undoable.

## Validate and analyse

Validation identifies supported structural and modelling issues. A clean
validation result does not certify complete BPMN standard conformance.

**Tools → Convert BPMN to Petri net** opens the converted model separately.
Use the [Petri editor](petri-nets.md) for token play and net analysis. Inspect
conversion diagnostics: a projection is not a guarantee that every BPMN execution
detail is represented. **Tools → Convert to Mermaid** creates a separate,
automatically laid-out flowchart with conversion notes.

The cost/duration workflow uses the same BPMN model and task properties. Read
[its assumptions](bpmn-evaluation.md) before using its numbers.

## Files and limits

Save `.opx` to retain an editable Open Process document. **File → Open** imports
supported BPMN XML; **File → Export** offers BPMN 2.0 XML with diagram
interchange, Mermaid source, Mermaid Markdown and PNG capture.

Mermaid retains the visible process structure but omits structured execution,
cost, duration, documentation and style data. PNG captures the visible region.
Keep `.opx` or BPMN XML for subsequent modelling. See the
[format catalogue](export-formats.md) and [shared controls](interface.md).

## Copying and duplicating selections

Use **Ctrl/Cmd+C**, **X** and **V** to copy, cut and paste selected elements,
**Ctrl/Cmd+D** to duplicate, or hold **Alt** when starting a drag. **Ctrl/Cmd+A**
selects diagram nodes and pools. These actions also appear in **Edit** and the
canvas context menu. Copies retain properties and internal flows and receive
fresh identifiers, including pool/lane identifiers. Copying a pool includes its
lanes and contained nodes; copying a lane includes its contained nodes in a new
pool with that lane. Paste accepts BPMN selections from another BPMN document.

Alt-drag duplication and movement undo together; **Escape** cancels the gesture.
The original nodes and pool remain in place. Focused text fields keep their own
clipboard shortcuts. See [shared canvas interactions](interface.md) for native
and browser menu-paste behavior.
