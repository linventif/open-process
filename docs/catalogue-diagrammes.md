# Proposed diagram catalogue

**Product proposals, ordered after the confirmed Looping and Bizagi Modeler
priorities.** Current status is documented in the [notation guides](notations-ajoutees.md)
and [coverage matrix](parite-looping-bizagi.md).

The goal is connected tools: design data, describe processes, specify decisions,
simulate and publish. These are proposals for Open Process, not claims that every
family exists in Looping or Bizagi.

## Immediate priority: data and processes

| Type / view | Purpose | Status / order |
| --- | --- | --- |
| Merise conceptual model (MCD) | Entities, associations, cardinalities and conceptual rules. | Next Looping workstream. |
| Relational logical model (MLD) | Tables, keys and dependencies derived from MCD. | After MCD. |
| Physical model / ERD (MPD) | DBMS-specific types, indexes and constraints. | Initial relational editor with [DDL generation](base-de-donnees-ddl.md). |
| Data dictionary | Structured attributes, domains, definitions and usages. | Alongside MCD; a table rather than a canvas. |
| Merise organizational flow model (MOF) | Actors and information exchanges. | Looping goal. |
| Process map | Organize/access project processes. | Modeler goal. |
| Value chain | Strategic, primary and support activities. | Modeler goal. |
| RACI matrix | Responsibilities for activities/processes. | Modeler goal; tabular view. |
| Scenario/result table | Compare simulation alternatives. | Modeler goal; tabular view. |
| DMN decision table | Executable typed input/output rules. | Initial version integrated. |
| DMN DRD | Decision, knowledge and data dependencies. | Extension after initial tables. |
| EPC | Event/function chains and logical operators. | Initial version integrated. |
| UML activity | Actions, decisions, merging and parallelism. | Initial version integrated. |

MCD, MLD and MPD should share transformation/provenance services while retaining
distinct models/invariants. RACI, dictionaries and results reuse the shell without
artificial canvas controls.

## Extensions after priority features

[Free drawing](dessin-libre.md) is available for sketches, shapes, notes and
freehand strokes; it does not replace the typed families below. The
[Mermaid source editor](mermaid.md) also previews selected diagram families, but
is separate from a dedicated typed graphical editor for each notation.

| Type | Proposed use | Dependency / distinction |
| --- | --- | --- |
| UML sequence | Messages between actors/components. | Timeline, lifelines, fragments, activations. |
| UML state | Object/order lifecycle. | Hierarchy, transitions, guards, actions. |
| UML component | Software contracts/dependencies. | Provided/required interfaces. |
| UML deployment | Applications, nodes, environments. | Infrastructure resources. |
| UML package | Domains and dependencies. | References to existing elements. |
| UML object | Instances and their links. | Links to typed classes. |
| UML communication | Numbered exchanges in object networks. | May share an interaction model with sequence. |
| UML composite structure | Component parts and ports. | Containers and typed connections. |
| UML timing | States and temporal constraints. | Dedicated time axis. |
| Interaction overview | Compose interactions. | Cross-diagram references/navigation. |
| UML profile | Domain stereotypes/properties. | Metamodel and validation need scoping. |
| Organization chart | Teams, roles, responsibilities. | Reuse the planned resource directory. |
| DFD | Data transformation/circulation. | Levels and data stores. |
| Merise conceptual processing (MCT) | Conceptual events/operations. | Additional proposal; do not attribute to Looping without evidence. |
| Merise organizational processing (MOT) | Operations, actors, places, timing. | Depends on an actor directory. |
| Flowchart | Simple algorithms/procedures. | Separate from BPMN/UML activity. |
| C4 context | People, system, neighboring systems. | References shared across C4 levels. |
| C4 container | Applications, services, stores. | Hierarchy and typed links. |
| C4 component | Components inside a container. | Navigation to UML/deployment. |
| C4 dynamic | Component interaction scenarios. | Ordered exchanges. |
| C4 deployment | Containers in environments. | Links to deployment views. |
| Mind map | Frame problems and ideas. | Trees, folding, automatic layout. |
| WBS | Deliverable/work-package breakdown. | Hierarchy and package IDs. |
| Gantt | Tasks, dependencies, milestones. | Dedicated timeline, not a free graph. |
| PERT / CPM | Dependencies and critical path. | Algorithms with explicit duration assumptions. |
| Ishikawa | Explore causes. | Cause trees/categories. |
| Customer journey | Stages, touchpoints, feelings, difficulties. | Multidimensional table/matrix. |
| Story map | Usages and product increments. | References to use cases. |
| Kanban | Project work items. | Operational view, not a UML diagram. |

## Visual Paradigm horizon

After the priority work, investigate ArchiMate, SysML, CMMN, SoaML, BMM, capability
maps, requirements/traceability, API models, wireframes and screen flows. Also
compare code generation/reverse engineering, model transformations, impact
analysis and shared repositories. These capabilities require more than symbols.

The [official feature catalogue](https://www.visual-paradigm.com/features/) is a
reference for that breadth, not a claim of parity or adoption of every feature.

## Entry requirements for a notation

Each type needs a use case, typed model, validation, registered factory, at least
one example, `.opx` saves, history, translations and a relevant exchange format.
Graphical editors reuse `DiagramLayout`; timelines/tables use appropriate content
layouts in the same shell. Follow [the extension guide](ajouter-une-notation.md).
