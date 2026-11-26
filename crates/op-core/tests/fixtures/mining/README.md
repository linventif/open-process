# Event-log import fixtures

These are hand-authored, synthetic XLSX files assembled from minimal OpenXML
parts in ZIP archives. They contain small invented examples, not copies of the
course workbooks. The tests embed them directly and do not require personal
course files, Excel or formula evaluation.

| Fixture | Case covered |
| --- | --- |
| [tiny-event-log.xlsx](tiny-event-log.xlsx) | Select the event sheet beside an unrelated sheet; preserve interleaved case order, an empty actor and a multiline actor; read a cached activity formula value. The sparse event sheet declares the full Excel column width. |
| [ambiguous-event-logs.xlsx](ambiguous-event-logs.xlsx) | Reject two worksheets containing valid event-log headers. |
| [duplicate-log-columns.xlsx](duplicate-log-columns.xlsx) | Reject ambiguous required column aliases. |
| [missing-activity.xlsx](missing-activity.xlsx) | Reject an event missing its required activity. |
| [no-event-log.xlsx](no-event-log.xlsx) | Reject a workbook without an event-log sheet. |

The formula text is deliberately synthetic. Only its stored result is imported;
the tests do not execute it. Assertions live in
[the event-log importer tests](../../../src/mining/import.rs).
