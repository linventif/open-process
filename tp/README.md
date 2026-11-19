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

These are intermediate results. BPMN models with organizational and
informational details, and the final TP report, have not been produced yet.
