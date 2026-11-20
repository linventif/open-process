# TakeAway TP delivery plan

All work uses Open Process domain models and editors. ProM and BizAgi are not
part of the implementation. The original coursework inputs are retained.

| Increment | Implementation and evidence | Status |
| --- | --- | --- |
| Corrected event-log imports | CSV/XLSX import, source provenance, sparse padding handling | Complete |
| Petri discovery | Three editable models, silent routing markers, 327 observed trace replays | Complete |
| Channel BPMN models | Three enriched OPX/BPMN/SVG models, explanation and source evidence | Complete |
| Global BPMN | One connected participant, shared roles/data, 327 accepted traces and 15 selected rejected counterexamples | Complete |
| Quantitative tools | Native weekday/customer-type curves, exact scenario filter, two explained KPIs and CSV exports | Complete |
| Independent analysis | Seven figures, sixteen tables, eight main native comparisons and a separate scenario diagnostic | Complete |
| Detailed final report | French academic PDF/HTML, four vector appendices, explicit data limits and conditional cancellation sensitivity | Complete |
| Verification and delivery | 351 tests, native/WASM/Firefox checks, official XSD, geometry, reproducibility and submission archive | Complete |

Do not interpret recorded sales as unique customers or assume that sales IDs
identify event-log cases. Weekdays are categorical profiles, not calendar dates.
Role titles and information associations are documented authoring assumptions.

Internet cancellation-template rows retain recorded monetary values; the report
publishes the original totals and a separate conditional sensitivity. It does
not silently remove rows or mistake potentially inapplicable satisfaction for a
proven collection defect.

Open the [final PDF](report/rapport-final.pdf) or [HTML report](report/rapport-final.html).
The full editable models, sources, figures and recorded checks are packaged in
`submission/takeaway-tp.zip`. [Reproduction instructions](REPRODUCE.md) describe
the native generator, independent analysis and publication steps.
