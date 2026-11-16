# Process discovery from event logs

The event-log workspace uses the Alpha algorithm to explain how recorded traces
lead to a Petri net. It includes a footprint matrix, construction steps, manual
exercises, trace replay and commands to open the discovered model.

## Prepare and import a log

Create an **Event log / process discovery** document, paste a log into its source
area or open a supported text, CSV or XES file. The card's **Examples** menu is
the simplest starting point.

For a compact teaching log, each character is an activity:

```text
ABCD
ACBD
AED
```

For named activities, write a trace per line:

```text
register -> check -> pay
register -> reject
```

A CSV can supply one event per row using case and activity columns:

```csv
case,activity
1,Register
1,Check
1,Pay
2,Register
2,Reject
```

The order of records within a case defines its trace. Quote names containing
separators or line breaks. Malformed quoting and incomplete records are rejected.
Text lines starting with `#` are comments. XES uses its recorded trace/event
structure. Inspect the parsed activities and traces before studying the result.

## Follow Alpha

The footprint shows direct succession (`>`), causality (`→`), reverse causality,
parallelism (`||`) and independence (`#`) between activities.

Follow the walkthrough from activity, initial and final sets through candidate
pairs `XW`, maximal pairs `YW`, places, transitions and arcs to the discovered
net `α(W)`. Its source and sink places delimit the workflow. Compare the sets
with the footprint rather than interpreting the final picture alone.

Manual exercises let you enter relations and construction sets, check answers,
request hints or fill a step. Copy controls provide supported text, CSV and
LaTeX representations. Exam mode removes computed assistance and generated
results without changing the stored event log.

## Replay and continue modelling

Replay the recorded traces on the discovered net. Read failures such as a
transition not being enabled or remaining tokens after completion. A soundness
result requires the workflow-net conditions and completed analysis; an
undecided/truncated result is not a proof of soundness.

**Open Petri net** creates an editable document for simulation and
[further analysis](petri-nets.md). **Deduce BPMN** creates the corresponding
supported process projection. These are separate documents, not synchronized
views of the original log. **File → Export** provides the Alpha write-up when
the current results are ready. Save `.opx` to preserve your log and authored work.

## Limits and interpretation

Alpha is sensitive to log completeness and its modelling assumptions. A log
contains observed behaviour, not proof that all possible behaviour was recorded.
Short loops, noise and incomplete observations can make its result unsuitable.
Additional miners and general conformance metrics are not implemented.

Discovery is bounded to 128 activities, 100,000 events and 2 MiB of source, with
additional construction limits. An incomplete construction is reported as an
error. Simplify oversized teaching examples; preserve the original data outside
the app before changing it. See [troubleshooting](troubleshooting.md).
