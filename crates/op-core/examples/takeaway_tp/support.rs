use calamine::{DataRef, Reader, Xlsx};
use op_core::mining::{EventLog, is_routing_marker};
use op_core::sales::{Money, SalesDataset};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fs;
use std::io::Cursor;
use std::path::Path;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

#[derive(Clone, Serialize)]
pub struct Event {
    pub case_id: String,
    pub activity: String,
    pub actor: Option<String>,
}

#[derive(Serialize)]
pub struct Row {
    pub row: u32,
    pub cells: BTreeMap<String, String>,
}

#[derive(Serialize)]
pub struct Sheet {
    pub name: String,
    pub rows: Vec<Row>,
}

pub struct Corpus {
    pub channel: String,
    pub csv_path: String,
    pub workbook_path: String,
    pub csv_fingerprint: String,
    pub workbook_fingerprint: String,
    pub events: Vec<Event>,
    pub log: EventLog,
    pub case_ids: Vec<String>,
    pub sheets: Vec<Sheet>,
    pub sales: SalesDataset,
}

/// A reproducible, explicitly named fingerprint; not a cryptographic signature.
pub fn fingerprint(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for &byte in bytes {
        hash = (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn csv_rows(source: &str) -> Result<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = source.trim_start_matches('\u{feff}').chars().peekable();
    let (mut quoted, mut closed) = (false, false);
    while let Some(c) = chars.next() {
        if quoted {
            if c == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    field.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                field.push(c);
            }
        } else if c == ';' || c == '\r' || c == '\n' {
            row.push(std::mem::take(&mut field));
            closed = false;
            if c != ';' {
                if c == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                if row.iter().any(|value| !value.is_empty()) {
                    rows.push(std::mem::take(&mut row));
                } else {
                    row.clear();
                }
            }
        } else if closed {
            return Err("Unexpected text after a closing CSV quote".into());
        } else if c == '"' {
            if !field.is_empty() {
                return Err("Unexpected CSV quote".into());
            }
            quoted = true;
        } else {
            field.push(c);
        }
    }
    if quoted {
        return Err("Unclosed CSV quote".into());
    }
    if !field.is_empty() || !row.is_empty() {
        row.push(field);
        rows.push(row);
    }
    Ok(rows)
}

fn column_name(mut index: u32) -> String {
    let mut name = Vec::new();
    loop {
        name.push((b'A' + (index % 26) as u8) as char);
        if index < 26 {
            break;
        }
        index = index / 26 - 1;
    }
    name.into_iter().rev().collect()
}

fn cached_value(value: &DataRef<'_>) -> Result<String> {
    Ok(match value {
        DataRef::Empty => String::new(),
        DataRef::String(value) | DataRef::DateTimeIso(value) | DataRef::DurationIso(value) => value.clone(),
        DataRef::SharedString(value) => (*value).to_owned(),
        DataRef::Int(value) => value.to_string(),
        DataRef::Float(value) => value.to_string(),
        DataRef::Bool(value) => value.to_string(),
        DataRef::DateTime(value) => value.to_string(),
        DataRef::Error(error) => return Err(format!("Worksheet cell error: {error}").into()),
    })
}

fn all_worksheets(bytes: &[u8]) -> Result<Vec<Sheet>> {
    let mut workbook = Xlsx::new(Cursor::new(bytes))?;
    let mut sheets = Vec::new();
    let mut cells = 0usize;
    for name in workbook.sheet_names() {
        let mut reader = workbook.worksheet_cells_reader(&name)?;
        let mut rows: BTreeMap<u32, BTreeMap<String, String>> = BTreeMap::new();
        while let Some(cell) = reader.next_cell()? {
            cells += 1;
            if cells > 1_000_000 {
                return Err("Workbook exceeds the artifact generator's cell limit".into());
            }
            let value = cached_value(cell.get_value())?;
            if !value.is_empty() {
                let (row, col) = cell.get_position();
                rows.entry(row + 1).or_default().insert(column_name(col), value);
            }
        }
        sheets.push(Sheet {
            name,
            rows: rows.into_iter().map(|(row, cells)| Row { row, cells }).collect(),
        });
    }
    Ok(sheets)
}

pub fn load(root: &Path, channel: &str) -> Result<Corpus> {
    let csv_path = format!("tp/clean/cleanTakeAway{channel}.csv");
    let workbook_path = format!("tp/clean/TakeAway-{channel}.xlsx");
    let csv_bytes = fs::read(root.join(&csv_path))?;
    let workbook_bytes = fs::read(root.join(&workbook_path))?;
    let source = std::str::from_utf8(&csv_bytes)?;
    let rows = csv_rows(source)?;
    if rows.first().map(|row| row.iter().map(String::as_str).collect::<Vec<_>>()) != Some(vec!["CaseId", "Activity", "Actor"]) {
        return Err(format!("Unexpected corrected CSV schema: {csv_path}").into());
    }
    let mut events = Vec::new();
    let mut case_ids = Vec::new();
    let mut case_traces = BTreeMap::<String, Vec<String>>::new();
    for row in rows.into_iter().skip(1) {
        if row.len() != 3 || row[0].is_empty() || row[1].is_empty() {
            return Err(format!("Invalid corrected event row in {csv_path}").into());
        }
        if !case_traces.contains_key(&row[0]) {
            case_ids.push(row[0].clone());
        }
        case_traces.entry(row[0].clone()).or_default().push(row[1].clone());
        events.push(Event {
            case_id: row[0].clone(),
            activity: row[1].clone(),
            actor: (!row[2].is_empty()).then(|| row[2].clone()),
        });
    }
    let log = EventLog::parse(source)?;
    let expected: Vec<_> = case_ids.iter().map(|case| case_traces[case].clone()).collect();
    assert_eq!(log.traces, expected, "Organizational extraction must preserve the discovery traces");
    let sheets = all_worksheets(&workbook_bytes)?;
    if sheets.len() != 4 {
        return Err(format!("Expected all four complementary worksheets in {workbook_path}").into());
    }
    let sales = op_core::sales::import::import_sales_workbook(&workbook_bytes, &format!("TakeAway-{channel}.xlsx"))?
        .ok_or("No sales worksheet in the complementary workbook")?;
    Ok(Corpus {
        channel: channel.into(),
        csv_path,
        workbook_path,
        csv_fingerprint: fingerprint(&csv_bytes),
        workbook_fingerprint: fingerprint(&workbook_bytes),
        events,
        log,
        case_ids,
        sheets,
        sales,
    })
}

pub fn actor_counts(events: &[Event], activity: &str) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    for event in events.iter().filter(|event| event.activity == activity) {
        *counts
            .entry(event.actor.clone().unwrap_or_else(|| "(unrecorded)".into()))
            .or_default() += 1;
    }
    counts
}

pub fn audit(corpus: &Corpus) -> Result<Value> {
    let events = &corpus.sheets[0];
    let workbook_events: Vec<_> = events
        .rows
        .iter()
        .skip(1)
        .map(|row| ["A", "B", "C"].map(|col| row.cells.get(col).cloned().unwrap_or_default()))
        .collect();
    let changed_rows = workbook_events
        .iter()
        .zip(&corpus.events)
        .filter(|(row, event)| row[0] != event.case_id || row[1] != event.activity || row[2] != event.actor.clone().unwrap_or_default())
        .count();
    let structure_sheet = &corpus.sheets[1];
    let mut structures = BTreeMap::<String, Vec<String>>::new();
    for row in structure_sheet.rows.iter().skip(1) {
        if let (Some(id), Some(activity)) = (row.cells.get("A"), row.cells.get("B")) {
            structures.entry(id.clone()).or_default().push(activity.clone());
        }
    }
    let cancellation_structures: BTreeSet<_> = structures
        .iter()
        .filter(|(_, trace)| trace.iter().any(|activity| activity == "Cancel order"))
        .map(|(id, _)| id.clone())
        .collect();
    let cancellation_records: Vec<_> = corpus
        .sales
        .records
        .iter()
        .filter(|record| cancellation_structures.contains(&record.structure_id))
        .collect();
    let cancellation_revenue = cancellation_records
        .iter()
        .try_fold(Money::default(), |sum, record| sum.checked_add(record.revenue))?;
    let cancellation_cost = cancellation_records
        .iter()
        .try_fold(Money::default(), |sum, record| sum.checked_add(record.cost))?;
    let cancellation_diagnostic = json!({
        "case_structure_worksheet":structure_sheet.name,"sales_worksheet":corpus.sheets[2].name,
        "source_name":corpus.sales.source_name,"cancellation_structure_ids":cancellation_structures,
        "all_sale_structure_ids_resolve":corpus.sales.records.iter().all(|record|structures.contains_key(&record.structure_id)),
        "recorded_sale_ids":cancellation_records.iter().map(|record|record.id.as_str()).collect::<Vec<_>>(),
        "record_count":cancellation_records.len(),"recorded_revenue_minor_units":cancellation_revenue.0,
        "recorded_cost_minor_units":cancellation_cost.0,"recorded_profit_minor_units":cancellation_revenue.checked_sub(cancellation_cost)?.0,
        "missing_satisfaction_records":cancellation_records.iter().filter(|record|record.satisfaction.is_none()).count(),
        "reference_scope":"Shared StructureId values classify sales records by the named trace template in this workbook. Sale IDs do not identify individual event-log cases.",
        "accounting_scope":"No row is removed or reclassified. The source does not define realized revenue, refunds, or the recognition of costs on cancellation. Any exclusion is a separately stated sensitivity assumption."
    });
    let structure_audit: Vec<_> = structures
        .into_iter()
        .map(|(id, raw)| {
            let corrected: Vec<_> = raw
                .iter()
                .map(|activity| {
                    if corpus.channel == "Phone" && activity == "Start" {
                        "Prepare".to_owned()
                    } else {
                        activity.clone()
                    }
                })
                .collect();
            let matches = corpus.log.traces.iter().filter(|trace| **trace == corrected).count();
            json!({"structure_id":id,"raw_worksheet_trace":raw,"comparison_trace":corrected,"matching_corrected_cases":matches})
        })
        .collect();
    let tariffs: Vec<_> = corpus.sheets[3].rows.iter().skip(1).filter_map(|row| {
        let [Some(kind), Some(size), Some(cost), Some(revenue)] = ["A", "B", "C", "D"].map(|col| row.cells.get(col)) else { return None };
        Some((kind, size, cost, revenue))
    }).map(|(kind, size, cost, revenue)| -> Result<Value> {
        Ok(json!({"pizza_type":kind,"pizza_size":size,"cost_minor_units":Money::parse_decimal(cost)?.0,"revenue_minor_units":Money::parse_decimal(revenue)?.0}))
    }).collect::<Result<_>>()?;
    let activities: BTreeMap<_, _> = corpus
        .log
        .activities()
        .into_iter()
        .map(|activity| {
            let counts = actor_counts(&corpus.events, &activity);
            (activity, counts)
        })
        .collect();
    let unknown: Vec<_> = corpus
        .events
        .iter()
        .filter(|event| event.actor.is_none() && !is_routing_marker(&event.activity))
        .collect();
    Ok(json!({
        "channel":corpus.channel,"authoritative_event_log":corpus.csv_path,"event_log_fingerprint":corpus.csv_fingerprint,
        "complementary_workbook":corpus.workbook_path,"workbook_fingerprint":corpus.workbook_fingerprint,
        "cases":corpus.log.traces.len(),"events":corpus.events.len(),"variants":corpus.log.variants().len(),
        "actors_by_activity":activities,"unknown_business_actor_events":unknown,
        "workbook_event_rows":workbook_events.len(),"workbook_event_rows_different_from_corrected_csv":changed_rows,
        "case_structure_comparison_aliases":if corpus.channel=="Phone" {json!({"Start":"Prepare"})}else{json!({})},
        "case_structures":structure_audit,"cancellation_template_diagnostic":cancellation_diagnostic,"tariff_catalogue":tariffs,"sale_rows":corpus.sales.records.len(),
        "notes":["The corrected CSV is authoritative for process behavior and actors.","The Phone structure comparison declares Start -> Prepare explicitly; no source file is modified.","A sale ID is not a case ID or customer ID. No row-level sales/event join is inferred.","Role titles and information associations in the BPMN are authored interpretations, not observed organizational attributes."],
        "all_workbook_worksheets_cached_values":corpus.sheets
    }))
}
