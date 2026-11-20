//! Content-based import of sales tables, independently of event-log worksheets.
//!
//! Formula results come from cached cell values. No formula or macro is executed,
//! and sale IDs are never interpreted as event-log case IDs.

use super::{Money, Sale, SalesDataset};
use crate::i18n::{tr, trf};
use crate::mining::import::{CsvRows, SparseRow, skip_preamble};
use calamine::{DataRef, Reader, Xlsx};
use std::collections::BTreeMap;
use std::io::Cursor;

const MAX_ROWS: usize = 100_001;
const MAX_CELLS: usize = 1_000_000;
const MAX_SHEETS: usize = 64;
const MAX_VALUE_BYTES: usize = 2 * 1024 * 1024;
const MAX_COLUMNS: usize = 16_384;
const LABELS: [&str; 10] = [
    "Id",
    "StructureId",
    "Pizza Type",
    "Pizza Size",
    "Revenue",
    "Cost",
    "Satisfaction",
    "Weekday",
    "Customer Location",
    "Customer Type",
];

fn column_kind(value: &str) -> Option<usize> {
    let normalized = value.trim().to_lowercase().replace([' ', '_', '-'], "");
    Some(match normalized.as_str() {
        "id" | "saleid" => 0,
        "structureid" | "variantid" => 1,
        "pizzatype" | "producttype" => 2,
        "pizzasize" | "productsize" => 3,
        "revenue" => 4,
        "cost" => 5,
        "satisfaction" => 6,
        "weekday" => 7,
        "customerlocation" | "location" => 8,
        "customertype" => 9,
        _ => return None,
    })
}

fn is_candidate(header: &SparseRow) -> bool {
    let names: Vec<_> = header
        .fields
        .values()
        .map(|value| value.trim().to_lowercase().replace([' ', '_', '-'], ""))
        .collect();
    let has_case = names.iter().any(|value| {
        matches!(
            value.as_str(),
            "case" | "caseid" | "id" | "instance" | "instanceid" | "case:concept:name"
        )
    });
    let has_activity = names.iter().any(|value| {
        matches!(
            value.as_str(),
            "activity" | "activityname" | "task" | "taskname" | "event" | "eventname" | "concept:name"
        )
    });
    if has_case && has_activity {
        return false;
    }
    let mut found = [false; 10];
    for value in header.fields.values() {
        if let Some(kind) = column_kind(value) {
            found[kind] = true;
        }
    }
    found[0] && ((found[4] && found[5]) || ((found[4] || found[5]) && found[1..4].iter().any(|v| *v)))
}

struct Header([usize; 10]);

impl Header {
    fn parse(row: &SparseRow) -> Result<Option<Self>, String> {
        if !is_candidate(row) {
            return Ok(None);
        }
        let mut columns = [None; 10];
        for (col, value) in &row.fields {
            if let Some(kind) = column_kind(value)
                && columns[kind].replace(*col).is_some()
            {
                return Err(trf("sales.import.duplicate_column", &[&LABELS[kind]]));
            }
        }
        let mut result = [0; 10];
        for (kind, col) in columns.into_iter().enumerate() {
            result[kind] = col.ok_or_else(|| trf("sales.import.missing_column", &[&LABELS[kind]]))?;
        }
        Ok(Some(Self(result)))
    }

    fn sale(&self, row: &SparseRow) -> Result<Sale, String> {
        let value = |kind: usize| row.fields.get(&self.0[kind]).map(String::as_str).unwrap_or("");
        let revenue = money(value(4))?;
        let cost = money(value(5))?;
        let satisfaction = if value(6).trim().is_empty() {
            None
        } else {
            let parsed = value(6)
                .trim()
                .replace(',', ".")
                .parse::<f64>()
                .map_err(|_| tr("sales.import.invalid_satisfaction").to_owned())?;
            if !parsed.is_finite() {
                return Err(tr("sales.import.invalid_satisfaction").into());
            }
            Some(parsed)
        };
        Ok(Sale {
            id: value(0).to_owned(),
            structure_id: value(1).to_owned(),
            pizza_type: value(2).to_owned(),
            pizza_size: value(3).to_owned(),
            revenue,
            cost,
            satisfaction,
            weekday: value(7).to_owned(),
            customer_location: value(8).to_owned(),
            customer_type: value(9).to_owned(),
        })
    }
}

fn money(value: &str) -> Result<Money, String> {
    if value.trim().is_empty() {
        return Err(tr("sales.import.missing_money").into());
    }
    Money::parse_decimal(value)
}

fn limit() -> String {
    tr("sales.import.limit").into()
}

fn xlsx_error(error: impl std::fmt::Display) -> String {
    trf("sales.import.xlsx_error", &[&error])
}

fn cell_text(value: &DataRef<'_>) -> Result<String, String> {
    let text = match value {
        DataRef::Empty => String::new(),
        DataRef::Int(value) => value.to_string(),
        DataRef::Float(value) => value.to_string(),
        DataRef::Bool(value) => value.to_string(),
        DataRef::String(value) | DataRef::DateTimeIso(value) | DataRef::DurationIso(value) => value.clone(),
        DataRef::SharedString(value) => (*value).to_owned(),
        DataRef::DateTime(value) => value.to_string(),
        DataRef::Error(error) => return Err(xlsx_error(error)),
    };
    if text.len() > MAX_VALUE_BYTES {
        return Err(limit());
    }
    Ok(text)
}

fn dataset(source_name: &str, fallback: &str, records: Vec<Sale>) -> Result<SalesDataset, String> {
    let name = if source_name.trim().is_empty() { fallback } else { source_name };
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let channel = basename
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map_or(basename, |(stem, _)| stem);
    let dataset = SalesDataset {
        source_name: name.to_owned(),
        channel: channel.to_owned(),
        records,
    };
    dataset.check_storage()?;
    Ok(dataset)
}

/// A positive sales-table probe. Partially specified sales headers still match
/// so the importer can explain missing columns instead of opening a false log.
pub fn is_sales_csv(source: &str) -> bool {
    let source = skip_preamble(source);
    [',', ';', '\t'].into_iter().any(|delimiter| {
        CsvRows::new(source, delimiter)
            .next()
            .ok()
            .flatten()
            .is_some_and(|header| is_candidate(&header))
    })
}

/// Import a sales CSV, or return `None` when its header does not identify sales.
/// IDs remain local to the imported source; no case-ID join is inferred.
pub fn import_sales_csv(source: &str, source_name: &str) -> Result<Option<SalesDataset>, String> {
    if source.len() > crate::document::MAX_DOCUMENT_BYTES {
        return Err(limit());
    }
    for delimiter in [',', ';', '\t'] {
        let mut reader = CsvRows::new(skip_preamble(source), delimiter);
        let Ok(Some(header_row)) = reader.next() else { continue };
        let Some(header) = Header::parse(&header_row)? else { continue };
        let channel_columns: Vec<_> = header_row
            .fields
            .iter()
            .filter_map(|(col, value)| (value.trim().eq_ignore_ascii_case("channel")).then_some(*col))
            .collect();
        if channel_columns.len() > 1 {
            return Err(trf("sales.import.duplicate_column", &[&"Channel"]));
        }
        let mut imported_channel: Option<String> = None;
        let mut records = Vec::new();
        let mut bytes = 0;
        let mut index = 1;
        while let Some(row) = reader.next()? {
            index += 1;
            if row.fields.is_empty() {
                continue;
            }
            bytes += row.fields.values().map(String::len).sum::<usize>();
            if records.len() >= MAX_ROWS - 1 || bytes > MAX_VALUE_BYTES {
                return Err(limit());
            }
            if row.width != header_row.width {
                return Err(trf("mining.invalid_csv_record", &[&index]));
            }
            if let Some(col) = channel_columns.first() {
                let channel = row.fields.get(col).cloned().unwrap_or_default();
                if imported_channel.as_ref().is_some_and(|previous| *previous != channel) {
                    return Err(tr("sales.import.mixed_channels").into());
                }
                imported_channel = Some(channel);
            }
            records.push(
                header
                    .sale(&row)
                    .map_err(|error| trf("sales.import.invalid_row", &[&index, &error]))?,
            );
        }
        let mut dataset = dataset(source_name, tr("sales.import.unnamed_source"), records)?;
        if let Some(channel) = imported_channel {
            dataset.channel = channel;
            dataset.check_storage()?;
        }
        return Ok(Some(dataset));
    }
    Ok(None)
}

/// Import the single sales worksheet by content. Return `None` only if no sales
/// header was found, allowing the caller's existing event-log import workflow.
/// Sparse streaming avoids allocations based on padded worksheet dimensions.
pub fn import_sales_workbook(bytes: &[u8], source_name: &str) -> Result<Option<SalesDataset>, String> {
    if bytes.len() > crate::document::MAX_DOCUMENT_BYTES {
        return Err(limit());
    }
    let mut workbook = Xlsx::new(Cursor::new(bytes)).map_err(xlsx_error)?;
    let names = workbook.sheet_names();
    if names.len() > MAX_SHEETS {
        return Err(limit());
    }
    let mut candidates = Vec::new();
    let mut scanned = 0;
    for name in names {
        let mut reader = workbook.worksheet_cells_reader(&name).map_err(xlsx_error)?;
        let mut header_row = SparseRow::default();
        let mut first_row = None;
        while let Some(cell) = reader.next_cell().map_err(xlsx_error)? {
            scanned += 1;
            if scanned > MAX_CELLS {
                return Err(limit());
            }
            let (row, col) = cell.get_position();
            if first_row.is_some_and(|first| row != first) {
                break;
            }
            let value = cell_text(cell.get_value())?;
            if !value.is_empty() {
                if col as usize >= MAX_COLUMNS {
                    return Err(limit());
                }
                first_row = Some(row);
                header_row.fields.insert(col as usize, value);
            }
        }
        if let Some(header) = Header::parse(&header_row)? {
            candidates.push((name, header, first_row.unwrap()));
        }
    }
    let (name, header, header_index) = match candidates.as_slice() {
        [] => return Ok(None),
        [candidate] => candidate,
        _ => {
            return Err(trf(
                "sales.import.ambiguous_sheets",
                &[&candidates.iter().map(|(name, _, _)| name.as_str()).collect::<Vec<_>>().join(", ")],
            ));
        }
    };
    let mut reader = workbook.worksheet_cells_reader(name).map_err(xlsx_error)?;
    let mut rows = BTreeMap::<u32, SparseRow>::new();
    let mut value_bytes = 0;
    while let Some(cell) = reader.next_cell().map_err(xlsx_error)? {
        scanned += 1;
        if scanned > MAX_CELLS {
            return Err(limit());
        }
        let (row, col) = cell.get_position();
        if row <= *header_index {
            continue;
        }
        let value = if [header.0[4], header.0[5]].contains(&(col as usize)) {
            match cell.get_value() {
                DataRef::Float(value) => Money::from_major(*value)?.to_string(),
                DataRef::Bool(_) | DataRef::DateTime(_) | DataRef::DateTimeIso(_) | DataRef::DurationIso(_) => {
                    return Err(trf("sales.import.invalid_row", &[&(row + 1), &tr("sales.import.missing_money")]));
                }
                value => cell_text(value)?,
            }
        } else {
            cell_text(cell.get_value())?
        };
        if value.is_empty() {
            continue;
        }
        value_bytes += value.len();
        if col as usize >= MAX_COLUMNS || value_bytes > MAX_VALUE_BYTES {
            return Err(limit());
        }
        rows.entry(row).or_default().fields.insert(col as usize, value);
        if rows.len() >= MAX_ROWS {
            return Err(limit());
        }
    }
    let records = rows
        .into_iter()
        .map(|(index, row)| {
            header
                .sale(&row)
                .map_err(|error| trf("sales.import.invalid_row", &[&(index + 1), &error]))
        })
        .collect::<Result<Vec<_>, _>>()?;
    dataset(source_name, name, records).map(Some)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKBOOK: &[u8] = include_bytes!("../../tests/fixtures/sales/tiny-sales.xlsx");
    const HEADER: &str = "Id;StructureId;Pizza Type;Pizza Size;Revenue;Cost;Satisfaction;Weekday;Customer Location;Customer Type";

    #[test]
    fn workbook_preserves_cached_money_scores_and_source_without_joining_event_ids() {
        let dataset = import_sales_workbook(WORKBOOK, "Counter.xlsx").unwrap().unwrap();
        assert_eq!(dataset.source_name, "Counter.xlsx");
        assert_eq!(dataset.channel, "Counter");
        assert_eq!(dataset.records.len(), 3);
        assert_eq!(dataset.records[0].id, "1");
        assert_eq!(dataset.records[0].structure_id, "1");
        assert_eq!(dataset.records[0].revenue, Money(1500));
        assert_eq!(dataset.records[0].cost, Money(1200));
        assert_eq!(dataset.records[0].satisfaction, Some(0.0));
        assert_eq!(dataset.records[0].customer_location, "London; Central");
        assert_eq!(
            dataset.records[1].revenue,
            Money(30),
            "insignificant cached binary roundoff is supported"
        );
        assert_eq!(dataset.records[1].satisfaction, None);
        assert_eq!(import_sales_workbook(WORKBOOK, "").unwrap().unwrap().channel, "SalesTable");
    }

    #[test]
    fn only_no_sales_workbook_allows_event_log_fallback() {
        assert!(
            import_sales_workbook(include_bytes!("../../tests/fixtures/mining/tiny-event-log.xlsx"), "example.xlsx")
                .unwrap()
                .is_none()
        );
        for bytes in [
            include_bytes!("../../tests/fixtures/sales/ambiguous-sales.xlsx").as_slice(),
            include_bytes!("../../tests/fixtures/sales/missing-money-cache.xlsx").as_slice(),
            include_bytes!("../../tests/fixtures/sales/partial-sales-header.xlsx").as_slice(),
            b"not a workbook".as_slice(),
        ] {
            assert!(import_sales_workbook(bytes, "sales.xlsx").is_err());
        }
    }

    #[test]
    fn csv_supports_decimal_comma_quotes_newlines_zero_scores_and_local_ids() {
        let text = format!(
            "\u{feff}# exported sales\n{HEADER}\r\n1;1;\"Margherita; Special\";Large;15,25;12.10;0;Friday;\"North\nDistrict\";Student\r\n1;2;Salami;Small;-1.25;0;;Monday;North;Adult\r\n"
        );
        let dataset = import_sales_csv(&text, "Phone.csv").unwrap().unwrap();
        assert_eq!(dataset.channel, "Phone");
        assert_eq!(dataset.records.len(), 2, "repeated local row IDs are preserved");
        assert_eq!(dataset.records[0].pizza_type, "Margherita; Special");
        assert_eq!(dataset.records[0].customer_location, "North\nDistrict");
        assert_eq!(dataset.records[0].revenue, Money(1525));
        assert_eq!(dataset.records[0].cost, Money(1210));
        assert_eq!(dataset.records[0].satisfaction, Some(0.0));
        assert_eq!(dataset.records[1].revenue, Money(-125));
        assert_eq!(dataset.records[1].cost, Money(0));
        assert_eq!(dataset.records[1].satisfaction, None);
    }

    #[test]
    fn positive_probe_does_not_claim_teaching_or_structured_event_logs() {
        for text in [
            "ABCD\nACBD\n",
            "CaseId;Activity;Actor\n7;Check;Pat\n",
            "Id,Activity,Revenue,Cost\n1,Check,15,12\n",
        ] {
            assert!(!is_sales_csv(text));
            assert!(import_sales_csv(text, "log.csv").unwrap().is_none());
        }
        let partial = "Id;StructureId;Revenue;Cost\n1;1;15;12\n";
        assert!(is_sales_csv(partial));
        assert!(import_sales_csv(partial, "sales.csv").is_err());
        let aliases = "sale_id;variant_id;product_type;product_size;Revenue;Cost;Satisfaction;Weekday;Location;Customer_Type\n1;1;Pizza;Large;15;12;5;Monday;North;Adult\n";
        assert!(is_sales_csv(aliases));
        assert!(import_sales_csv(aliases, "sales.csv").unwrap().is_some());
    }

    #[test]
    fn csv_rejects_missing_money_nonfinite_scores_ambiguous_headers_and_invalid_rows() {
        for row in [
            "1;1;Pizza;Large;;12;5;Friday;North;Adult",
            "1;1;Pizza;Large;15;;5;Friday;North;Adult",
            "1;1;Pizza;Large;Get_Revenue();12;5;Friday;North;Adult",
            "1;1;Pizza;Large;15;12;NaN;Friday;North;Adult",
            "1;1;Pizza;Large;15;12;inf;Friday;North;Adult",
            "1;1;Pizza;Large;15;12;5;Friday;North",
        ] {
            assert!(import_sales_csv(&format!("{HEADER}\n{row}\n"), "sales.csv").is_err(), "{row}");
        }
        assert!(
            import_sales_csv(
                &format!("{HEADER};Revenue\n1;1;Pizza;Large;15;12;5;Friday;North;Adult;15\n"),
                "sales.csv"
            )
            .is_err()
        );
    }

    #[test]
    fn csv_preserves_one_explicit_channel_and_rejects_mixed_channels() {
        let text = format!("Channel;Source;{HEADER}\nPhone;original.xlsx;1;1;Pizza;Large;15;12;5;Friday;North;Adult\n");
        let dataset = import_sales_csv(&text, "export.csv").unwrap().unwrap();
        assert_eq!(dataset.channel, "Phone");
        assert_eq!(dataset.source_name, "export.csv");
        let mixed = format!("{text}Counter;other.xlsx;1;1;Pizza;Large;15;12;5;Friday;North;Adult\n");
        assert!(import_sales_csv(&mixed, "export.csv").is_err());
    }

    #[test]
    fn csv_padding_is_counted_without_allocating_empty_column_values() {
        let padding = ";".repeat(MAX_COLUMNS - LABELS.len());
        let text = format!("{HEADER}{padding}\n1;1;Pizza;Large;15;12;5;Friday;North;Adult{padding}\n");
        assert_eq!(import_sales_csv(&text, "padded.csv").unwrap().unwrap().records.len(), 1);
    }
}
