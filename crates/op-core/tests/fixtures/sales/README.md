# Sales import fixtures

These are hand-authored, synthetic XLSX files assembled from minimal OpenXML
parts in ZIP archives. Their invented sale rows exercise the importer without
depending on personal course files, Excel or formula evaluation.

| Fixture | Case covered |
| --- | --- |
| [tiny-sales.xlsx](tiny-sales.xlsx) | Select the sales sheet beside an event sheet; preserve local identifiers, a zero satisfaction score, missing satisfaction and a location containing a separator. Cached revenue/cost formulas and a cached `0.1+0.2` result exercise exact monetary conversion despite insignificant binary roundoff. The sparse sales sheet declares the full Excel column width. |
| [ambiguous-sales.xlsx](ambiguous-sales.xlsx) | Reject two worksheets containing valid sales headers. |
| [missing-money-cache.xlsx](missing-money-cache.xlsx) | Reject a required revenue formula without a stored result. |
| [partial-sales-header.xlsx](partial-sales-header.xlsx) | Reject an incomplete recognized sales header rather than treating it as an event log. |

Formula names and cached results are authored test data. The importer reads
stored values and never evaluates the formulas. Tests embed these files directly;
their assertions live in [the sales importer tests](../../../src/sales/import.rs).
