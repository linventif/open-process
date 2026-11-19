# UML class diagrams

Use the UML class editor to describe classifiers, their members and their
relationships. It supports classes, interfaces, enumerations and abstract
classes, with graphical editing, diagnostics and PlantUML/Mermaid export.

## Create a class model

1. Choose **UML classes** in **File → New…**, or open the card's example.
2. Place two classifiers, such as `Customer` and `Order`.
3. Select a classifier and use its inspector to edit its name, attributes and
   operations. Choose the classifier kind appropriate to the model.
4. Open the relationship picker in the floating palette. Choose a relation type,
   then connect its source and target.
5. Select the relationship to edit its label and supported endpoint properties,
   including multiplicities and roles.
6. Check diagnostics and save as `.opx`.

Keep attributes and operations in their separate compartments. An example
attribute is `name: String`; an operation such as `total(): Money` includes
parentheses. Those distinctions also matter when converting to Mermaid.

## Relationships and layout

The editor supports association, directed association, aggregation, composition,
generalization, realization and dependency. Choose the relation according to its
meaning and read any invalid-endpoint diagnostics.

Use the selection tool for movement and edge/corner resizing. Select several
elements to use the alignment and distribution actions in the inspector or
canvas context menu. Clipboard and duplication commands are available through
the shared editing controls. Text fields keep their own clipboard and undo
behaviour while being edited.

View configuration controls grid snapping, zoom and inspector visibility. The
relationship picker belongs to the floating palette; there is no separate UML
business toolbar. See [shared controls](interface.md).

## Exchange and Mermaid conversion

Save `.opx` to keep the native editable model. **File → Export** provides
PlantUML (`.puml`), Mermaid source, Mermaid Markdown and visible canvas PNG.
PlantUML export does not imply PlantUML import.

**Tools → Convert to Mermaid** opens a separate `classDiagram` source document.
In a Mermaid class diagram, **Convert to UML classes** performs the reverse
supported mapping. The documents have independent histories; subsequent edits
do not synchronize them.

The mapping preserves supported classifier kinds, member compartments and
relationship types. Positions and sizes are regenerated. Separate endpoint roles,
namespaces, annotations and unsupported syntax can be omitted, approximated or
rejected with diagnostics. Inheritance cycles and invalid relationships are not
silently converted. Read the detailed
[conversion guide](mermaid.md#converting-uml-class-diagrams).

## Scope

This is a class modelling tool, rather than a source-code generator or a complete
UML/XMI interchange environment. [Use cases](uml-use-cases.md) and
[activities](uml-activities.md) have their own document types. Keep the native
file when an exported projection omits properties you need to edit later.
