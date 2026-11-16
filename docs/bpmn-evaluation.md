# BPMN costs and durations

The BPMN evaluation workflow computes trace costs and expected duration for a
supported structured, acyclic process. It uses the BPMN model and task cost/duration
properties; it is not a resource, calendar or queue simulation.

## Prepare the model

1. Open the **BPMN costs/durations** workflow or an existing BPMN document.
2. Build a connected process with the supported structured XOR and AND branches.
   Start with a sequential example before adding branching.
3. Select each task and enter its **Cost CAi** and **Duration TAi** properties.
   Use consistent units throughout the model.
4. Open the evaluation inspector and read structural diagnostics before using
   its output.
5. Edit task values or the process to compare scenarios. Results update from
   the current model; outdated background results are discarded.

## Calculation assumptions

Sequential task durations add. Concurrent AND branches execute in parallel, so
their elapsed duration is the maximum branch duration, while their task costs
still contribute to the execution cost. XOR alternatives are equiprobable.
The evaluation assumes no waiting time for resources or scheduling.

For example, two sequential tasks taking 2 and 3 time units take 5 in total.
If the same tasks are independent parallel branches, their elapsed duration is
3. Task costs of 10 and 20 total 30 when both execute. Equal-probability XOR
alternatives lasting 2 and 6 give an expected duration of 4 under these assumptions.

These examples explain the arithmetic; the diagram must still satisfy the
evaluator's structural rules. Labels on XOR branches do not become configurable
probabilities or executable conditions.

## Read the result

Use the displayed trace costs and expected duration to compare supported process
structures. A result is specific to the model, values and assumptions above.
Read any rejected-structure or shortened-trace message. Choice exploration is
capped at 256 scenarios and displayed traces at 512 words.

Cycles, unstructured flows and unsupported BPMN constructs can prevent evaluation.
Do not replace a diagnostic with a manually assumed result. For token behaviour,
convert a compatible process to a [Petri net](petri-nets.md) and inspect the
conversion notes instead.

## Saving and exports

Save `.opx` for the editable process and its authored task properties. Standard
BPMN document exports remain available through **File → Export**. Mermaid
projections describe the visible flow but do not retain costs and durations as
structured BPMN data. See [BPMN editing](bpmn.md) and [formats](export-formats.md).

Exam mode suppresses generated evaluation assistance. Resource capacities,
calendars, stochastic arrivals, waiting queues and full process simulation remain
outside the implemented evaluator. [Product goals](parite-looping-bizagi.md)
describe future simulation directions separately.
