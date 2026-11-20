//! Recorded sales studies and quantitative summaries, independent of the UI.
//!
//! A sales row is a recorded sale. Its identifier does not identify a customer,
//! an order or an event-log case. Monetary values share the source's unit;
//! no currency or relationship to event logs is inferred.

use crate::document::JsonCodec;
use crate::i18n::tr;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

pub mod import;

const MAX_ROWS: usize = 100_000;
const MAX_DATASETS: usize = 256;
const MAX_LABEL_BYTES: usize = 4096;

/// Hundredths of the recorded monetary unit. Integer storage avoids summing
/// binary floating-point amounts; the source does not necessarily specify EUR.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Money(pub i64);

impl Money {
    pub fn as_major(self) -> f64 {
        self.0 as f64 / 100.0
    }

    /// Accept a decimal point or comma, without guessing thousands separators.
    /// Sub-cent precision is rejected unless the additional digits are zero.
    pub fn parse_decimal(source: &str) -> Result<Self, String> {
        let source = source.trim();
        let (negative, digits) = match source.as_bytes().first() {
            Some(b'-') => (true, &source[1..]),
            Some(b'+') => (false, &source[1..]),
            _ => (false, source),
        };
        let mut parts = digits.split(['.', ',']);
        let whole = parts.next().unwrap_or_default();
        let fraction = parts.next().unwrap_or_default();
        if parts.next().is_some()
            || (whole.is_empty() && fraction.is_empty())
            || !whole.bytes().all(|b| b.is_ascii_digit())
            || !fraction.bytes().all(|b| b.is_ascii_digit())
            || fraction.as_bytes().get(2..).is_some_and(|tail| tail.iter().any(|&b| b != b'0'))
        {
            return Err(tr("sales.invalid_money").into());
        }
        let whole: i128 = if whole.is_empty() {
            0
        } else {
            whole.parse().map_err(|_| tr("sales.invalid_money").to_owned())?
        };
        let fraction = fraction.as_bytes();
        let minor = fraction.first().map_or(0, |b| i128::from(b - b'0') * 10) + fraction.get(1).map_or(0, |b| i128::from(b - b'0'));
        let value = whole
            .checked_mul(100)
            .and_then(|value| value.checked_add(minor))
            .and_then(|value| if negative { value.checked_neg() } else { Some(value) })
            .and_then(|value| i64::try_from(value).ok())
            .ok_or_else(|| tr("sales.invalid_money").to_owned())?;
        Ok(Self(value))
    }

    /// Convert a numeric spreadsheet cell while allowing insignificant binary
    /// representation error, rather than silently rounding sub-cent amounts.
    pub fn from_major(value: f64) -> Result<Self, String> {
        let scaled = value * 100.0;
        let rounded = scaled.round();
        if !value.is_finite()
            || !scaled.is_finite()
            || (scaled - rounded).abs() > 0.000_001
            || rounded < i64::MIN as f64
            || rounded >= i64::MAX as f64
        {
            return Err(tr("sales.invalid_money").into());
        }
        Ok(Self(rounded as i64))
    }

    pub fn checked_add(self, other: Self) -> Result<Self, String> {
        self.0
            .checked_add(other.0)
            .map(Self)
            .ok_or_else(|| tr("sales.invalid_money").into())
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, String> {
        self.0
            .checked_sub(other.0)
            .map(Self)
            .ok_or_else(|| tr("sales.invalid_money").into())
    }
}

impl fmt::Display for Money {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let absolute = self.0.unsigned_abs();
        write!(f, "{}{}.{:02}", if self.0 < 0 { "-" } else { "" }, absolute / 100, absolute % 100)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sale {
    pub id: String,
    pub structure_id: String,
    pub pizza_type: String,
    pub pizza_size: String,
    pub revenue: Money,
    pub cost: Money,
    pub satisfaction: Option<f64>,
    pub weekday: String,
    pub customer_location: String,
    pub customer_type: String,
}

impl Sale {
    fn labels(&self) -> [&str; 7] {
        [
            &self.id,
            &self.structure_id,
            &self.pizza_type,
            &self.pizza_size,
            &self.weekday,
            &self.customer_location,
            &self.customer_type,
        ]
    }

    fn check_storage(&self) -> Result<(), String> {
        if self.id.trim().is_empty() || self.labels().iter().any(|label| label.len() > MAX_LABEL_BYTES) {
            return Err(tr("sales.invalid_dataset").into());
        }
        if self.satisfaction.is_some_and(|value| !value.is_finite()) {
            return Err(tr("sales.invalid_number").into());
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SalesDataset {
    pub source_name: String,
    pub channel: String,
    pub records: Vec<Sale>,
}

impl SalesDataset {
    pub fn check_storage(&self) -> Result<(), String> {
        if self.source_name.trim().is_empty()
            || self.channel.trim().is_empty()
            || self.source_name.len() > MAX_LABEL_BYTES
            || self.channel.len() > MAX_LABEL_BYTES
            || self.records.is_empty()
        {
            return Err(tr("sales.invalid_dataset").into());
        }
        if self.records.len() > MAX_ROWS {
            return Err(tr("sales.analysis_limit").into());
        }
        for record in &self.records {
            record.check_storage()?;
        }
        Ok(())
    }

    fn duplicates(&self, other: &Self) -> bool {
        self.source_name == other.source_name || (self.channel == other.channel && self.records == other.records)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SalesStudy {
    pub name: String,
    pub datasets: Vec<SalesDataset>,
}

impl SalesStudy {
    pub const CODEC: JsonCodec<Self> = JsonCodec::checked("process.sales", Self::check_storage);

    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            datasets: Vec::new(),
        }
    }

    pub fn check_storage(&self) -> Result<(), String> {
        if self.name.len() > MAX_LABEL_BYTES || self.datasets.len() > MAX_DATASETS {
            return Err(tr("sales.analysis_limit").into());
        }
        let mut rows = 0usize;
        let mut text_bytes = self.name.len();
        for (index, dataset) in self.datasets.iter().enumerate() {
            dataset.check_storage()?;
            rows += dataset.records.len();
            text_bytes += dataset.source_name.len() + dataset.channel.len();
            for record in &dataset.records {
                text_bytes += record.labels().iter().map(|label| label.len()).sum::<usize>();
            }
            if rows > MAX_ROWS || text_bytes > crate::document::MAX_DOCUMENT_BYTES {
                return Err(tr("sales.analysis_limit").into());
            }
            if self.datasets[..index].iter().any(|existing| existing.duplicates(dataset)) {
                return Err(tr("sales.duplicate_dataset").into());
            }
        }
        Ok(())
    }

    /// An imported source is added once. Repeated row IDs in distinct sources
    /// are retained and never interpreted as a join or a customer identifier.
    pub fn add_dataset(&mut self, dataset: SalesDataset) -> Result<(), String> {
        dataset.check_storage()?;
        if self.datasets.iter().any(|existing| existing.duplicates(&dataset)) {
            return Err(tr("sales.duplicate_dataset").into());
        }
        if self.datasets.len() >= MAX_DATASETS
            || self.datasets.iter().map(|d| d.records.len()).sum::<usize>() + dataset.records.len() > MAX_ROWS
        {
            return Err(tr("sales.analysis_limit").into());
        }
        self.datasets.push(dataset);
        if let Err(error) = self.check_storage() {
            self.datasets.pop();
            return Err(error);
        }
        Ok(())
    }

    pub fn analyze(&self, filter: &SalesFilter) -> Result<SalesAnalysis, String> {
        self.check_storage()?;
        let mut totals = Accumulator::default();
        let mut groups: BTreeMap<SalesGroupBy, BTreeMap<SalesGroupKey, Accumulator>> =
            SalesGroupBy::ALL.into_iter().map(|by| (by, BTreeMap::new())).collect();
        let mut weekday_totals = std::array::from_fn(|_| Accumulator::default());
        let mut customer_weekdays: BTreeMap<String, [Accumulator; 7]> = BTreeMap::new();
        let mut unrecognized_weekday_rows = 0;
        for dataset in &self.datasets {
            for record in &dataset.records {
                if !filter.matches(&dataset.channel, record) {
                    continue;
                }
                totals.add(record)?;
                for (by, entries) in &mut groups {
                    entries.entry(by.key(&dataset.channel, record)).or_default().add(record)?;
                }
                if let Some(day) = Weekday::from_source(&record.weekday) {
                    weekday_totals[day.index()].add(record)?;
                    customer_weekdays
                        .entry(record.customer_type.clone())
                        .or_insert_with(|| std::array::from_fn(|_| Accumulator::default()))[day.index()]
                    .add(record)?;
                } else {
                    unrecognized_weekday_rows += 1;
                }
            }
        }
        let grouped = groups
            .into_iter()
            .map(|(by, entries)| {
                let mut rows = entries
                    .into_iter()
                    .map(|(key, values)| {
                        Ok(SalesGroup {
                            key,
                            metrics: values.finish()?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                match by {
                    SalesGroupBy::Weekday => rows.sort_by(|a, b| {
                        let rank = |key: &SalesGroupKey| match key {
                            SalesGroupKey::Weekday(day) => weekday_rank(day),
                            _ => usize::MAX,
                        };
                        rank(&a.key).cmp(&rank(&b.key)).then(a.key.cmp(&b.key))
                    }),
                    SalesGroupBy::PizzaType | SalesGroupBy::PizzaCustomerWeekday => rows.sort_by(|a, b| {
                        b.metrics
                            .sales_count
                            .cmp(&a.metrics.sales_count)
                            .then(b.metrics.revenue.cmp(&a.metrics.revenue))
                            .then(a.key.cmp(&b.key))
                    }),
                    _ => {}
                }
                Ok((by, rows))
            })
            .collect::<Result<BTreeMap<_, _>, String>>()?;
        Ok(SalesAnalysis {
            totals: totals.finish()?,
            grouped,
            weekday_evolution: SalesWeekdayEvolution {
                total: weekday_points(weekday_totals)?,
                customer_types: customer_weekdays
                    .into_iter()
                    .map(|(customer_type, values)| {
                        Ok(CustomerWeekdaySeries {
                            customer_type,
                            points: weekday_points(values)?,
                        })
                    })
                    .collect::<Result<_, String>>()?,
                unrecognized_weekday_rows,
            },
        })
    }

    /// Export filtered recorded rows, including source provenance. Aggregations
    /// do not turn these rows into distinct people, orders or event-log cases.
    pub fn to_csv(&self, filter: &SalesFilter) -> Result<String, String> {
        self.check_storage()?;
        let escape = |value: &str| {
            if value.contains([';', '"', '\r', '\n']) || value.trim() != value {
                format!("\"{}\"", value.replace('"', "\"\""))
            } else {
                value.to_owned()
            }
        };
        let mut output =
            "Channel;Source;Id;StructureId;Pizza Type;Pizza Size;Revenue;Cost;Satisfaction;Weekday;Customer Location;Customer Type\n"
                .to_owned();
        for dataset in &self.datasets {
            for record in &dataset.records {
                if filter.matches(&dataset.channel, record) {
                    let fields = [
                        dataset.channel.clone(),
                        dataset.source_name.clone(),
                        record.id.clone(),
                        record.structure_id.clone(),
                        record.pizza_type.clone(),
                        record.pizza_size.clone(),
                        record.revenue.to_string(),
                        record.cost.to_string(),
                        record.satisfaction.map(|value| value.to_string()).unwrap_or_default(),
                        record.weekday.clone(),
                        record.customer_location.clone(),
                        record.customer_type.clone(),
                    ];
                    output += &fields.iter().map(|value| escape(value)).collect::<Vec<_>>().join(";");
                    output.push('\n');
                    if output.len() > crate::document::MAX_DOCUMENT_BYTES {
                        return Err(tr("sales.analysis_limit").into());
                    }
                }
            }
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SalesFilter {
    pub channel: Option<String>,
    /// A source-local scenario identifier; it is not a sale or event-log case ID.
    #[serde(default)]
    pub structure_id: Option<String>,
    pub pizza_type: Option<String>,
    pub customer_type: Option<String>,
    pub weekday: Option<String>,
    pub customer_location: Option<String>,
}

impl SalesFilter {
    pub fn matches(&self, channel: &str, sale: &Sale) -> bool {
        [
            (&self.channel, channel),
            (&self.structure_id, sale.structure_id.as_str()),
            (&self.pizza_type, sale.pizza_type.as_str()),
            (&self.customer_type, sale.customer_type.as_str()),
            (&self.weekday, sale.weekday.as_str()),
            (&self.customer_location, sale.customer_location.as_str()),
        ]
        .into_iter()
        .all(|(filter, value)| filter.as_deref().is_none_or(|selected| selected == value))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SalesGroupBy {
    Channel,
    Weekday,
    CustomerType,
    CustomerLocation,
    PizzaType,
    PizzaSize,
    PizzaCustomerWeekday,
}

impl SalesGroupBy {
    pub const ALL: [Self; 7] = [
        Self::Channel,
        Self::Weekday,
        Self::CustomerType,
        Self::CustomerLocation,
        Self::PizzaType,
        Self::PizzaSize,
        Self::PizzaCustomerWeekday,
    ];

    fn key(self, channel: &str, sale: &Sale) -> SalesGroupKey {
        match self {
            Self::Channel => SalesGroupKey::Channel(channel.to_owned()),
            Self::Weekday => SalesGroupKey::Weekday(sale.weekday.clone()),
            Self::CustomerType => SalesGroupKey::CustomerType(sale.customer_type.clone()),
            Self::CustomerLocation => SalesGroupKey::CustomerLocation(sale.customer_location.clone()),
            Self::PizzaType => SalesGroupKey::PizzaType(sale.pizza_type.clone()),
            Self::PizzaSize => SalesGroupKey::PizzaSize(sale.pizza_size.clone()),
            Self::PizzaCustomerWeekday => SalesGroupKey::PizzaCustomerWeekday {
                pizza_type: sale.pizza_type.clone(),
                customer_type: sale.customer_type.clone(),
                weekday: sale.weekday.clone(),
            },
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SalesGroupKey {
    Channel(String),
    Weekday(String),
    CustomerType(String),
    CustomerLocation(String),
    PizzaType(String),
    PizzaSize(String),
    PizzaCustomerWeekday {
        pizza_type: String,
        customer_type: String,
        weekday: String,
    },
}

impl SalesGroupKey {
    pub fn values(&self) -> Vec<&str> {
        match self {
            Self::Channel(value)
            | Self::Weekday(value)
            | Self::CustomerType(value)
            | Self::CustomerLocation(value)
            | Self::PizzaType(value)
            | Self::PizzaSize(value) => vec![value],
            Self::PizzaCustomerWeekday {
                pizza_type,
                customer_type,
                weekday,
            } => vec![pizza_type, customer_type, weekday],
        }
    }

    pub fn label(&self) -> String {
        self.values().join(" / ")
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SalesMetrics {
    pub sales_count: usize,
    pub revenue: Money,
    pub cost: Money,
    pub profit: Money,
    /// Recorded monetary units per sale row, rather than an assumed order value.
    pub average_revenue: Option<f64>,
    /// A fraction, undefined when total revenue is zero.
    pub profit_rate: Option<f64>,
    pub satisfaction_average: Option<f64>,
    pub satisfaction_observed: usize,
    pub satisfaction_missing: usize,
}

impl SalesMetrics {
    /// Completeness of the recorded satisfaction evidence, not proof that the
    /// observations are representative of all customers. Zero is an observation.
    pub fn satisfaction_observation_rate(&self) -> Option<f64> {
        (self.sales_count > 0).then(|| self.satisfaction_observed as f64 / self.sales_count as f64)
    }
}

/// A weekday category from the recorded source. It contains no calendar date.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Weekday {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

impl Weekday {
    pub const ALL: [Self; 7] = [
        Self::Monday,
        Self::Tuesday,
        Self::Wednesday,
        Self::Thursday,
        Self::Friday,
        Self::Saturday,
        Self::Sunday,
    ];

    pub fn source_name(self) -> &'static str {
        match self {
            Self::Monday => "Monday",
            Self::Tuesday => "Tuesday",
            Self::Wednesday => "Wednesday",
            Self::Thursday => "Thursday",
            Self::Friday => "Friday",
            Self::Saturday => "Saturday",
            Self::Sunday => "Sunday",
        }
    }

    pub fn from_source(source: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|day| day.source_name().eq_ignore_ascii_case(source.trim()))
    }

    fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WeekdayPoint {
    pub weekday: Weekday,
    pub metrics: SalesMetrics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CustomerWeekdaySeries {
    pub customer_type: String,
    pub points: Vec<WeekdayPoint>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct SalesWeekdayEvolution {
    pub total: Vec<WeekdayPoint>,
    pub customer_types: Vec<CustomerWeekdaySeries>,
    /// These rows remain in totals and raw grouped tables; no day is guessed.
    pub unrecognized_weekday_rows: usize,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SalesGroup {
    pub key: SalesGroupKey,
    pub metrics: SalesMetrics,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SalesAnalysis {
    pub totals: SalesMetrics,
    pub grouped: BTreeMap<SalesGroupBy, Vec<SalesGroup>>,
    #[serde(default)]
    pub weekday_evolution: SalesWeekdayEvolution,
}

impl SalesAnalysis {
    pub fn groups(&self, by: SalesGroupBy) -> &[SalesGroup] {
        self.grouped.get(&by).map(Vec::as_slice).unwrap_or_default()
    }

    /// A grouped analysis snapshot, distinct from the importable sale-row table.
    /// Rates are fractions and undefined averages/rates are empty cells.
    pub fn to_csv(&self, by: SalesGroupBy) -> String {
        let labels: &[&str] = match by {
            SalesGroupBy::Channel => &["Channel"],
            SalesGroupBy::Weekday => &["Weekday"],
            SalesGroupBy::CustomerType => &["Customer Type"],
            SalesGroupBy::CustomerLocation => &["Customer Location"],
            SalesGroupBy::PizzaType => &["Pizza Type"],
            SalesGroupBy::PizzaSize => &["Pizza Size"],
            SalesGroupBy::PizzaCustomerWeekday => &["Pizza Type", "Customer Type", "Weekday"],
        };
        let escape = |value: &str| {
            if value.contains([';', '"', '\r', '\n']) || value.trim() != value {
                format!("\"{}\"", value.replace('"', "\"\""))
            } else {
                value.to_owned()
            }
        };
        let mut output = labels.join(";");
        output += ";Recorded sales;Revenue;Cost;Profit;Average revenue per recorded sale;Profit rate (fraction);Satisfaction average;Satisfaction observed;Satisfaction missing\n";
        for group in self.groups(by) {
            let mut cells: Vec<_> = group.key.values().into_iter().map(escape).collect();
            let metrics = &group.metrics;
            cells.extend([
                metrics.sales_count.to_string(),
                metrics.revenue.to_string(),
                metrics.cost.to_string(),
                metrics.profit.to_string(),
                metrics.average_revenue.map(|value| value.to_string()).unwrap_or_default(),
                metrics.profit_rate.map(|value| value.to_string()).unwrap_or_default(),
                metrics.satisfaction_average.map(|value| value.to_string()).unwrap_or_default(),
                metrics.satisfaction_observed.to_string(),
                metrics.satisfaction_missing.to_string(),
            ]);
            output += &cells.join(";");
            output.push('\n');
        }
        output
    }

    /// Export the seven categorical weekdays, never an inferred dated timeline.
    /// Unknown source weekday labels are retained as separate total rows.
    pub fn weekday_series_csv(&self) -> String {
        let mut output = "Series;Customer Type;Weekday;Recorded sales;Revenue;Cost;Profit;Satisfaction average;Satisfaction observed;Satisfaction missing\n".to_owned();
        let mut append = |series: &str, customer_type: &str, weekday: &str, metrics: &SalesMetrics| {
            let cells = [
                csv_cell(series),
                csv_cell(customer_type),
                csv_cell(weekday),
                metrics.sales_count.to_string(),
                metrics.revenue.to_string(),
                metrics.cost.to_string(),
                metrics.profit.to_string(),
                metrics.satisfaction_average.map(|value| value.to_string()).unwrap_or_default(),
                metrics.satisfaction_observed.to_string(),
                metrics.satisfaction_missing.to_string(),
            ];
            output += &cells.join(";");
            output.push('\n');
        };
        for point in &self.weekday_evolution.total {
            append("Total", "", point.weekday.source_name(), &point.metrics);
        }
        for series in &self.weekday_evolution.customer_types {
            for point in &series.points {
                append("Customer type", &series.customer_type, point.weekday.source_name(), &point.metrics);
            }
        }
        for group in self.groups(SalesGroupBy::Weekday) {
            if let SalesGroupKey::Weekday(day) = &group.key
                && Weekday::from_source(day).is_none()
            {
                append("Unrecognized weekday", "", day, &group.metrics);
            }
        }
        output
    }

    /// Two managerial indicators whose denominators are supported by sale rows.
    /// These do not require a customer identifier, order join or rating threshold.
    pub fn kpis_csv(&self) -> String {
        let metrics = &self.totals;
        format!(
            "KPI;Value (fraction);Numerator;Denominator;Definition\nRecorded margin rate;{};{};{};Profit / recorded revenue\nSatisfaction observation rate;{};{};{};Rated sales / recorded sales\n",
            metrics.profit_rate.map(|value| value.to_string()).unwrap_or_default(),
            metrics.profit,
            metrics.revenue,
            metrics
                .satisfaction_observation_rate()
                .map(|value| value.to_string())
                .unwrap_or_default(),
            metrics.satisfaction_observed,
            metrics.sales_count,
        )
    }
}

fn csv_cell(value: &str) -> String {
    if value.contains([';', '"', '\r', '\n']) || value.trim() != value {
        format!("\"{}\"", value.replace('"', "\"\""))
    } else {
        value.to_owned()
    }
}

fn weekday_points(values: [Accumulator; 7]) -> Result<Vec<WeekdayPoint>, String> {
    Weekday::ALL
        .into_iter()
        .zip(values)
        .map(|(weekday, values)| {
            Ok(WeekdayPoint {
                weekday,
                metrics: values.finish()?,
            })
        })
        .collect()
}

#[derive(Default)]
struct Accumulator {
    count: usize,
    revenue: Money,
    cost: Money,
    satisfaction_sum: f64,
    satisfaction_observed: usize,
}

impl Accumulator {
    fn add(&mut self, record: &Sale) -> Result<(), String> {
        self.count += 1;
        self.revenue = self.revenue.checked_add(record.revenue)?;
        self.cost = self.cost.checked_add(record.cost)?;
        if let Some(value) = record.satisfaction {
            self.satisfaction_sum += value;
            if !self.satisfaction_sum.is_finite() {
                return Err(tr("sales.invalid_number").into());
            }
            self.satisfaction_observed += 1;
        }
        Ok(())
    }

    fn finish(self) -> Result<SalesMetrics, String> {
        let profit = self.revenue.checked_sub(self.cost)?;
        Ok(SalesMetrics {
            sales_count: self.count,
            revenue: self.revenue,
            cost: self.cost,
            profit,
            average_revenue: (self.count > 0).then(|| self.revenue.as_major() / self.count as f64),
            profit_rate: (self.revenue.0 != 0).then(|| profit.0 as f64 / self.revenue.0 as f64),
            satisfaction_average: (self.satisfaction_observed > 0).then(|| self.satisfaction_sum / self.satisfaction_observed as f64),
            satisfaction_observed: self.satisfaction_observed,
            satisfaction_missing: self.count - self.satisfaction_observed,
        })
    }
}

fn weekday_rank(day: &str) -> usize {
    ["monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday"]
        .iter()
        .position(|known| *known == day.trim().to_ascii_lowercase())
        .unwrap_or(7)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::document::{Document, DocumentCodec};

    fn sale(id: &str, pizza: &str, day: &str, revenue: i64, cost: i64, satisfaction: Option<f64>) -> Sale {
        Sale {
            id: id.into(),
            structure_id: "recorded-structure".into(),
            pizza_type: pizza.into(),
            pizza_size: "M".into(),
            revenue: Money(revenue),
            cost: Money(cost),
            satisfaction,
            weekday: day.into(),
            customer_location: "Toulouse".into(),
            customer_type: "Regular".into(),
        }
    }

    fn study() -> SalesStudy {
        let mut study = SalesStudy::new("Recorded sales");
        study
            .add_dataset(SalesDataset {
                source_name: "counter.xlsx".into(),
                channel: "Counter".into(),
                records: vec![
                    sale("1", "Margherita", "Sunday", 1001, 401, Some(0.0)),
                    sale("2", "Margherita", "Monday", 2002, 1202, None),
                ],
            })
            .unwrap();
        let mut student = sale("1", "Pepperoni", "Monday", 999, 699, Some(4.0));
        student.customer_type = "Student".into();
        student.customer_location = "Paris".into();
        study
            .add_dataset(SalesDataset {
                source_name: "phone.xlsx".into(),
                channel: "Phone".into(),
                records: vec![student],
            })
            .unwrap();
        study
    }

    #[test]
    fn decimal_money_is_exact_checked_and_neutral_about_currency() {
        assert_eq!(
            Money::parse_decimal("0.10")
                .unwrap()
                .checked_add(Money::parse_decimal("0,20").unwrap())
                .unwrap(),
            Money(30)
        );
        assert_eq!(Money::parse_decimal(" +12.3400 ").unwrap(), Money(1234));
        assert_eq!(Money::parse_decimal("-.05").unwrap(), Money(-5));
        assert_eq!(Money::from_major(0.1 + 0.2).unwrap(), Money(30));
        assert_eq!(Money::parse_decimal("-92233720368547758.08").unwrap(), Money(i64::MIN));
        assert_eq!(Money(i64::MIN).to_string(), "-92233720368547758.08");
        for invalid in ["", "1.234", "1,000.00", "1e2", "EUR 5", "92233720368547758.08"] {
            assert!(Money::parse_decimal(invalid).is_err(), "{invalid}");
        }
        for invalid in [f64::NAN, f64::INFINITY, 0.001, 1e30] {
            assert!(Money::from_major(invalid).is_err());
        }
        assert!(Money(i64::MAX).checked_add(Money(1)).is_err());
    }

    #[test]
    fn aggregates_recorded_rows_without_inventing_customer_or_order_counts() {
        let analysis = study().analyze(&SalesFilter::default()).unwrap();
        let totals = &analysis.totals;
        assert_eq!(totals.sales_count, 3);
        assert_eq!(
            (totals.revenue, totals.cost, totals.profit),
            (Money(4002), Money(2302), Money(1700))
        );
        assert!((totals.average_revenue.unwrap() - 13.34).abs() < 1e-12);
        assert!((totals.profit_rate.unwrap() - 1700.0 / 4002.0).abs() < 1e-12);
        assert_eq!((totals.satisfaction_observed, totals.satisfaction_missing), (2, 1));
        assert_eq!(totals.satisfaction_average, Some(2.0));
        assert_eq!(
            analysis
                .groups(SalesGroupBy::Weekday)
                .iter()
                .map(|g| g.key.label())
                .collect::<Vec<_>>(),
            ["Monday", "Sunday"]
        );
        let pizzas = analysis.groups(SalesGroupBy::PizzaType);
        assert_eq!(pizzas[0].key, SalesGroupKey::PizzaType("Margherita".into()));
        assert_eq!(pizzas[0].metrics.sales_count, 2);
        assert_eq!(
            analysis.groups(SalesGroupBy::PizzaCustomerWeekday)[0].key.values(),
            ["Margherita", "Regular", "Monday"]
        );
    }

    #[test]
    fn filters_all_requested_dimensions_and_keeps_zero_satisfaction_observed() {
        let filter = SalesFilter {
            channel: Some("Counter".into()),
            structure_id: None,
            weekday: Some("Sunday".into()),
            pizza_type: Some("Margherita".into()),
            customer_type: Some("Regular".into()),
            customer_location: Some("Toulouse".into()),
        };
        let result = study().analyze(&filter).unwrap();
        assert_eq!(result.totals.sales_count, 1);
        assert_eq!(result.totals.revenue, Money(1001));
        assert_eq!(result.totals.satisfaction_average, Some(0.0));
        assert_eq!(result.totals.satisfaction_observed, 1);
        let empty = study()
            .analyze(&SalesFilter {
                channel: Some("Unknown".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(empty.totals.sales_count, 0);
        assert_eq!(empty.totals.average_revenue, None);
        assert_eq!(empty.totals.profit_rate, None);
        assert_eq!(empty.totals.satisfaction_average, None);
        assert!(SalesGroupBy::ALL.into_iter().all(|by| empty.groups(by).is_empty()));
    }

    #[test]
    fn scenario_filter_is_exact_and_can_be_scoped_to_a_source_channel() {
        let mut study = study();
        study.datasets[0].records[0].structure_id = "7".into();
        study.datasets[0].records[1].structure_id = "17".into();
        study.datasets[1].records[0].structure_id = "7".into();
        let original = study.clone();
        let filter = SalesFilter {
            structure_id: Some("7".into()),
            ..Default::default()
        };
        assert_eq!(study.analyze(&filter).unwrap().totals.sales_count, 2);
        let scoped = SalesFilter {
            channel: Some("Counter".into()),
            ..filter
        };
        let result = study.analyze(&scoped).unwrap();
        assert_eq!(result.totals.sales_count, 1);
        assert_eq!(result.totals.revenue, Money(1001));
        assert_eq!(result.totals.satisfaction_average, Some(0.0));
        assert_eq!(study.to_csv(&scoped).unwrap().lines().count(), 2);
        assert_eq!(study, original);
        let sale_id_is_not_a_scenario = SalesFilter {
            structure_id: Some("1".into()),
            ..Default::default()
        };
        assert_eq!(study.analyze(&sale_id_is_not_a_scenario).unwrap().totals.sales_count, 0);
        let legacy: SalesFilter = serde_json::from_value(serde_json::json!({"channel":"Counter"})).unwrap();
        assert_eq!(legacy.structure_id, None);
    }

    #[test]
    fn weekday_series_compare_recorded_types_in_calendar_order_without_dates() {
        let result = study().analyze(&SalesFilter::default()).unwrap();
        let evolution = &result.weekday_evolution;
        assert_eq!(evolution.total.iter().map(|point| point.weekday).collect::<Vec<_>>(), Weekday::ALL);
        assert_eq!(evolution.total[0].metrics.sales_count, 2);
        assert_eq!(evolution.total[0].metrics.revenue, Money(3001));
        assert_eq!(evolution.total[6].metrics.sales_count, 1);
        assert_eq!(evolution.total[1].metrics.sales_count, 0);
        assert_eq!(evolution.total[1].metrics.satisfaction_average, None);
        assert_eq!(evolution.unrecognized_weekday_rows, 0);
        for day in 0..7 {
            assert_eq!(
                evolution
                    .customer_types
                    .iter()
                    .map(|series| series.points[day].metrics.sales_count)
                    .sum::<usize>(),
                evolution.total[day].metrics.sales_count
            );
            assert_eq!(
                evolution
                    .customer_types
                    .iter()
                    .map(|series| series.points[day].metrics.revenue.0)
                    .sum::<i64>(),
                evolution.total[day].metrics.revenue.0
            );
        }
        let regular = evolution
            .customer_types
            .iter()
            .find(|series| series.customer_type == "Regular")
            .unwrap();
        assert_eq!(regular.points[6].metrics.satisfaction_average, Some(0.0));
        assert_eq!(regular.points[6].metrics.satisfaction_observed, 1);
        let filtered = study()
            .analyze(&SalesFilter {
                customer_type: Some("Student".into()),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(filtered.weekday_evolution.total[0].metrics.sales_count, 1);
        assert_eq!(filtered.weekday_evolution.customer_types.len(), 1);
        assert_eq!(Weekday::from_source(" monday "), Some(Weekday::Monday));
        assert_eq!(Weekday::from_source("2026-11-19"), None);
    }

    #[test]
    fn unrecognized_weekdays_remain_accounted_for_and_series_exports_preserve_labels() {
        let mut study = study();
        study.datasets[0]
            .records
            .push(sale("3", "Four cheese", "Unknown day", 800, 500, None));
        study.datasets[1].records[0].customer_type = "New;Group".into();
        let result = study.analyze(&SalesFilter::default()).unwrap();
        assert_eq!(result.totals.sales_count, 4);
        assert_eq!(result.weekday_evolution.unrecognized_weekday_rows, 1);
        assert_eq!(
            result
                .weekday_evolution
                .total
                .iter()
                .map(|point| point.metrics.sales_count)
                .sum::<usize>(),
            3
        );
        let csv = result.weekday_series_csv();
        assert!(csv.starts_with("Series;Customer Type;Weekday;Recorded sales;"));
        assert!(csv.contains("Customer type;\"New;Group\";Monday;1;9.99;6.99;3.00;4;1;0\n"));
        assert!(csv.contains("Unrecognized weekday;;Unknown day;1;8.00;5.00;3.00;;0;1\n"));
        assert_eq!(csv.lines().count(), 1 + 7 + 2 * 7 + 1);
    }

    #[test]
    fn kpis_explain_supported_denominators_and_keep_missing_distinct_from_zero() {
        let result = study().analyze(&SalesFilter::default()).unwrap();
        assert_eq!(result.totals.satisfaction_observation_rate(), Some(2.0 / 3.0));
        let kpis = result.kpis_csv();
        assert!(kpis.contains("Recorded margin rate;"));
        assert!(kpis.contains(";17.00;40.02;Profit / recorded revenue\n"));
        assert!(kpis.contains(";2;3;Rated sales / recorded sales\n"));
        let empty = SalesStudy::new("Empty").analyze(&SalesFilter::default()).unwrap();
        assert_eq!(empty.totals.satisfaction_observation_rate(), None);
        assert!(empty.kpis_csv().contains("Satisfaction observation rate;;0;0;"));
        let missing = SalesMetrics {
            sales_count: 1,
            satisfaction_missing: 1,
            ..Default::default()
        };
        assert_eq!(missing.satisfaction_observation_rate(), Some(0.0));
    }

    #[test]
    fn zero_revenue_has_no_margin_and_missing_satisfaction_has_no_average() {
        let mut study = SalesStudy::new("Zero revenue");
        study
            .add_dataset(SalesDataset {
                source_name: "zero.csv".into(),
                channel: "Counter".into(),
                records: vec![sale("1", "Sample", "Monday", 0, 123, None)],
            })
            .unwrap();
        let result = study.analyze(&SalesFilter::default()).unwrap();
        assert_eq!(result.totals.profit, Money(-123));
        assert_eq!(result.totals.average_revenue, Some(0.0));
        assert_eq!(result.totals.profit_rate, None);
        assert_eq!(result.totals.satisfaction_average, None);
        assert_eq!(result.totals.satisfaction_missing, 1);
    }

    #[test]
    fn duplicate_import_rejection_does_not_join_ids_across_channels() {
        let mut study = study();
        let mut relabeled = study.datasets[0].clone();
        relabeled.channel = "Edited channel".into();
        assert!(study.add_dataset(relabeled).is_err());
        let mut duplicate = study.datasets[0].clone();
        duplicate.source_name = "renamed.xlsx".into();
        assert!(study.add_dataset(duplicate.clone()).is_err());
        duplicate.channel = "Internet".into();
        assert!(study.add_dataset(duplicate).is_ok());
        assert_eq!(study.analyze(&SalesFilter::default()).unwrap().totals.sales_count, 5);
        assert_eq!(study.datasets[0].records[0].id, study.datasets[1].records[0].id);
    }

    #[test]
    fn checked_codec_preserves_money_and_rejects_invalid_or_unsupported_models() {
        let study = study();
        let encoded = SalesStudy::CODEC.encode(&study);
        let restored = SalesStudy::CODEC
            .decode(&Document::read(encoded.to_json().as_bytes()).unwrap())
            .unwrap();
        assert_eq!(restored, study);
        assert_eq!(encoded.model_version, 1);
        assert_eq!(encoded.kind, "process.sales");
        let mut invalid = encoded.clone();
        invalid.model["datasets"][0]["records"][0]["id"] = serde_json::json!("");
        assert!(SalesStudy::CODEC.decode(&invalid).is_err());
        invalid = encoded;
        invalid.model_version = 2;
        assert!(SalesStudy::CODEC.decode(&invalid).is_err());
        let mut invalid = study;
        invalid.datasets[0].records[0].satisfaction = Some(f64::NAN);
        assert!(invalid.check_storage().is_err());
        assert!(invalid.analyze(&SalesFilter::default()).is_err());
    }

    #[test]
    fn financial_overflow_is_an_error_and_exports_preserve_distinct_schemas() {
        let mut study = study();
        study.datasets[0].records[0].revenue = Money(i64::MAX);
        assert!(study.analyze(&SalesFilter::default()).is_err());
        let study = self::study();
        let raw = study
            .to_csv(&SalesFilter {
                channel: Some("Phone".into()),
                ..Default::default()
            })
            .unwrap();
        assert!(raw.starts_with("Channel;Source;Id;StructureId;"));
        assert_eq!(raw.lines().count(), 2);
        assert!(raw.contains("9.99;6.99;4;Monday;Paris;Student"));
        let grouped = study
            .analyze(&SalesFilter::default())
            .unwrap()
            .to_csv(SalesGroupBy::PizzaCustomerWeekday);
        assert!(grouped.starts_with("Pizza Type;Customer Type;Weekday;Recorded sales;"));
        assert_eq!(grouped.lines().count(), 4);
    }
}
