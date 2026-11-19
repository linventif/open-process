# UML activity diagrams

The activity editor models control flows with initial nodes, actions, decisions,
merges, forks, joins, activity finals and flow finals. It has typed connection
rules and structural diagnostics, rather than executable UML activity semantics.

## Build an activity

1. Choose **UML activities** in the new-document picker or open its example.
2. Add an initial node, one or more actions and an appropriate final node.
3. Connect them with control flows and name the actions in the inspector.
4. Add a decision for alternatives and a merge to reunite them. Add a fork/join
   pair to describe concurrent control flows.
5. Select outgoing decision flows and write their guards, such as `[accepted]`
   and `[rejected]`.
6. Inspect diagnostics and save the native document as `.opx`.

For example, model **Initial → Review request → Decision**, with accepted and
rejected action paths leading to a merge and then an activity final. Guard text
explains the branches but is not evaluated.

## Connections and editing

Initial nodes cannot have incoming flows; final nodes cannot have outgoing flows.
Additional node-specific degree checks report malformed local structures.
Choose an activity final to describe completion of the activity, or a flow final
when documenting termination of one control flow.

Use the shared selection, rubber-band selection, movement, edge/corner resizing,
duplication and deletion controls. Nodes and flow labels can be edited in the
inspector. Two-click and drag connection gestures are supported. Zoom, fit and
grid snapping use the common configuration bar; edits use shared undo/redo.
See [interface controls](interface.md).

## Exports and conversion

**File → Export** provides SVG, Graphviz DOT, Mermaid source, Mermaid Markdown
and visible canvas PNG. **Tools → Convert to Mermaid** creates an independent
flowchart projection with conversion notes. Forks, joins and final-node shapes
are approximations in that projection.

Keep `.opx` for editable activity semantics. Exporting an SVG, DOT or Mermaid
file does not establish an import path back to the original model. See
[formats](export-formats.md).

## Scope

Guards are documentary text. Object flows, partitions, pins, interruptible regions,
structured activities, simulation and XMI exchange are not implemented. Structural
validation is narrower than complete UML conformance. Refer to
[detailed activity coverage](notations-ajoutees.md#uml-activities) for the current
implementation boundaries, or use [BPMN](bpmn.md) for process modelling and its
separate cost/duration workflow.
