# Process discovery from event logs

The event-log workspace uses the Alpha algorithm to explain how recorded traces
lead to a Petri net. It includes a footprint matrix, construction steps,
generated exercises with step-by-step answers, exam answer fields, trace replay
and commands to open the discovered model.

## Prepare and import a log

Create an **Event log / process discovery** document, paste a log into its source
area or open a supported text, CSV, XES or XLSX file. The card's **Examples** menu is
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

### Spreadsheet imports

**File → Open** checks XLSX workbooks for a sales table first. A matching workbook
opens the [sales analysis workspace](#analyze-recorded-sales). An event-only
workbook still opens as an event log when exactly one worksheet contains
recognized case and activity columns. That sheet is imported as CSV.
To open the event sheet of a workbook that also contains sales, create an
**Event log / process discovery** document and choose **Open workbook event log…**
in its input panel. The selected sheet opens as a separate document; the current
authored log is preserved.
Selecting several workbooks opens each valid event log as a separate document;
an invalid workbook is reported without discarding the other selected logs.
Additional event columns, including actors, remain in the working source, while
discovery uses each case's ordered activity names. Resource-to-lane mapping is
not implemented by event-log import. Sales analysis remains a separate document;
sales identifiers are not assumed to identify event-log cases.

CSV import removes entirely empty trailing spreadsheet columns before starting
discovery. Populated columns and event order are preserved. This allows padded
logs to fit the analysis transport without increasing its limits. An import note
identifies prepared sources; opening a file does not rewrite the original.
Malformed records still fail instead of being repaired or omitted. Import a
corrected source as a new document when the supplied data changes.

## Follow Alpha

The footprint shows direct succession (`>`), causality (`→`), reverse causality,
parallelism (`||`) and independence (`#`) between activities. Matrix columns and
the row-header column widen to fit activity names; names longer than about
180 points are shortened with an ellipsis and shown in full on hover. The exam
answer matrix uses the same layout.

Follow the walkthrough from activity, initial and final sets through candidate
pairs `XW`, maximal pairs `YW`, places, transitions and arcs to the discovered
net `α(W)`. Its source and sink places delimit the workflow. Compare the sets
with the footprint rather than interpreting the final picture alone.

**Normalize process boundaries** adds generated common entry and exit activities
before Alpha construction. Their Petri transitions are silent, so replay still
uses the original recorded traces. This supports, for example, cases whose final
activities can occur in either order. Generated activities are identified in the
construction and write-up. The option is enabled for structured CSV/XLSX imports;
new teaching logs and historical documents keep classic Alpha by default. It is
saved in `.opx` and can be undone like other document edits.

Recorded routing markers named `Vide` or `Empty`, optionally followed by a
numeric suffix (such as `Vide1`, `Vide_2` or `Empty-3`), become unnamed silent
transitions in the discovered Petri net. Matching ignores case and surrounding
whitespace; ordinary task names such as `Empty basket` are preserved. Generated
place labels use `τ` in place of marker names.

This projection happens after Alpha construction, so the source, footprint and
recorded trace order remain intact. Removing the markers before discovery can
change the inferred choices and preparation loops. The routing connections are
therefore retained as silent transitions, and replay uses each trace's visible
projection. This rule is independent of **Normalize process boundaries**.

Copy controls provide supported text, CSV and LaTeX representations. Exam mode
removes computed assistance and generated results without changing the stored
event log. Its fields record relations, matrix cells and construction sets; they
are saved with the log and are not checked.

## Practise with generated exercises

The **Exercise** section of the input panel generates practice logs. Choose a
difficulty, then click **New exercise**. The log is replaced by one trace per
line, using one-letter activities. Generation also clears exam answers written
for the previous log and turns off **Normalize process boundaries**, because
exercises use classic Alpha. A single **Undo** restores the previous log, its
answers and its options. The document name is unchanged.

| Difficulty | Activities | Traces | Constructs |
| --- | --- | --- | --- |
| Easy | 4–6 | 2–4 | Choice or parallel blocks of single activities, without nesting. |
| Medium | 6–8 | 2–6 | Exclusive choices and parallel branches, both present, nested up to two levels. |
| Hard | 7–10 | 2–8 | Blocks nested up to three levels, with a loop of at least three activities and a choice or parallel block. |

Each log comes from a random block-structured process with unique activities.
Every case starts with `A` and ends with the last letter. The traces cover every
direct succession of that process, so the log is complete for Alpha. A log is
kept only when α(W) is a sound workflow net and accepts exactly the process
traces up to the longest trace that repeats each loop at most once. Otherwise
another process is drawn. Loops of one or two activities, silent steps,
duplicate activities and parallel starts or ends, which Alpha cannot rediscover,
are never generated. The generating process and its random seed are not shown.

**Reveal answers step by step** hides the results behind a statement card. The
walkthrough has 11 steps: direct successions, relations, the relation matrix,
TW, TI, TO, XW, YW, PW, FW and α(W). Each card states the rule before its
answer. Successions, TI and TO list the working for each distinct trace. YW
lists removed pairs of XW, each with a maximal pair that contains it. The last
step shows the net, the word check, the soundness verdict and the commands to
open α(W), deduce BPMN or copy the write-up. Explanations show at most 12 lines
and count the rest.

Use **Show answer** on the next card, or **Previous** and **Next step** in the
status bar. **Show all** and **Hide answers** change every step at once. The
walkthrough also works for logs that you type or import. Opening, BPMN and copy
commands appear only once every step is revealed. **File → Export** still
provides the write-up.

The reveal option, the number of revealed steps and the selected difficulty are
view state. Each view keeps its own state, and session restoration resumes it.
Saved `.opx` files contain the generated log but not the view state. Exam mode
hides the exercise controls and the walkthrough; the generated log remains
available as the exam subject.

## Replay and continue modelling

Replay the recorded traces on the discovered net. Read failures such as a
transition not being enabled or remaining tokens after completion. A soundness
result requires the workflow-net conditions and completed analysis; an
undecided/truncated result is not a proof of soundness.
Replay explores silent routes and alternative transitions with the same label,
with bounded search. An exploration-limit message is inconclusive.

**Open Petri net** creates an editable document for simulation and
[further analysis](petri-nets.md). **Deduce BPMN** creates the corresponding
supported process projection. Both commands, and the **Copy** menu, appear in
the results header and again under the word check. They create separate
documents, not synchronized views of the original log. **File → Export** provides the Alpha write-up when
the current results are ready. Save `.opx` to preserve your log and authored work.

Silent Petri routes become BPMN routing gateways rather than unnamed business
tasks. Transparent connectors are contracted only when doing so preserves
branches and loops. Generated start/end events remain unnamed so converting the
BPMN back to Petri does not invent visible boundary activities. Review conversion
warnings: weighted/inhibitor arcs and place capacities are not represented by
the BPMN projection. Replaying observed traces is not a complete equivalence
proof.

## Limits and interpretation

Alpha is sensitive to log completeness and its modelling assumptions. A log
contains observed behaviour, not proof that all possible behaviour was recorded.
Short loops, noise and incomplete observations can make its result unsuitable.
Additional miners and general conformance metrics are not implemented.

Discovery is bounded to 128 activities, 100,000 events and 2 MiB of source, with
additional construction limits. An incomplete construction is reported as an
error. Boundary normalization needs two of the activity slots. The 2 MiB bound
applies to the prepared event-log source, not the original padded CSV file.
Oversized pasted text is rejected; open the CSV/XLSX file to prepare its source.
Simplify oversized teaching examples; preserve the original data outside
the app before changing it. See [troubleshooting](troubleshooting.md).

## Analyze recorded sales

Create an **Analyze sales** document, open a CSV containing the sales columns, or
open a workbook with a sales worksheet. **Add sales file…** imports another CSV
or workbook into the same study. The source filename is retained and supplies
the initial channel name; edit channel labels in the shared inspector. Source
filenames must be unique within a study: reimporting a filename is rejected even
after its channel label changes. Renamed copies of identical records in the same
channel are also rejected. Separate
sources can reuse sale identifiers without creating a join.

The recognized columns are `Id`, `StructureId`, `Pizza Type`, `Pizza Size`,
`Revenue`, `Cost`, `Satisfaction`, `Weekday`, `Customer Location` and
`Customer Type`. Import validates the table before adding it. A missing amount
or an Excel formula without a usable cached result is an error. Excel formulas
are not executed. Empty satisfaction cells are missing observations; recorded
zero ratings remain observations. Original files are not rewritten.

The **Summary** displays recorded sales count, revenue, costs, profit, revenue per
sale, profit/revenue ratio and average satisfaction with its observed/missing
counts. Profit subtracts the recorded costs only. Amounts use hundredths of the
source unit, without assuming a currency; decimal amounts beyond this precision
are rejected rather than silently rounded. The derived mean and ratio are
displayed to two decimal places.

**Breakdown** groups by channel, weekday, customer category, customer location,
pizza, pizza size, or pizza/customer/day. Pizza rankings order by recorded sales
count, then revenue. Filters match source values exactly and remain local to
each view. Charts show up to 20 groups; the scrollable table and exports include
all groups. Aggregation runs on immutable snapshots in the existing background
worker, and edits or changed filters invalidate older results.

The optional **Recorded scenario (StructureId)** filter matches the recorded
value exactly. Select its channel as well: a scenario identifier refers to a
template in its own source workbook, not to `Sales.Id` or an event-log `CaseId`.
Its availability does not assert a join between unrelated sources.

**Weekday trends** draws Monday-to-Sunday profiles for all recorded sales and
each recorded customer type, with a numeric comparison table. Choose counts,
revenue, profit or average observed satisfaction. Empty-day counts are zero;
satisfaction averages without any score leave gaps. The display shows up to 20 types and
the CSV retains every type. Unrecognized weekday values remain in overall/grouped
totals and are reported separately rather than assigned a guessed day.

The summary explains two manager KPIs: profit divided by recorded revenue, and
observed satisfaction scores divided by recorded sales. The first has no value
when revenue is zero; the second when there are no rows. Score zero remains an
observation. Neither indicator establishes the completeness of recorded costs
or the representativeness of customer feedback.

**File → Export** provides the current grouped analysis CSV or the filtered sales
records CSV, plus separate weekday-series and KPI CSVs. Row exports retain
channel/source columns. The initial CSV reader
accepts one channel per file; a mixed-channel row export must be split or filtered
to one channel before reimport. Save `.opx` for lossless study persistence across
all channels. Undo/redo covers imported sources, removal and channel edits;
filters and chart settings are separate view state. Exam mode hides generated
analytics while keeping source data available.

The study is limited to 100,000 records and 256 datasets, with the normal document
and worker transport limits. Recorded rows do not establish unique customers or
transaction counts. A weekday supports a weekday distribution, not a dated time
series; no processing duration is inferred. Prices and other workbook sheets are
not imported into this sales study. Use separately corrected CSVs for process
discovery when a workbook's event log is older.
