//! Structural import of event-log CSVs and XLSX worksheets.
//!
//! Preserve event values and order. Empty spreadsheet padding is not event data;
//! semantic changes belong to explicit, separately authored transformations.

use crate::i18n::{tr, trf};
use calamine::{DataRef, Reader, Xlsx};
use std::collections::BTreeMap;
use std::io::Cursor;
use std::iter::Peekable;
use std::str::Chars;

const MAX_TEXT_BYTES: usize = 2 * 1024 * 1024;
const MAX_ROWS: usize = 100_001;
const MAX_COLUMNS: usize = 16_384;
const MAX_CELLS: usize = 1_000_000;
const MAX_SHEETS: usize = 64;

#[derive(Default)]
pub(crate) struct SparseRow {
    pub(crate) width: usize,
    pub(crate) fields: BTreeMap<usize, String>,
}

impl SparseRow {
    fn push(&mut self, value: String) -> Result<(), String> {
        if self.width >= MAX_COLUMNS {
            return Err(tr("mining.analysis_limit").into());
        }
        if !value.is_empty() {
            self.fields.insert(self.width, value);
        }
        self.width += 1;
        Ok(())
    }

    fn log_columns(&self) -> Result<Option<(usize, usize)>, String> {
        let normalized: Vec<_> = self
            .fields
            .iter()
            .map(|(col, v)| (*col, v.trim().to_lowercase().replace([' ', '_', '-'], "")))
            .collect();
        let cases: Vec<_> = normalized
            .iter()
            .filter(|(_, v)| {
                matches!(
                    v.as_str(),
                    "case" | "caseid" | "id" | "instance" | "instanceid" | "case:concept:name"
                )
            })
            .map(|(col, _)| *col)
            .collect();
        let activities: Vec<_> = normalized
            .iter()
            .filter(|(_, v)| {
                matches!(
                    v.as_str(),
                    "activity" | "activityname" | "task" | "taskname" | "event" | "eventname" | "concept:name"
                )
            })
            .map(|(col, _)| *col)
            .collect();
        if cases.is_empty() || activities.is_empty() {
            Ok(None)
        } else if cases.len() != 1 || activities.len() != 1 {
            Err(tr("mining.import.ambiguous_columns").into())
        } else {
            Ok(Some((cases[0], activities[0])))
        }
    }

    fn used_width(&self) -> usize {
        self.fields.last_key_value().map_or(0, |(col, _)| col + 1)
    }
}

/// Parse a single sparse record at a time, including quoted line breaks. Repeated
/// empty fields increment a counter rather than allocating strings or cells.
pub(crate) struct CsvRows<'a> {
    chars: Peekable<Chars<'a>>,
    delimiter: char,
}

impl<'a> CsvRows<'a> {
    pub(crate) fn new(source: &'a str, delimiter: char) -> Self {
        Self {
            chars: source.chars().peekable(),
            delimiter,
        }
    }

    pub(crate) fn next(&mut self) -> Result<Option<SparseRow>, String> {
        let mut row = SparseRow::default();
        let mut field = String::new();
        let (mut quoted, mut was_quoted, mut closed, mut any) = (false, false, false, false);
        while let Some(c) = self.chars.next() {
            any = true;
            if quoted {
                if c == '"' {
                    if self.chars.peek() == Some(&'"') {
                        field.push('"');
                        self.chars.next();
                    } else {
                        quoted = false;
                        closed = true;
                    }
                } else {
                    field.push(c);
                }
            } else if c == self.delimiter || c == '\n' || c == '\r' {
                row.push(if was_quoted {
                    std::mem::take(&mut field)
                } else {
                    field.trim().to_owned()
                })?;
                field.clear();
                was_quoted = false;
                closed = false;
                if c != self.delimiter {
                    if c == '\r' && self.chars.peek() == Some(&'\n') {
                        self.chars.next();
                    }
                    return Ok(Some(row));
                }
            } else if closed {
                if c != ' ' && c != '\t' {
                    return Err(tr("mining.invalid_csv_quoting").into());
                }
            } else if c == '"' {
                if !field.trim().is_empty() {
                    return Err(tr("mining.invalid_csv_quoting").into());
                }
                field.clear();
                quoted = true;
                was_quoted = true;
            } else {
                field.push(c);
            }
            if field.len() > MAX_TEXT_BYTES {
                return Err(tr("mining.analysis_limit").into());
            }
        }
        if quoted {
            return Err(tr("mining.invalid_csv_quoting").into());
        }
        if any {
            row.push(if was_quoted { field } else { field.trim().to_owned() })?;
            Ok(Some(row))
        } else {
            Ok(None)
        }
    }
}

pub(crate) fn skip_preamble(mut source: &str) -> &str {
    source = source.trim_start_matches('\u{feff}');
    while let Some(line) = source.lines().next() {
        let line = line.trim();
        if !line.is_empty() && !line.starts_with('#') && !line.starts_with("//") {
            break;
        }
        source = source.split_once('\n').map_or("", |(_, rest)| rest);
    }
    source
}

/// Recognize a CSV event-log header using the same aliases and preamble rules
/// as structural normalization, without parsing its event records.
pub fn is_csv_event_log(source: &str) -> bool {
    let body = skip_preamble(source);
    [',', ';', '\t'].into_iter().any(|delimiter| {
        CsvRows::new(body, delimiter)
            .next()
            .ok()
            .flatten()
            .and_then(|header| header.log_columns().ok().flatten())
            .is_some()
    })
}

/// Remove only globally empty trailing CSV columns. Other CSVs, teaching logs
/// and XES retain their original source text, including its formatting.
pub fn normalize_event_log_text(source: &str) -> Result<String, String> {
    if source.len() > crate::document::MAX_DOCUMENT_BYTES {
        return Err(tr("file.input_limit").into());
    }
    let body = skip_preamble(source);
    if body.trim_start().starts_with('<') {
        return Ok(source.to_owned());
    }
    for delimiter in [',', ';', '\t'] {
        let mut reader = CsvRows::new(body, delimiter);
        let Ok(Some(header)) = reader.next() else { continue };
        let Some((case_col, activity_col)) = header.log_columns()? else {
            continue;
        };
        let mut rows = vec![header];
        let mut value_bytes = 0;
        while let Some(row) = reader.next()? {
            if row.fields.is_empty() {
                continue;
            }
            value_bytes += row.fields.values().map(String::len).sum::<usize>();
            if rows.len() >= MAX_ROWS || value_bytes > MAX_TEXT_BYTES {
                return Err(tr("mining.analysis_limit").into());
            }
            rows.push(row);
        }
        let width = rows.iter().map(SparseRow::used_width).max().unwrap_or(0);
        for (index, row) in rows.iter().enumerate() {
            if row.width < width
                || (index > 0
                    && [case_col, activity_col]
                        .iter()
                        .any(|col| row.fields.get(col).is_none_or(|value| value.trim().is_empty())))
            {
                return Err(trf("mining.invalid_csv_record", &[&(index + 1)]));
            }
        }
        if rows.iter().all(|row| row.width == width) {
            if source.len() > MAX_TEXT_BYTES {
                return Err(tr("mining.analysis_limit").into());
            }
            return Ok(source.to_owned());
        }
        return csv_text(&rows, width);
    }
    Ok(source.to_owned())
}

fn xlsx_error(error: impl std::fmt::Display) -> String {
    trf("mining.import.xlsx_error", &[&error])
}

fn cell_text(value: &DataRef<'_>) -> Result<String, String> {
    Ok(match value {
        DataRef::Empty => String::new(),
        DataRef::Int(value) => value.to_string(),
        DataRef::Float(value) => value.to_string(),
        DataRef::Bool(value) => value.to_string(),
        DataRef::String(value) | DataRef::DateTimeIso(value) | DataRef::DurationIso(value) => value.clone(),
        DataRef::SharedString(value) => (*value).to_owned(),
        DataRef::DateTime(value) => value.to_string(),
        DataRef::Error(error) => return Err(xlsx_error(error)),
    })
}

/// Import the sole worksheet whose first nonempty row has case and activity
/// columns. Read cached values only; never evaluate Excel formulas or macros.
pub fn import_xlsx_event_log(bytes: &[u8]) -> Result<String, String> {
    if bytes.len() > crate::document::MAX_DOCUMENT_BYTES {
        return Err(tr("file.input_limit").into());
    }
    let mut workbook = Xlsx::new(Cursor::new(bytes)).map_err(xlsx_error)?;
    let names = workbook.sheet_names();
    if names.len() > MAX_SHEETS {
        return Err(tr("mining.analysis_limit").into());
    }
    let mut candidates = Vec::new();
    let mut scanned = 0;
    for name in names {
        let mut reader = workbook.worksheet_cells_reader(&name).map_err(xlsx_error)?;
        let mut header = SparseRow::default();
        let mut header_row = None;
        while let Some(cell) = reader.next_cell().map_err(xlsx_error)? {
            scanned += 1;
            if scanned > MAX_CELLS {
                return Err(tr("mining.analysis_limit").into());
            }
            let (row, col) = cell.get_position();
            if header_row.is_some_and(|first| row != first) {
                break;
            }
            let value = cell_text(cell.get_value())?;
            if !value.is_empty() {
                if col as usize >= MAX_COLUMNS || value.len() > MAX_TEXT_BYTES {
                    return Err(tr("mining.analysis_limit").into());
                }
                header_row = Some(row);
                header.fields.insert(col as usize, value);
            }
        }
        if header.log_columns()?.is_some() {
            candidates.push(name);
        }
    }
    let name = match candidates.as_slice() {
        [name] => name,
        [] => return Err(tr("mining.import.xlsx_missing").into()),
        _ => return Err(trf("mining.import.xlsx_ambiguous", &[&candidates.join(", ")])),
    };
    let mut reader = workbook.worksheet_cells_reader(name).map_err(xlsx_error)?;
    let mut rows = BTreeMap::<u32, SparseRow>::new();
    let mut value_bytes = 0;
    while let Some(cell) = reader.next_cell().map_err(xlsx_error)? {
        scanned += 1;
        if scanned > MAX_CELLS {
            return Err(tr("mining.analysis_limit").into());
        }
        let value = cell_text(cell.get_value())?;
        if value.is_empty() {
            continue;
        }
        value_bytes += value.len();
        let (row, col) = cell.get_position();
        if value_bytes > MAX_TEXT_BYTES || col as usize >= MAX_COLUMNS {
            return Err(tr("mining.analysis_limit").into());
        }
        rows.entry(row).or_default().fields.insert(col as usize, value);
        if rows.len() > MAX_ROWS {
            return Err(tr("mining.analysis_limit").into());
        }
    }
    let rows: Vec<_> = rows.into_values().collect();
    if rows.len() < 2 {
        return Err(tr("mining.no_trace_found").into());
    }
    let (case_col, activity_col) = rows[0].log_columns()?.ok_or_else(|| tr("mining.import.xlsx_missing").to_owned())?;
    for (index, row) in rows.iter().enumerate().skip(1) {
        if [case_col, activity_col]
            .iter()
            .any(|col| row.fields.get(col).is_none_or(|value| value.trim().is_empty()))
        {
            return Err(trf("mining.invalid_csv_record", &[&(index + 1)]));
        }
    }
    let width = rows.iter().map(SparseRow::used_width).max().unwrap_or(0);
    csv_text(&rows, width)
}

fn csv_text(rows: &[SparseRow], width: usize) -> Result<String, String> {
    if rows.len().saturating_mul(width) > MAX_TEXT_BYTES {
        return Err(tr("mining.analysis_limit").into());
    }
    let mut output = String::new();
    for row in rows {
        for col in 0..width {
            if col > 0 {
                output.push(';');
            }
            if let Some(value) = row.fields.get(&col) {
                if value.contains([';', '"', '\r', '\n']) || value.trim() != value {
                    output.push('"');
                    for c in value.chars() {
                        if c == '"' {
                            output.push('"');
                        }
                        output.push(c);
                    }
                    output.push('"');
                } else {
                    output.push_str(value);
                }
            }
            if output.len() > MAX_TEXT_BYTES {
                return Err(tr("mining.analysis_limit").into());
            }
        }
        output.push('\n');
        if output.len() > MAX_TEXT_BYTES {
            return Err(tr("mining.analysis_limit").into());
        }
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    const WORKBOOK: &[u8] = include_bytes!("../../tests/fixtures/mining/tiny-event-log.xlsx");

    #[test]
    fn padded_csv_is_compact_and_preserves_all_nonempty_fields_and_case_order() {
        let padding = ";".repeat(16_381);
        let source =
            format!("CaseId;Activity;Actor{padding}\r\n7;Register;William{padding}\r\n9;Ship;{padding}\r\n7;Check;Patrick{padding}\r\n");
        let normalized = normalize_event_log_text(&source).unwrap();
        assert_eq!(normalized, "CaseId;Activity;Actor\n7;Register;William\n9;Ship;\n7;Check;Patrick\n");
        assert_eq!(
            super::super::EventLog::parse(&normalized).unwrap().traces,
            [vec!["Register", "Check"], vec!["Ship"]]
        );
    }

    #[test]
    fn normalization_preserves_quoted_newlines_delimiters_whitespace_and_quotes() {
        let source = "# source comment\r\nCase ID,Activity,Actor,Note,,\r\n7,\"Mix, \\\"?\",Pat,unused,,\r\n";
        // A closing quote followed by text must remain a hard parse error.
        assert!(normalize_event_log_text(source).is_err());
        let source = "\u{feff}# source comment\r\nCase ID,Activity,Actor,Note,,\r\n7,\"Mix, \"\"pizza\"\"\",\" Pat\r\nRick \",kept,,\r\n";
        let normalized = normalize_event_log_text(source).unwrap();
        assert_eq!(
            normalized,
            "Case ID;Activity;Actor;Note\n7;\"Mix, \"\"pizza\"\"\";\" Pat\r\nRick \";kept\n"
        );
        assert_eq!(super::super::EventLog::parse(&normalized).unwrap().traces, [vec!["Mix, \"pizza\""]]);
    }

    #[test]
    fn csv_without_padding_teaching_logs_and_xes_remain_unchanged() {
        for source in [
            "ABCD\nACBD\n",
            "case,activity,Actor\r\n1,\"Check, then sign\",Pat\r\n",
            "<log><trace/></log>",
        ] {
            assert_eq!(normalize_event_log_text(source).unwrap(), source);
        }
    }

    #[test]
    fn csv_recognition_honors_header_aliases_bom_and_comment_preambles() {
        assert!(is_csv_event_log(
            "\u{feff}# imported log\n\n// source note\nid,activity,Actor\n1,Check,Pat\n"
        ));
        assert!(is_csv_event_log("instance_id\ttask_name\n1\tCheck\n"));
        assert!(!is_csv_event_log("ABCD\nACBD\n"));
        assert!(!is_csv_event_log("case,id,activity\n1,1,Check\n"));
    }

    #[test]
    fn malformed_or_ambiguous_csv_records_are_not_repaired() {
        for source in [
            "case,activity,Actor\n1,Check\n",
            "case,activity\n1,\"unclosed\n",
            "case,activity\n,Check\n",
            "case,activity\n1,\n",
            "CaseId,Case,Activity\n1,1,Check\n",
        ] {
            assert!(normalize_event_log_text(source).is_err(), "{source}");
        }
    }

    #[test]
    fn columns_are_removed_only_when_empty_across_the_whole_csv() {
        let source = "CaseId;Activity;Actor;;\n1;Check;Pat;;important\n";
        assert_eq!(normalize_event_log_text(source).unwrap(), source);
        let source = "CaseId;Activity;Actor;;;\n1;Check;Pat;;important;\n";
        assert_eq!(
            normalize_event_log_text(source).unwrap(),
            "CaseId;Activity;Actor;;\n1;Check;Pat;;important\n"
        );
    }

    #[test]
    fn xlsx_selects_event_sheet_preserves_actors_and_reads_cached_formula_values() {
        assert_eq!(
            import_xlsx_event_log(WORKBOOK).unwrap(),
            "CaseId;Activity;Actor\n7;Register;William\n9;Ship;\n7;Check;\"Pat\nRick\"\n"
        );
    }

    #[test]
    fn xlsx_rejects_ambiguous_or_missing_event_sheets_and_missing_values() {
        assert!(
            import_xlsx_event_log(include_bytes!("../../tests/fixtures/mining/ambiguous-event-logs.xlsx"))
                .unwrap_err()
                .contains("First")
        );
        assert!(import_xlsx_event_log(include_bytes!("../../tests/fixtures/mining/no-event-log.xlsx")).is_err());
        assert!(import_xlsx_event_log(include_bytes!("../../tests/fixtures/mining/duplicate-log-columns.xlsx")).is_err());
        assert!(import_xlsx_event_log(include_bytes!("../../tests/fixtures/mining/missing-activity.xlsx")).is_err());
        assert!(import_xlsx_event_log(b"not a workbook").is_err());
    }

    #[test]
    fn excessive_output_padding_is_rejected_before_it_is_materialized() {
        let source = format!(
            "CaseId;Activity{};Resource\n1;Check{};Pat\n",
            ";".repeat(MAX_COLUMNS - 4),
            ";".repeat(MAX_COLUMNS - 4)
        );
        assert_eq!(normalize_event_log_text(&source).unwrap(), source);
        let source = format!("{source}{}", format!("1;Check{};Pat\n", ";".repeat(MAX_COLUMNS - 4)).repeat(140));
        assert!(normalize_event_log_text(&source).is_err());
    }
}
