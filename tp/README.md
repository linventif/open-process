# TP inputs and working results

This directory contains the supplied course documents, source data and generated
working results. The original files are retained unchanged for provenance.

## Source documents and data

- [Assignment, 2026–2027](TpBPM%20FA%202026-2027.pdf).
- [Supplied ProM user guide](User%20Guide%20of%20PROM_Shujun%20Hou.pdf).
- [Running example](running_example.csv).

The corrected CSV files in `clean/` are the authoritative event logs:

| Channel | Corrected event log | Original event log | Supplied workbook |
| --- | --- | --- | --- |
| Counter | [cleanTakeAwayCounter.csv](clean/cleanTakeAwayCounter.csv) | [TakeAway-Counter.csv](TakeAway-Counter.csv) | [TakeAway-Counter.xlsx](TakeAway-Counter.xlsx) |
| Phone | [cleanTakeAwayPhone.csv](clean/cleanTakeAwayPhone.csv) | [TakeAway-Phone.csv](TakeAway-Phone.csv) | [TakeAway-Phone.xlsx](TakeAway-Phone.xlsx) |
| Internet | [cleanTakeAwayInternet.csv](clean/cleanTakeAwayInternet.csv) | [TakeAway-Internet.csv](TakeAway-Internet.csv) | [TakeAway-Internet.xlsx](TakeAway-Internet.xlsx) |

The three XLSX copies in `clean/` are byte-identical to the original workbooks.
Their Counter and Phone event worksheets do **not** incorporate the corrected
CSVs: Counter still lacks some actor assignments, and Phone still uses `Start`
where its corrected CSV uses `Prepare`. Internet's original and corrected CSVs
are identical. Use the corrected CSVs for process discovery and the workbooks'
Sales worksheets for sales analysis; sale IDs are not event-log case IDs.

## Generated working results

These files can be opened in Open Process. The imported log documents preserve
the corrected CSV content. In the Petri models, routing markers such as `Vide1`
become unnamed silent transitions; the source logs remain unchanged. All 327
observed cases were replayed successfully against the projected Petri models.

| Channel | Imported event log | Editable Petri model |
| --- | --- | --- |
| Counter | [Log](results/import-petri/takeaway-counter-log.opx) | [Petri model](results/import-petri/takeaway-counter-petri.opx) |
| Phone | [Log](results/import-petri/takeaway-phone-log.opx) | [Petri model](results/import-petri/takeaway-phone-petri.opx) |
| Internet | [Log](results/import-petri/takeaway-internet-log.opx) | [Petri model](results/import-petri/takeaway-internet-petri.opx) |

The [sales study](results/sales/takeaway-sales.opx) combines the three Sales
worksheets. Its CSV summaries cover [channels](results/sales/channels.csv),
[weekdays](results/sales/weekdays.csv), [pizza types](results/sales/pizzas.csv),
[customer types](results/sales/customers.csv),
[customer locations](results/sales/locations.csv) and
[rankings](results/sales/rankings.csv). Monetary amounts retain the source's
unspecified unit. Sales counts do not establish a count of unique customers.

## Final deliverables

- [Detailed French report, PDF](report/rapport-final.pdf), with vector BPMN appendices.
- [HTML report](report/rapport-final.html), with figures that open at full resolution.
- [Complete submission archive](submission/takeaway-tp.zip) and its [SHA-256](submission/takeaway-tp.zip.sha256).
- [Delivery plan and status](PLAN.md), [reproduction instructions](REPRODUCE.md)
  and [recorded verification results](results/verification.json).

| BPMN model | Editable native file | BPMN 2.0 exchange | Vector diagram |
| --- | --- | --- | --- |
| Counter | [OPX](results/bpmn/takeaway-counter.opx) | [BPMN](results/bpmn/takeaway-counter.bpmn) | [SVG](results/bpmn/takeaway-counter.svg) |
| Phone | [OPX](results/bpmn/takeaway-phone.opx) | [BPMN](results/bpmn/takeaway-phone.bpmn) | [SVG](results/bpmn/takeaway-phone.svg) |
| Internet | [OPX](results/bpmn/takeaway-internet.opx) | [BPMN](results/bpmn/takeaway-internet.bpmn) | [SVG](results/bpmn/takeaway-internet.svg) |
| Global | [OPX](results/bpmn/takeaway-global.opx) | [BPMN](results/bpmn/takeaway-global.bpmn) | [SVG](results/bpmn/takeaway-global.svg) |

The models include observed assignment evidence, explicitly inferred role lanes
and proposed information associations. The global model connects all three
channels inside one participant while retaining their control-flow constraints.
All 327 observed traces replay after conversion and XML round-trip; selected
counterexamples are rejected. These checks do not prove complete behavioral
equivalence or human-resource scheduling.

[Evidence](results/bpmn/evidence.json) retains complementary worksheet values,
scenario comparisons, actor counts and assumptions. [Model validation](results/bpmn/validation.json)
records the exact scope and each case result. [Weekday series](results/sales/weekday-series.csv)
and [KPI exports](results/sales/kpis.csv) supplement the native sales study.
[Independent figures and tables](report/analysis/summary.json) reconcile with all
eight native CSVs. The report answers the assignment with the observed units and
explicitly identifies unobservable unique customers, dated transactions and
cross-source joins.

The [Internet scenario-7 subset](results/sales/internet-structure-7.csv) and
[independent cancellation diagnostic](report/analysis/tables/scenario_diagnostics.csv)
identify three source rows linked to a cancellation template, with nonzero cached
amounts and missing satisfaction. The report keeps recorded totals and shows a
conditional exclusion separately; template references within a workbook are
supported, while sales-ID-to-case-ID joins are not assumed.
