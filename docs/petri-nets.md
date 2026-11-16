# Petri nets

The Petri editor combines graphical modelling, a token game, matrices and
analysis. It supports weighted arcs, inhibitor arcs and place capacities. Save
the net as `.opx`; use PNML when exchanging it with compatible tools.

## Build a first net

1. Create a **Petri net** from **File → New…**, or open an example from its card.
2. Add two places and one transition, then connect **place → transition → place**.
   Ordinary arcs connect different node kinds.
3. In the inspector, give the first place an initial token and the second none.
4. Select an arc to inspect its weight and supported type. Set place capacities
   and transition properties only when your model needs them.
5. Save your editable document before exploring it.

The palette and inspector edit the model. Zoom, fit, grid and inspector visibility
are view controls; they do not change the mathematical net.

## Play tokens

**Tools → Start/stop token game** enters simulation. Fire enabled transitions
to observe the marking. Ordinary input arcs consume their weights and output
arcs produce theirs; inhibitor conditions and capacities also affect enabling.
Use the simulation's marking/reset controls to explore again.

The simulation marking is an exploration state. Distinguish it from the initial
marking authored in the place inspector. A disabled transition can be expected:
check its input tokens, inhibitor thresholds and output capacities.

## Inspect matrices and exercises

**Tools → Matrices (Pre, Post, C)** opens matrix inspection. For ordinary arcs,
`C = Post − Pre` and a firing-count vector satisfies `M′ = M + C · σ` when the
sequence can actually fire. The equation alone does not establish enabling;
inhibitors and capacities require their additional rules.

Matrix exercises let you enter answers, check them and use the available help.
Matrix copy/export controls include CSV and LaTeX. In exam mode, assistance and
generated answers are hidden; the authored document remains editable.

## Analyse the net

Use **Tools → Analysis & invariants** for supported structural properties,
invariants, deadlocks and workflow-net checks. Reachability, states and graph
views expose the explored markings. Multiple views can share the same document
and history while keeping their own cameras.

Always read the result status and exploration limits. A truncated state space
cannot establish absence of an unexplored behaviour. An unavailable invariant
result can reflect arithmetic or resource limits. Workflow soundness applies to
the required workflow-net structure, rather than arbitrary nets.

Analysis runs in background jobs; editing its input invalidates outdated results.
Large nets may need a smaller exploration limit or a simpler example. Browser
jobs have time and transfer limits; consult [troubleshooting](troubleshooting.md).

## Convert and export

**Tools → Convert Petri net to BPMN** creates a separate BPMN document within
the conversion's supported mapping. **Convert to Mermaid** creates a flowchart
projection with notes, without token simulation.

**File → Export** provides PNML, Mermaid source, Mermaid Markdown and visible
canvas PNG. Keep `.opx` for editable state and read the
[projection limits](export-formats.md). Generated analysis and conversions are
unavailable when the application-wide exam preference suppresses assistance.
