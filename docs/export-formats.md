# Export formats

**Status: implemented in File → Export, on desktop and WebAssembly.**

The active document supplies its formats. **Save** and **Save as** still produce
editable `.opx` documents; exports make separate files and do not change the
document, saved state, history or camera. Desktop opens a save dialog; the browser
starts a download. PNG captures the visible diagram where supported.

| Document | File exports |
| --- | --- |
| BPMN | BPMN 2.0 XML (`.bpmn`), Mermaid (`.mmd`), Mermaid Markdown (`.mermaid.md`), PNG. |
| Petri net | PNML (`.pnml`), Mermaid, Mermaid Markdown, PNG. |
| UML classes / use cases | PlantUML (`.puml`), Mermaid, Mermaid Markdown, PNG. |
| EPC / UML activities | SVG, Graphviz (`.dot`), Mermaid, Mermaid Markdown, PNG. |
| Mermaid source | Mermaid (`.mmd`), Mermaid Markdown, SVG when the current preview is ready. |
| Database schema | SQL DDL (`.sql`) for the selected dialect, PNG. |
| Free drawing | SVG, PNG. |
| DMN decision table | DMN XML (`.dmn`), CSV. |
| Event log / process discovery | Alpha analysis report (`.txt`) when current results are ready. |

## Mermaid source and Markdown

Choose **Export Mermaid source** for a plain `.mmd` file. Choose **Export Mermaid
Markdown** for `<document>.mermaid.md`, containing a title and a fenced `mermaid`
code block. The Markdown form also displays conversion notes as text. This uses
the [Mermaid code-block format documented by GitHub](https://docs.github.com/en/get-started/writing-on-github/working-with-advanced-formatting/creating-diagrams).
Other Markdown readers need Mermaid rendering support.

**File → Open** accepts `.mmd`, `.mermaid`, and Markdown containing exactly one
complete fenced Mermaid block. Ordinary prose and other code blocks around it
are ignored. Multiple Mermaid blocks or an unfinished fence produce an error
rather than silently opening a partial diagram. A complete explicit Mermaid
block can contain invalid source: it opens as an editable draft.

Reopening these files creates a Mermaid source document. It does not reconstruct
the originating BPMN, Petri, EPC or UML activity document. Keep `.opx` or the
notation's native export for that purpose.

## What the projections preserve

BPMN exports use a portable [Mermaid flowchart](https://mermaid.js.org/syntax/flowchart.html).
They retain nodes, names, event/task/gateway types as labels/shapes, connections,
branch labels, default-flow labels, pools and lanes as groups. Sequence arrows
are solid, message arrows dashed and associations dashed without an arrowhead.
Mermaid recalculates the layout; documentation, costs, durations, authored styles
and BPMN execution semantics are not carried as structured BPMN data.

Petri projections retain directed arcs and weights, annotate the initial marking,
place capacities and silent transitions, and distinguish inhibitor arcs with a
circle head and threshold label. The result is a visualization without token
simulation. EPC and activity exports preserve nodes, type labels and directed
branches; their flowchart shapes approximate the original notation, including
forks, joins and final nodes.

UML classes reuse the existing typed `classDiagram` conversion, including member
and relation validation. Layout and separate endpoint roles have the documented
[conversion limits](mermaid.md#converting-uml-class-diagrams). Use-case diagrams
use flowcharts: actors become labelled boxes, use cases rounded shapes and
systems groups. Include/extend links are dashed and labelled; generalization is
a labelled directed link. Native actor shapes, UML arrow decorations and endpoint
properties are approximated or omitted.

Conversion notes accompany generated files, as Mermaid comments in `.mmd` and
visible text plus comments in Markdown. Original Mermaid documents preserve
their source, including invalid drafts, without requiring a rendered preview
for either text format. Code fences in a draft cannot prematurely close the
Markdown wrapper, and titles are escaped as text.

Exam mode hides generated Mermaid projections while retaining existing native
diagram exports. Exporting authored Mermaid source/Markdown remains available.

Typed projections and Markdown packaging live in `op-core/src/mermaid/`.
Each notation declares its export descriptors and generates its own projection;
the shell only writes the returned bytes. There are no new dependencies or
document/session schema changes. See [verification](verification.md) for checks
actually run.
