# UML use-case diagrams

Use-case documents describe actors, system boundaries and the goals actors
pursue. They share the graphical UML editor but retain their own element and
relationship rules.

## Build a use-case diagram

1. Choose **UML use cases** in the new-document picker. Its **Examples** menu
   offers a starter model.
2. Place a system boundary and give it the system's name.
3. Add use cases inside that boundary. Name them as goals, for example
   **Place order** or **Track shipment**.
4. Add actors and connect them to the use cases they participate in.
5. Choose include, extend or generalization relationships where appropriate.
   Select each relationship to inspect its direction and properties.
6. Read the diagnostics and save the document as `.opx`.

An include arrow points from the including use case toward the included one.
An extend relationship describes additional behaviour extending a base use case.
Use these relations deliberately rather than treating every relationship as an
ordinary actor association.

## Edit the model

The floating palette provides selection, element creation and relationship
choices. The inspector edits names and relationship properties. Actors, use cases
and system boundaries support edge/corner resizing with the selection tool.
Shared alignment/distribution and clipboard controls help organize larger models.

Grid, zoom and fit adjust the current view. Undo/redo applies to completed model
edits. When editing text, keyboard input belongs to the focused text field. Refer
to [interface controls](interface.md) for shared docking and compact layouts.

## Save, export and convert

**File → Export** provides PlantUML, Mermaid source, Mermaid Markdown and PNG.
PlantUML output is an export format; importing arbitrary PlantUML is not supported.
PNG captures the visible diagram area.

**Tools → Convert to Mermaid** opens a separate flowchart projection. Actors
become labelled boxes, use cases rounded shapes and systems groups. Include and
extend links remain labelled, dashed relationships. UML-specific actor shapes,
arrow decorations and endpoint properties are approximated or omitted.

Mermaid's reverse native conversion supports class diagrams only. A use-case
flowchart does not reconstruct a native use-case document. Keep `.opx` for
continued editing; see [format limits](export-formats.md).

## Scope

Use-case diagrams document participation and goals. They do not execute scenarios
or replace textual requirements, activity flows or BPMN process evaluation.
Use [UML activities](uml-activities.md) for control flows or
[BPMN](bpmn.md) for process models.
