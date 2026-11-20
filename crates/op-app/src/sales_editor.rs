//! Sales datasets and quantitative analysis share the document shell without a canvas.
pub(crate) mod analysis_job;

use crate::editor::layout::DiagramLayout;
use crate::editor::*;
use crate::history::History;
use egui::{RichText, Ui};
use op_core::document::{Document, DocumentCodec, JsonCodec, LogDocument, LogImportNote};
use op_core::i18n::{Lang, key, tr, trf};
use op_core::sales::{
    Money, Sale, SalesAnalysis, SalesDataset, SalesFilter, SalesGroup, SalesGroupBy, SalesMetrics, SalesStudy, Weekday, WeekdayPoint,
};
use std::sync::{Arc, Mutex};

#[derive(Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum ChartMetric {
    #[default]
    Revenue,
    Count,
    Profit,
    Satisfaction,
}

impl ChartMetric {
    const ALL: [Self; 4] = [Self::Revenue, Self::Count, Self::Profit, Self::Satisfaction];
    fn label(self) -> &'static str {
        match self {
            Self::Revenue => key("sales.revenue"),
            Self::Count => key("sales.sales_count"),
            Self::Profit => key("sales.profit"),
            Self::Satisfaction => key("sales.satisfaction"),
        }
    }
    fn value(self, metrics: &SalesMetrics) -> Option<f64> {
        match self {
            Self::Revenue => Some(metrics.revenue.as_major()),
            Self::Count => Some(metrics.sales_count as f64),
            Self::Profit => Some(metrics.profit.as_major()),
            Self::Satisfaction => metrics.satisfaction_average,
        }
    }
}

#[derive(Clone)]
pub struct SalesView {
    uid: u64,
    inspector: Option<bool>,
    filter: SalesFilter,
    breakdown: bool,
    evolution: bool,
    group_by: SalesGroupBy,
    metric: ChartMetric,
    trend_metric: ChartMetric,
    result: Option<Arc<analysis_job::Output>>,
    // Cloned view states share one task; closing its last view releases the job.
    pending: Option<Arc<Mutex<Pending>>>,
}

impl Default for SalesView {
    fn default() -> Self {
        Self {
            uid: crate::canvas::next_uid(),
            inspector: None,
            filter: SalesFilter::default(),
            breakdown: false,
            evolution: false,
            group_by: SalesGroupBy::Channel,
            metric: ChartMetric::Revenue,
            trend_metric: ChartMetric::Count,
            result: None,
            pending: None,
        }
    }
}

struct Pending {
    generation: u64,
    filter: SalesFilter,
    lang: Lang,
    job: crate::platform::jobs::Job,
}

pub struct SalesEditor {
    pub study: SalesStudy,
    history: History<SalesStudy>,
    state: SalesView,
    files: crate::io::FileIo,
    status: Option<Result<String, String>>,
}

impl SalesEditor {
    pub fn new(study: SalesStudy) -> Self {
        Self {
            history: History::new(&study),
            study,
            state: SalesView::default(),
            files: crate::io::FileIo::default(),
            status: None,
        }
    }

    fn add_file(&mut self, name: &str, bytes: &[u8]) -> Result<usize, String> {
        let dataset = import_dataset(name, bytes)?.ok_or_else(|| tr("sales.no_sales").to_string())?;
        let count = dataset.records.len();
        self.history.sync(&self.study, false);
        self.study.add_dataset(dataset)?;
        self.history.sync(&self.study, false);
        Ok(count)
    }

    fn poll_files(&mut self) {
        for error in self.files.poll_errors() {
            self.status = Some(Err(error));
        }
        for (path, bytes) in self.files.poll() {
            let name = crate::io::file_name(&path);
            self.status = Some(match self.add_file(name, &bytes) {
                Ok(count) => Ok(trf("sales.import_added", &[&name, &count])),
                Err(error) => Err(trf("sales.import_failed", &[&name, &error])),
            });
        }
    }

    fn analyze(&mut self, ui: &Ui) {
        let generation = self.history.generation();
        let lang = op_core::i18n::lang();
        if self
            .state
            .result
            .as_ref()
            .is_some_and(|result| result.generation == generation && result.filter == self.state.filter && result.lang == lang)
        {
            return;
        }
        self.state.result = None;
        let current = self.state.pending.as_ref().is_some_and(|pending| {
            let pending = pending.lock().unwrap();
            pending.generation == generation && pending.filter == self.state.filter && pending.lang == lang
        });
        if !current {
            self.state.pending = None;
            let input = serde_json::to_string(&analysis_job::Input {
                task: "sales.analysis".into(),
                study: self.study.clone(),
                filter: self.state.filter.clone(),
                generation,
                lang,
            })
            .expect("validated sales documents are serializable");
            match crate::platform::jobs::Job::spawn(input, analysis_job::compute_json, ui.ctx()) {
                Ok(job) => {
                    self.state.pending = Some(Arc::new(Mutex::new(Pending {
                        generation,
                        filter: self.state.filter.clone(),
                        lang,
                        job,
                    })));
                }
                Err(error) => {
                    self.state.result = Some(Arc::new(analysis_job::Output {
                        generation,
                        filter: self.state.filter.clone(),
                        lang,
                        result: Err(error),
                    }));
                }
            }
        }
        let finished = self.state.pending.as_ref().and_then(|pending| pending.lock().unwrap().job.poll());
        if let Some(result) = finished {
            self.state.pending = None;
            let output = result.and_then(|text| serde_json::from_str(&text).map_err(|error| error.to_string()));
            self.state.result = Some(Arc::new(output.unwrap_or_else(|error| analysis_job::Output {
                generation,
                filter: self.state.filter.clone(),
                lang,
                result: Err(error),
            })));
        }
        if self.state.pending.is_some() {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
        }
    }

    fn properties(&mut self, ui: &mut Ui, exam: bool) {
        DiagramLayout::inspector_body(ui, |ui| {
            ui.heading(tr("sales.datasets"));
            ui.label(tr("common.name"));
            ui.text_edit_singleline(&mut self.study.name);
            let mut remove = None;
            for (index, dataset) in self.study.datasets.iter_mut().enumerate() {
                ui.push_id(index, |ui| {
                    ui.separator();
                    ui.label(tr("sales.source"));
                    ui.label(RichText::new(&dataset.source_name).strong());
                    ui.label(trf("sales.source_records", &[&dataset.records.len()]));
                    let label = ui.label(tr("sales.channel"));
                    ui.text_edit_singleline(&mut dataset.channel).labelled_by(label.id);
                    if ui.small_button(tr("sales.remove_dataset")).clicked() {
                        remove = Some(index);
                    }
                });
            }
            if let Some(index) = remove {
                self.study.datasets.remove(index);
            }
            if exam {
                return;
            }
            ui.separator();
            ui.heading(tr("sales.filters"));
            filter_text(ui, key("sales.channel"), &mut self.state.filter.channel);
            filter_text(ui, key("sales.structure_id"), &mut self.state.filter.structure_id);
            ui.label(RichText::new(tr("sales.scenario_help")).small().weak());
            filter_text(ui, key("sales.customer_type"), &mut self.state.filter.customer_type);
            filter_text(ui, key("sales.weekday"), &mut self.state.filter.weekday);
            filter_text(ui, key("sales.customer_location"), &mut self.state.filter.customer_location);
            filter_text(ui, key("sales.pizza_type"), &mut self.state.filter.pizza_type);
            ui.label(RichText::new(tr("sales.filters_help")).small().weak());
        });
    }

    fn content(&mut self, ui: &mut Ui, exam: bool) {
        if exam {
            ui.label(tr("exam.assistance_disabled"));
            return;
        }
        if self.study.datasets.is_empty() {
            ui.label(tr("sales.empty"));
            return;
        }
        self.analyze(ui);
        let Some(cached) = self.state.result.as_ref().cloned() else {
            ui.spinner();
            ui.label(tr("sales.preparing"));
            return;
        };
        let analysis = match &cached.result {
            Ok(analysis) => analysis,
            Err(error) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
                return;
            }
        };
        ui.horizontal_wrapped(|ui| {
            if ui
                .selectable_label(!self.state.breakdown && !self.state.evolution, tr("sales.summary"))
                .clicked()
            {
                self.state.breakdown = false;
                self.state.evolution = false;
            }
            if ui
                .selectable_label(self.state.breakdown && !self.state.evolution, tr("sales.breakdown"))
                .clicked()
            {
                self.state.breakdown = true;
                self.state.evolution = false;
            }
            if ui.selectable_label(self.state.evolution, tr("sales.evolution")).clicked() {
                self.state.evolution = true;
                self.state.breakdown = false;
            }
        });
        ui.separator();
        if analysis.totals.sales_count == 0 {
            ui.label(tr("sales.no_matches"));
            return;
        }
        if self.state.evolution {
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("sales.chart_metric"));
                egui::ComboBox::from_id_salt("sales_trend_metric")
                    .selected_text(tr(self.state.trend_metric.label()))
                    .show_ui(ui, |ui| {
                        for metric in ChartMetric::ALL {
                            ui.selectable_value(&mut self.state.trend_metric, metric, tr(metric.label()));
                        }
                    });
            });
            egui::ScrollArea::vertical().show(ui, |ui| weekday_evolution(ui, analysis, self.state.trend_metric));
        } else if !self.state.breakdown {
            egui::ScrollArea::vertical().show(ui, |ui| {
                summary(ui, analysis);
                ui.separator();
                ui.heading(format!("{} · {}", tr("sales.channel"), tr(self.state.metric.label())));
                bar_chart(ui, analysis.groups(SalesGroupBy::Channel), self.state.metric);
                if analysis.groups(SalesGroupBy::Channel).len() > 20 {
                    ui.label(RichText::new(tr("sales.chart_limit")).small().weak());
                }
                ui.label(RichText::new(tr("sales.limits_note")).small().weak());
                ui.label(RichText::new(tr("sales.units_note")).small().weak());
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("sales.group_by"));
                egui::ComboBox::from_id_salt("sales_group_by")
                    .selected_text(tr(group_label(self.state.group_by)))
                    .show_ui(ui, |ui| {
                        for by in SalesGroupBy::ALL {
                            ui.selectable_value(&mut self.state.group_by, by, tr(group_label(by)));
                        }
                    });
                ui.label(tr("sales.chart_metric"));
                egui::ComboBox::from_id_salt("sales_metric")
                    .selected_text(tr(self.state.metric.label()))
                    .show_ui(ui, |ui| {
                        for metric in ChartMetric::ALL {
                            ui.selectable_value(&mut self.state.metric, metric, tr(metric.label()));
                        }
                    });
            });
            let groups = analysis.groups(self.state.group_by);
            egui::CollapsingHeader::new(tr("sales.chart_metric"))
                .default_open(true)
                .show(ui, |ui| {
                    bar_chart(ui, groups, self.state.metric);
                    ui.label(RichText::new(tr("sales.chart_limit")).small().weak());
                });
            group_table(ui, groups);
        }
    }

    fn current_analysis(&self) -> Result<&SalesAnalysis, String> {
        let cached = self
            .state
            .result
            .as_ref()
            .filter(|cached| {
                cached.generation == self.history.generation() && cached.filter == self.state.filter && cached.lang == op_core::i18n::lang()
            })
            .ok_or_else(|| tr("sales.preparing").to_string())?;
        cached.result.as_ref().map_err(Clone::clone)
    }
}

fn filter_text(ui: &mut Ui, label: &'static str, value: &mut Option<String>) {
    let label = ui.label(tr(label));
    let mut text = value.clone().unwrap_or_default();
    if ui
        .add(egui::TextEdit::singleline(&mut text).hint_text(tr("sales.all")))
        .labelled_by(label.id)
        .changed()
    {
        *value = (!text.is_empty()).then_some(text);
    }
}

fn group_label(by: SalesGroupBy) -> &'static str {
    match by {
        SalesGroupBy::Channel => key("sales.channel"),
        SalesGroupBy::Weekday => key("sales.weekday"),
        SalesGroupBy::CustomerType => key("sales.customer_type"),
        SalesGroupBy::CustomerLocation => key("sales.customer_location"),
        SalesGroupBy::PizzaType => key("sales.pizza_type"),
        SalesGroupBy::PizzaSize => key("sales.pizza_size"),
        SalesGroupBy::PizzaCustomerWeekday => key("sales.pizza_customer_weekday"),
    }
}

fn decimal(value: Option<f64>) -> String {
    value.map(|value| format!("{value:.2}")).unwrap_or_else(|| "—".into())
}

fn percentage(value: Option<f64>) -> String {
    value.map(|value| format!("{:.2}%", value * 100.0)).unwrap_or_else(|| "—".into())
}

fn summary(ui: &mut Ui, analysis: &SalesAnalysis) {
    let metrics = &analysis.totals;
    egui::Grid::new("sales_totals").striped(true).num_columns(2).show(ui, |ui| {
        for (label, value) in [
            (key("sales.sales_count"), metrics.sales_count.to_string()),
            (key("sales.revenue"), metrics.revenue.to_string()),
            (key("sales.cost"), metrics.cost.to_string()),
            (key("sales.profit"), metrics.profit.to_string()),
            (key("sales.average_revenue"), decimal(metrics.average_revenue)),
            (key("sales.satisfaction"), decimal(metrics.satisfaction_average)),
        ] {
            ui.label(tr(label));
            ui.label(RichText::new(value).strong());
            ui.end_row();
        }
    });
    ui.label(trf(
        "sales.satisfaction_note",
        &[&metrics.satisfaction_observed, &metrics.satisfaction_missing],
    ));
    ui.separator();
    ui.heading(tr("sales.kpis"));
    for (label, value, help) in [
        (
            key("sales.kpi_margin"),
            percentage(metrics.profit_rate),
            key("sales.kpi_margin_help"),
        ),
        (
            key("sales.kpi_coverage"),
            percentage(metrics.satisfaction_observation_rate()),
            key("sales.kpi_coverage_help"),
        ),
    ] {
        ui.group(|ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(tr(label)).strong());
                ui.label(RichText::new(value).strong());
            });
            ui.label(RichText::new(tr(help)).small().weak());
        });
    }
}

fn weekday_label(day: Weekday) -> &'static str {
    match day {
        Weekday::Monday => key("sales.day_monday"),
        Weekday::Tuesday => key("sales.day_tuesday"),
        Weekday::Wednesday => key("sales.day_wednesday"),
        Weekday::Thursday => key("sales.day_thursday"),
        Weekday::Friday => key("sales.day_friday"),
        Weekday::Saturday => key("sales.day_saturday"),
        Weekday::Sunday => key("sales.day_sunday"),
    }
}

fn metric_text(metric: ChartMetric, metrics: Option<&SalesMetrics>) -> String {
    let value = metrics.and_then(|metrics| metric.value(metrics));
    if metric == ChartMetric::Count {
        value.map(|value| format!("{value:.0}")).unwrap_or_else(|| "—".into())
    } else {
        decimal(value)
    }
}

fn weekday_evolution(ui: &mut Ui, analysis: &SalesAnalysis, metric: ChartMetric) {
    let evolution = &analysis.weekday_evolution;
    ui.label(tr("sales.weekday_profile_note"));
    if evolution.unrecognized_weekday_rows > 0 {
        ui.label(RichText::new(trf("sales.weekday_unclassified", &[&evolution.unrecognized_weekday_rows])).weak());
    }
    let total_name = tr("sales.weekday_total");
    ui.heading(total_name);
    line_chart(ui, &[(total_name, &evolution.total)], metric);
    ui.heading(tr("sales.customer_weekday_lines"));
    let labels: Vec<_> = evolution
        .customer_types
        .iter()
        .take(20)
        .map(|series| {
            if series.customer_type.is_empty() {
                tr("sales.unknown_customer_type").to_owned()
            } else {
                series.customer_type.clone()
            }
        })
        .collect();
    let series: Vec<_> = evolution
        .customer_types
        .iter()
        .take(20)
        .zip(&labels)
        .map(|(series, label)| (label.as_str(), series.points.as_slice()))
        .collect();
    line_chart(ui, &series, metric);
    if evolution.customer_types.len() > 20 {
        ui.label(RichText::new(tr("sales.weekday_series_limit")).small().weak());
    }
    egui::ScrollArea::horizontal().id_salt("sales_weekday_table").show(ui, |ui| {
        egui::Grid::new("sales_weekday_values").striped(true).show(ui, |ui| {
            ui.label(RichText::new(tr("sales.weekday")).strong());
            ui.label(RichText::new(total_name).strong());
            for label in &labels {
                ui.label(RichText::new(label).strong());
            }
            ui.end_row();
            for (index, day) in Weekday::ALL.into_iter().enumerate() {
                ui.label(tr(weekday_label(day)));
                ui.label(metric_text(metric, evolution.total.get(index).map(|point| &point.metrics)));
                for series in evolution.customer_types.iter().take(20) {
                    ui.label(metric_text(metric, series.points.get(index).map(|point| &point.metrics)));
                }
                ui.end_row();
            }
        });
    });
    ui.label(RichText::new(tr("sales.units_note")).small().weak());
}

fn line_chart(ui: &mut Ui, series: &[(&str, &[WeekdayPoint])], metric: ChartMetric) {
    let colors = [
        crate::theme::accent(ui),
        egui::Color32::from_rgb(61, 132, 203),
        egui::Color32::from_rgb(216, 120, 38),
        egui::Color32::from_rgb(140, 95, 199),
        egui::Color32::from_rgb(189, 77, 112),
        egui::Color32::from_rgb(40, 151, 115),
    ];
    ui.horizontal_wrapped(|ui| {
        for (index, (label, _)) in series.iter().enumerate() {
            ui.colored_label(colors[index % colors.len()], "●");
            ui.label(*label);
        }
    });
    let values: Vec<_> = series
        .iter()
        .flat_map(|(_, points)| points.iter().filter_map(|point| metric.value(&point.metrics)))
        .collect();
    let min = values.iter().copied().fold(0.0_f64, f64::min);
    let max = values.iter().copied().fold(0.0_f64, f64::max);
    let max = if metric == ChartMetric::Count {
        ((max / 4.0).ceil() * 4.0).max(4.0)
    } else {
        max
    };
    // Scaling first keeps even finite extreme imported scores from overflowing
    // the subtraction used to map values to the chart's finite coordinates.
    let scale = min.abs().max(max.abs()).max(1.0);
    let low = min / scale;
    let high = max / scale;
    let span = (high - low).max(f64::EPSILON);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(ui.available_width().max(1.0), 220.0), egui::Sense::hover());
    let left = rect.left() + (rect.width() * 0.12).min(52.0);
    let plot = egui::Rect::from_min_max(
        egui::pos2(left, rect.top() + 8.0),
        egui::pos2((rect.right() - 5.0).max(left + 1.0), rect.bottom() - 30.0),
    );
    let at = |index: usize, value: f64| {
        egui::pos2(
            plot.left() + index as f32 * plot.width() / 6.0,
            plot.bottom() - ((value / scale - low) / span) as f32 * plot.height(),
        )
    };
    let weak = ui.visuals().weak_text_color();
    let stroke = ui.visuals().widgets.noninteractive.bg_stroke;
    for step in 0..=4 {
        let fraction = step as f32 / 4.0;
        let y = plot.bottom() - fraction * plot.height();
        ui.painter()
            .line_segment([egui::pos2(plot.left(), y), egui::pos2(plot.right(), y)], stroke);
        let value = low * (1.0 - f64::from(fraction)) * scale + high * f64::from(fraction) * scale;
        ui.painter().text(
            egui::pos2(plot.left() - 4.0, y),
            egui::Align2::RIGHT_CENTER,
            if metric == ChartMetric::Count {
                format!("{value:.0}")
            } else {
                format!("{value:.2}")
            },
            egui::FontId::proportional(11.0),
            weak,
        );
    }
    for (index, day) in Weekday::ALL.into_iter().enumerate() {
        let label: String = tr(weekday_label(day)).chars().take(3).collect();
        ui.painter().text(
            egui::pos2(at(index, 0.0).x, plot.bottom() + 10.0),
            egui::Align2::CENTER_TOP,
            label,
            egui::FontId::proportional(11.0),
            weak,
        );
    }
    for (index, (_, points)) in series.iter().enumerate() {
        let color = colors[index % colors.len()];
        let mut previous = None;
        for (day, point) in points.iter().take(7).enumerate() {
            if let Some(value) = metric.value(&point.metrics) {
                let position = at(day, value);
                if let Some(previous) = previous {
                    ui.painter().line_segment([previous, position], egui::Stroke::new(2.0, color));
                }
                ui.painter().circle_filled(position, 3.0, color);
                previous = Some(position);
            } else {
                previous = None;
            }
        }
    }
    if let Some(pointer) = response.hover_pos() {
        let day = (((pointer.x - plot.left()) / plot.width()) * 6.0).round().clamp(0.0, 6.0) as usize;
        response.on_hover_ui(|ui| {
            ui.label(RichText::new(tr(weekday_label(Weekday::ALL[day]))).strong());
            for (label, points) in series {
                ui.label(format!(
                    "{label}: {}",
                    metric_text(metric, points.get(day).map(|point| &point.metrics))
                ));
            }
        });
    }
}

fn group_table(ui: &mut Ui, groups: &[SalesGroup]) {
    egui::ScrollArea::horizontal().id_salt("sales_group_columns").show(ui, |ui| {
        ui.horizontal(|ui| {
            for label in [
                key("sales.category"),
                key("sales.sales_count"),
                key("sales.revenue"),
                key("sales.cost"),
                key("sales.profit"),
                key("sales.satisfaction"),
                key("sales.satisfaction_observed"),
                key("sales.satisfaction_missing"),
            ] {
                ui.add_sized(
                    [if label == "sales.category" { 200.0 } else { 100.0 }, 24.0],
                    egui::Label::new(RichText::new(tr(label)).strong()),
                );
            }
        });
        egui::ScrollArea::vertical()
            .id_salt("sales_groups")
            .show_rows(ui, 24.0, groups.len(), |ui, range| {
                for index in range {
                    let group = &groups[index];
                    ui.horizontal(|ui| {
                        ui.add_sized([200.0, 24.0], egui::Label::new(group.key.label()).truncate());
                        for value in [
                            group.metrics.sales_count.to_string(),
                            group.metrics.revenue.to_string(),
                            group.metrics.cost.to_string(),
                            group.metrics.profit.to_string(),
                            decimal(group.metrics.satisfaction_average),
                            group.metrics.satisfaction_observed.to_string(),
                            group.metrics.satisfaction_missing.to_string(),
                        ] {
                            ui.add_sized([100.0, 24.0], egui::Label::new(value));
                        }
                    });
                }
            });
    });
}

fn bar_chart(ui: &mut Ui, groups: &[SalesGroup], metric: ChartMetric) {
    let values: Vec<_> = groups
        .iter()
        .take(20)
        .map(|group| (group.key.label(), metric.value(&group.metrics)))
        .collect();
    let min = values.iter().filter_map(|(_, value)| *value).fold(0.0_f64, f64::min);
    let max = values.iter().filter_map(|(_, value)| *value).fold(0.0_f64, f64::max);
    let span = (max - min).max(f64::EPSILON);
    let available = ui.available_width();
    let label_width = (available * 0.35).clamp(70.0, 200.0);
    let graph_width = (available - label_width - 80.0).max(20.0);
    for (label, value) in values {
        ui.horizontal(|ui| {
            ui.add_sized([label_width, 22.0], egui::Label::new(label).truncate());
            let (rect, _) = ui.allocate_exact_size(egui::vec2(graph_width, 18.0), egui::Sense::hover());
            let x = |value: f64| rect.left() + (((value - min) / span) as f32 * rect.width());
            let zero = x(0.0);
            ui.painter().line_segment(
                [egui::pos2(zero, rect.top()), egui::pos2(zero, rect.bottom())],
                ui.visuals().widgets.noninteractive.bg_stroke,
            );
            if let Some(value) = value {
                let at = x(value);
                let color = if value < 0.0 {
                    ui.visuals().error_fg_color
                } else {
                    crate::theme::accent(ui)
                };
                ui.painter().rect_filled(
                    egui::Rect::from_min_max(egui::pos2(zero.min(at), rect.top()), egui::pos2(zero.max(at), rect.bottom())),
                    2.0,
                    color,
                );
            }
            ui.label(if metric == ChartMetric::Count {
                value.map(|v| format!("{v:.0}")).unwrap_or_else(|| "—".into())
            } else {
                decimal(value)
            });
        });
    }
}

fn import_dataset(name: &str, bytes: &[u8]) -> Result<Option<SalesDataset>, String> {
    if bytes.starts_with(b"PK\x03\x04") {
        op_core::sales::import::import_sales_workbook(bytes, name)
    } else {
        let text = std::str::from_utf8(bytes).map_err(|error| error.to_string())?;
        op_core::sales::import::import_sales_csv(text, name)
    }
}

const COMMANDS: &[CommandDescriptor] = &[
    CommandDescriptor {
        id: UNDO,
        label: key("common.undo"),
        group: CommandGroup::Edit,
    },
    CommandDescriptor {
        id: REDO,
        label: key("common.redo"),
        group: CommandGroup::Edit,
    },
];

impl EditorBackend for SalesEditor {
    type ViewState = SalesView;
    fn capture_view(&self) -> SalesView {
        self.state.clone()
    }
    fn restore_view(&mut self, state: &SalesView) {
        self.state = state.clone();
    }
    fn fresh_view(&self) -> SalesView {
        SalesView::default()
    }
    fn save_view(state: &SalesView) -> serde_json::Value {
        serde_json::json!({"inspector":state.inspector,"filter":state.filter,"breakdown":state.breakdown,"evolution":state.evolution,"group_by":state.group_by,"metric":state.metric,"trend_metric":state.trend_metric})
    }
    fn load_view(state: &mut SalesView, value: &serde_json::Value) {
        state.inspector = value["inspector"].as_bool();
        if let Ok(filter) = serde_json::from_value(value["filter"].clone()) {
            state.filter = filter;
        }
        if let Some(breakdown) = value["breakdown"].as_bool() {
            state.breakdown = breakdown;
        }
        state.evolution = value["evolution"].as_bool().unwrap_or(false);
        if let Ok(by) = serde_json::from_value(value["group_by"].clone()) {
            state.group_by = by;
        }
        if let Ok(metric) = serde_json::from_value(value["metric"].clone()) {
            state.metric = metric;
        }
        state.trend_metric = serde_json::from_value(value["trend_metric"].clone()).unwrap_or(ChartMetric::Count);
    }
    fn set_exam_mode(&mut self, active: bool) {
        if active {
            self.state.pending = None;
            self.state.result = None;
        }
    }
    fn kind(&self) -> &'static str {
        "process.sales"
    }
    fn name(&self) -> &str {
        &self.study.name
    }
    fn rename(&mut self, name: String) {
        self.history.sync(&self.study, false);
        self.study.name = name;
        self.history.sync(&self.study, false);
    }
    fn content_generation(&self) -> Option<u64> {
        Some(self.history.generation())
    }
    fn has_content(&self) -> bool {
        !self.study.datasets.is_empty()
    }
    fn validate_storage(&self) -> Result<(), String> {
        self.study.check_storage()
    }
    fn document(&self) -> Document {
        SalesStudy::CODEC.encode(&self.study)
    }
    fn cancel_edit(&mut self) {
        self.history.cancel(&mut self.study);
    }
    fn ui(&mut self, ui: &mut Ui, _: &str) -> Vec<HostEvent> {
        self.poll_files();
        let exam = crate::editor::exam::active(ui.ctx());
        // Sources and filters live in the inspector: keep it open like Mermaid and DMN.
        let layout = DiagramLayout::with_primary_inspector(ui, self.state.uid, self.state.inspector);
        layout.tools(ui, |ui| {
            if ui.button(tr("sales.add_file")).clicked() {
                self.files.open_dialog(ui.ctx(), vec!["xlsx", "csv"]);
            }
            if ui.selectable_label(layout.inspector_visible, tr("common.inspector")).clicked() {
                self.state.inspector = Some(!layout.inspector_visible);
            }
            if let Some(status) = &self.status {
                match status {
                    Ok(message) => {
                        ui.label(message);
                    }
                    Err(error) => {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                }
            }
        });
        layout.inspector(ui, |ui| self.properties(ui, exam));
        self.history.sync(
            &self.study,
            ui.ctx().egui_wants_keyboard_input() || ui.input(|input| input.pointer.any_down()),
        );
        egui::CentralPanel::default().show(ui, |ui| self.content(ui, exam));
        vec![]
    }
    fn commands(&self) -> &'static [CommandDescriptor] {
        COMMANDS
    }
    fn enabled(&self, command: &str) -> bool {
        match command {
            UNDO => self.history.can_undo(),
            REDO => self.history.can_redo(),
            _ => false,
        }
    }
    fn execute(&mut self, command: &str, _: f64) -> Vec<HostEvent> {
        match command {
            UNDO => self.history.undo(&mut self.study),
            REDO => self.history.redo(&mut self.study),
            _ => {}
        }
        self.state.pending = None;
        self.state.result = None;
        vec![]
    }
    fn export(&self) -> Result<ExportData, String> {
        let analysis = self.current_analysis()?;
        Ok(ExportData {
            suffix: "csv",
            bytes: analysis.to_csv(self.state.group_by).into_bytes(),
        })
    }
    fn export_label(&self) -> &'static str {
        key("sales.export_csv")
    }
    fn exports(&self) -> Vec<ExportDescriptor> {
        vec![
            ExportDescriptor {
                id: "standard",
                label: key("sales.export_csv"),
            },
            ExportDescriptor {
                id: "rows",
                label: key("sales.export_rows"),
            },
            ExportDescriptor {
                id: "weekday-series",
                label: key("sales.export_weekday"),
            },
            ExportDescriptor {
                id: "kpis",
                label: key("sales.export_kpis"),
            },
        ]
    }
    fn export_as(&self, format: &str) -> Result<ExportData, String> {
        if format == "rows" {
            return Ok(ExportData {
                suffix: "csv",
                bytes: self.study.to_csv(&self.state.filter)?.into_bytes(),
            });
        }
        if format == "standard" {
            return self.export();
        }
        if format == "weekday-series" || format == "kpis" {
            let analysis = self.current_analysis()?;
            return Ok(ExportData {
                suffix: "csv",
                bytes: if format == "weekday-series" {
                    analysis.weekday_series_csv()
                } else {
                    analysis.kpis_csv()
                }
                .into_bytes(),
            });
        }
        Err("Unknown sales export".into())
    }
    fn export_format_requires_assistance(&self, format: &str) -> bool {
        format != "rows"
    }
}

pub fn register(registry: &mut DiagramRegistry) {
    registry.diagrams.push(DiagramDescriptor {
        id: "process.sales",
        aliases: &[],
        home_order: 12,
        icon: crate::icons::decision_table,
        new_label: key("sales.new"),
        title: key("sales.title"),
        description: key("sales.description"),
        badge: "SALES",
        color: egui::Color32::from_rgb(33, 134, 116),
        create: |number| SalesStudy::CODEC.encode(&SalesStudy::new(trf("sales.document_name", &[&number]))),
        open: |document| {
            let study = SalesStudy::CODEC.decode(document)?;
            Ok(Session::boxed(SalesEditor::new(study)))
        },
    });
    registry.examples.push(ExampleDescriptor {
        id: "sales.example",
        kind: "process.sales",
        label: key("sales.example"),
        create: || SalesStudy::CODEC.encode(&example()),
    });
    registry.binary_imports.push(BinaryImportDescriptor {
        extensions: &["xlsx", "csv"],
        accepts: |bytes| bytes.starts_with(b"PK\x03\x04") || std::str::from_utf8(bytes).is_ok_and(op_core::sales::import::is_sales_csv),
        decode: |bytes, source| {
            let name = source.unwrap_or("");
            if let Some(dataset) = import_dataset(name, bytes)? {
                let title = if name.is_empty() {
                    dataset.source_name.clone()
                } else {
                    name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name).to_string()
                };
                let mut study = SalesStudy::new(title);
                study.add_dataset(dataset)?;
                return Ok(SalesStudy::CODEC.encode(&study));
            }
            let text = op_core::mining::import::import_xlsx_event_log(bytes)?;
            if text.len() > 2 * 1024 * 1024 {
                return Err(tr("mining.analysis_limit").into());
            }
            Ok(JsonCodec::new("event-log").encode(&LogDocument {
                name: if name.is_empty() {
                    "Imported log".into()
                } else {
                    name.rsplit_once('.').map(|(stem, _)| stem).unwrap_or(name).to_string()
                },
                text,
                discovery: op_core::mining::DiscoveryOptions {
                    normalize_boundaries: true,
                },
                import_note: Some(LogImportNote::WorkbookEventLog),
                ..Default::default()
            }))
        },
    });
}

fn example() -> SalesStudy {
    let mut study = SalesStudy::new(tr("sales.example"));
    study.datasets.push(SalesDataset {
        source_name: "Example.csv".into(),
        channel: "Counter".into(),
        records: vec![Sale {
            id: "1".into(),
            structure_id: "1".into(),
            pizza_type: "Margherita".into(),
            pizza_size: "Medium".into(),
            revenue: Money(1500),
            cost: Money(600),
            satisfaction: Some(4.0),
            weekday: "Monday".into(),
            customer_location: "Centre".into(),
            customer_type: "Regular".into(),
        }],
    });
    study
}

#[cfg(test)]
mod tests;
