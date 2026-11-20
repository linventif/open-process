use super::*;
use std::sync::atomic::AtomicBool;

const CSV: &str = "Id;StructureId;Pizza Type;Pizza Size;Revenue;Cost;Satisfaction;Weekday;Customer Location;Customer Type\n1;1;Margherita;Medium;15;6;4;Monday;Centre;Regular\n2;1;Pepperoni;Large;20;8;;Tuesday;North;New\n";

#[test]
fn adding_sales_sources_is_undoable_and_does_not_silently_double_count() {
    let mut editor = SalesEditor::new(SalesStudy::new("Study"));
    assert_eq!(editor.add_file("Counter.csv", CSV.as_bytes()).unwrap(), 2);
    assert!(editor.add_file("Counter.csv", CSV.as_bytes()).is_err());
    assert_eq!(editor.study.datasets.len(), 1);
    editor.add_file("Phone.csv", CSV.as_bytes()).unwrap();
    assert_eq!(
        editor.study.datasets.len(),
        2,
        "independent channels can contain matching recorded values"
    );
    editor.execute(UNDO, 0.0);
    assert_eq!(editor.study.datasets.len(), 1);
    editor.execute(REDO, 0.0);
    assert_eq!(editor.study.datasets.len(), 2);
    let saved = editor.document();
    editor.state.filter.channel = Some("Counter".into());
    assert_eq!(
        editor.document(),
        saved,
        "filters belong to the view rather than saved domain content"
    );
    assert_eq!(SalesStudy::CODEC.decode(&saved).unwrap(), editor.study);
}

#[test]
fn sales_worker_returns_filtered_metrics_and_preserves_request_identity() {
    let mut study = SalesStudy::new("Study");
    study
        .add_dataset(import_dataset("Counter.csv", CSV.as_bytes()).unwrap().unwrap())
        .unwrap();
    let filter = SalesFilter {
        weekday: Some("Monday".into()),
        ..Default::default()
    };
    let input = serde_json::to_string(&analysis_job::Input {
        task: "sales.analysis".into(),
        study,
        filter: filter.clone(),
        generation: 42,
        lang: Lang::En,
    })
    .unwrap();
    let result = analysis_job::compute_json(input.clone(), Arc::new(AtomicBool::new(false))).unwrap();
    let output: analysis_job::Output = serde_json::from_str(&result).unwrap();
    assert_eq!(output.generation, 42);
    assert_eq!(output.filter, filter);
    assert_eq!(output.lang, Lang::En);
    let analysis = output.result.unwrap();
    assert_eq!(analysis.weekday_evolution.total.len(), 7);
    assert_eq!(analysis.weekday_evolution.total[0].metrics.sales_count, 1);
    assert_eq!(analysis.weekday_evolution.total[1].metrics.sales_count, 0);
    assert_eq!(analysis.weekday_evolution.customer_types[0].customer_type, "Regular");
    assert_eq!(analysis.totals.satisfaction_observation_rate(), Some(1.0));
    let metrics = analysis.totals;
    assert_eq!(metrics.sales_count, 1);
    assert_eq!(metrics.revenue, Money(1500));
    assert_eq!(metrics.profit, Money(900));
    assert_eq!(metrics.satisfaction_observed, 1);
    assert_eq!(metrics.satisfaction_missing, 0);
    assert!(analysis_job::compute_json(input, Arc::new(AtomicBool::new(true))).is_err());
}

#[test]
fn exports_reject_stale_filters_and_use_cached_grouped_analysis() {
    let mut editor = SalesEditor::new(example());
    editor.state.result = Some(Arc::new(analysis_job::Output {
        generation: editor.history.generation(),
        filter: SalesFilter::default(),
        lang: op_core::i18n::lang(),
        result: editor.study.analyze(&SalesFilter::default()),
    }));
    assert!(String::from_utf8(editor.export().unwrap().bytes).unwrap().contains("Counter"));
    let series = String::from_utf8(editor.export_as("weekday-series").unwrap().bytes).unwrap();
    assert!(series.starts_with("Series;Customer Type;Weekday;Recorded sales;"));
    assert!(series.contains("Customer type;Regular;Monday;1;15.00;6.00;9.00;4;1;0\n"));
    let kpis = String::from_utf8(editor.export_as("kpis").unwrap().bytes).unwrap();
    assert!(kpis.starts_with("KPI;Value (fraction);Numerator;Denominator;Definition\n"));
    assert!(kpis.contains("Satisfaction observation rate;1;1;1;"));
    editor.state.filter.channel = Some("Other".into());
    assert!(editor.export().is_err());
    assert!(editor.export_as("weekday-series").is_err());
    assert!(editor.export_as("kpis").is_err());
    assert!(!editor.export_format_requires_assistance("rows"));
    assert!(editor.export_format_requires_assistance("standard"));
    assert!(editor.export_format_requires_assistance("weekday-series"));
    assert!(editor.export_format_requires_assistance("kpis"));
}

#[test]
fn weekday_view_preferences_roundtrip_without_changing_study_or_legacy_views() {
    let mut editor = SalesEditor::new(example());
    let original = editor.document();
    editor.state.evolution = true;
    editor.state.trend_metric = ChartMetric::Profit;
    editor.state.filter.customer_type = Some("Regular".into());
    editor.state.filter.structure_id = Some("7".into());
    let saved = SalesEditor::save_view(&editor.state);
    let mut restored = SalesView::default();
    SalesEditor::load_view(&mut restored, &saved);
    assert!(restored.evolution);
    assert!(restored.trend_metric == ChartMetric::Profit);
    assert_eq!(restored.filter.customer_type.as_deref(), Some("Regular"));
    assert_eq!(restored.filter.structure_id.as_deref(), Some("7"));
    assert_eq!(editor.document(), original);
    SalesEditor::load_view(
        &mut restored,
        &serde_json::json!({"filter":{"channel":"Counter"},"breakdown":true,"group_by":"Weekday","metric":"Revenue"}),
    );
    assert!(restored.breakdown);
    assert!(!restored.evolution);
    assert!(restored.trend_metric == ChartMetric::Count);
    assert_eq!(restored.filter.structure_id, None);
}

#[test]
fn scenario_filter_worker_keeps_local_identifiers_separate_from_sale_ids() {
    let source = CSV.replace("2;1;Pepperoni", "2;7;Pepperoni");
    let mut editor = SalesEditor::new(SalesStudy::new("Scenarios"));
    editor.add_file("Counter.csv", source.as_bytes()).unwrap();
    editor.add_file("Phone.csv", source.as_bytes()).unwrap();
    let saved = editor.document();
    let filter = SalesFilter {
        channel: Some("Counter".into()),
        structure_id: Some("7".into()),
        ..Default::default()
    };
    let input = serde_json::to_string(&analysis_job::Input {
        task: "sales.analysis".into(),
        study: editor.study.clone(),
        filter: filter.clone(),
        generation: 1,
        lang: Lang::En,
    })
    .unwrap();
    let raw = analysis_job::compute_json(input, Arc::new(AtomicBool::new(false))).unwrap();
    let output: analysis_job::Output = serde_json::from_str(&raw).unwrap();
    assert_eq!(output.filter, filter);
    let totals = output.result.unwrap().totals;
    assert_eq!(totals.sales_count, 1);
    assert_eq!(totals.revenue, Money(2000));
    assert_eq!(totals.satisfaction_observed, 0);
    assert_eq!(totals.satisfaction_missing, 1);
    assert_eq!(editor.document(), saved);
}

#[test]
fn named_workbook_import_preserves_provenance_and_dispatches_plain_logs_separately() {
    let registry = crate::composition::registry();
    let sales = registry
        .load_named(
            "Counter.xlsx",
            include_bytes!("../../../op-core/tests/fixtures/sales/tiny-sales.xlsx"),
        )
        .unwrap();
    assert_eq!(sales.kind, "process.sales");
    let study = SalesStudy::CODEC.decode(&sales).unwrap();
    assert_eq!(study.datasets[0].source_name, "Counter.xlsx");
    assert_eq!(study.datasets[0].channel, "Counter");
    assert_eq!(study.datasets[0].records.len(), 3);
    let log = registry
        .load_named(
            "events.xlsx",
            include_bytes!("../../../op-core/tests/fixtures/mining/tiny-event-log.xlsx"),
        )
        .unwrap();
    assert_eq!(log.kind, "event-log");
}
