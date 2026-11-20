#!/usr/bin/env python3
"""Independently analyse the recorded TakeAway sales and render report figures.

Run with a Python environment containing matplotlib. Calculations use integer
minor units and Decimal; Rust's precomputed metrics are comparison inputs only.
No customer/order identity, calendar date, currency or rating scale is inferred.
"""

from __future__ import annotations

import argparse
import csv
import hashlib
import io
import json
import math
import os
from collections import defaultdict
from decimal import Decimal, InvalidOperation, localcontext
from pathlib import Path
from typing import Any, Iterable


ROOT = Path(__file__).resolve().parents[1]
DEFAULT_STUDY = ROOT / "tp/results/sales/takeaway-sales.opx"
DEFAULT_OUTPUT = ROOT / "tp/report/analysis"
DEFAULT_EVIDENCE = ROOT / "tp/results/bpmn/evidence.json"
WEEKDAYS = ("Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday")
DIMENSIONS = {
    "channels": ("channel",),
    "weekdays": ("weekday",),
    "customers": ("customer_type",),
    "locations": ("customer_location",),
    "pizzas": ("pizza_type",),
    "pizza_sizes": ("pizza_size",),
    "rankings": ("pizza_type", "customer_type", "weekday"),
}
NATIVE_DIMENSIONS = {
    "Channel": "channel", "Weekday": "weekday", "Customer Type": "customer_type",
    "Customer Location": "customer_location", "Pizza Type": "pizza_type", "Pizza Size": "pizza_size",
}
METRIC_FIELDS = (
    "recorded_sales", "revenue_minor", "cost_minor", "profit_minor", "revenue", "cost", "profit",
    "average_revenue", "profit_rate", "satisfaction_average", "satisfaction_observed",
    "satisfaction_missing", "satisfaction_sum", "satisfaction_zero_count",
)
LIMITATIONS_FR = [
    "Une ligne de vente est une observation commerciale enregistrée. Elle ne prouve pas une transaction ou un client unique.",
    "Sales.Id est local à sa source et n'est jamais joint à un CaseId. StructureId peut référencer un scénario dans le même classeur, après vérification des feuilles complémentaires.",
    "Les ventes indiquent un jour de la semaine, sans date calendaire. Les courbes sont des profils agrégés, pas une chronologie datée.",
    "La monnaie n'est pas précisée : les montants sont exprimés dans l'unité monétaire enregistrée, sans symbole de devise.",
    "Le bénéfice calculé est Revenue − Cost dans les lignes fournies ; l'exhaustivité des charges comptables n'est pas documentée.",
    "L'échelle de satisfaction n'est pas documentée. Les moyennes utilisent les scores enregistrés, conservent zéro et excluent les valeurs absentes.",
    "La couverture des notes mesure leur présence dans les données ; elle ne garantit pas l'absence de biais de réponse.",
]


def decimal_text(value: Decimal | None) -> str | None:
    return None if value is None else format(value, "f")


def amount(minor: int) -> str:
    return format(Decimal(minor) / 100, ".2f")


def path_text(path: Path) -> str:
    try:
        return path.resolve().relative_to(ROOT).as_posix()
    except ValueError:
        return str(path.resolve())


def load_rows(path: Path) -> tuple[dict[str, Any], list[dict[str, Any]]]:
    raw = path.read_bytes()
    envelope = json.loads(raw, parse_float=Decimal)
    if envelope.get("format") != "open-process" or envelope.get("version") != 2:
        raise ValueError("Expected an Open Process v2 envelope")
    document = envelope.get("document", {})
    if document.get("kind") != "process.sales" or document.get("model_version") != 1:
        raise ValueError("Expected process.sales model version 1")
    model = document["model"]
    rows: list[dict[str, Any]] = []
    sources = []
    identities = set()
    for dataset in model["datasets"]:
        source, channel = dataset["source_name"], dataset["channel"]
        if not isinstance(source, str) or not source.strip() or not isinstance(channel, str) or not channel.strip():
            raise ValueError("Each dataset needs source and channel labels")
        if source in identities:
            raise ValueError(f"Repeated source identity: {source}")
        identities.add(source)
        for record in dataset["records"]:
            row = dict(record, source_name=source, channel=channel)
            for field in ("id", "structure_id", "pizza_type", "pizza_size", "weekday", "customer_location", "customer_type"):
                if not isinstance(row[field], str):
                    raise ValueError(f"Non-text {field} in {source}")
            for field in ("revenue", "cost"):
                value = row[field]
                if isinstance(value, bool) or not isinstance(value, int) or not -(2**63) <= value < 2**63:
                    raise ValueError(f"{source}: {field} is not integer minor units")
            score = row["satisfaction"]
            if score is not None:
                score = Decimal(str(score))
                if not score.is_finite():
                    raise ValueError(f"{source}: nonfinite satisfaction")
            row["satisfaction"] = score
            rows.append(row)
        sources.append({"source_name": source, "channel": channel, "recorded_sales": len(dataset["records"])})
    if not rows or len(rows) > 100_000:
        raise ValueError("Expected between 1 and 100,000 recorded sales")
    return {
        "path": path_text(path), "sha256": hashlib.sha256(raw).hexdigest(), "name": model["name"],
        "kind": document["kind"], "model_version": document["model_version"], "sources": sources,
    }, rows


def metrics(records: Iterable[dict[str, Any]]) -> dict[str, Any]:
    records = list(records)
    count = len(records)
    revenue = sum(row["revenue"] for row in records)
    cost = sum(row["cost"] for row in records)
    rated = [row["satisfaction"] for row in records if row["satisfaction"] is not None]
    score_sum = sum(rated, Decimal(0))
    with localcontext() as context:
        context.prec = 34
        return {
            "recorded_sales": count,
            "revenue_minor": revenue, "cost_minor": cost, "profit_minor": revenue - cost,
            "revenue": amount(revenue), "cost": amount(cost), "profit": amount(revenue - cost),
            "average_revenue": decimal_text(Decimal(revenue) / 100 / count) if count else None,
            "profit_rate": decimal_text(Decimal(revenue - cost) / revenue) if revenue else None,
            "satisfaction_average": decimal_text(score_sum / len(rated)) if rated else None,
            "satisfaction_observed": len(rated), "satisfaction_missing": count - len(rated),
            "satisfaction_sum": decimal_text(score_sum), "satisfaction_zero_count": sum(score == 0 for score in rated),
        }


def grouped(rows: list[dict[str, Any]], fields: tuple[str, ...]) -> list[dict[str, Any]]:
    groups: dict[tuple[str, ...], list[dict[str, Any]]] = defaultdict(list)
    for row in rows:
        groups[tuple(row[field] for field in fields)].append(row)
    result = [dict(zip(fields, key), **metrics(records)) for key, records in groups.items()]
    if fields == ("weekday",):
        result.sort(key=lambda row: (WEEKDAYS.index(row["weekday"]) if row["weekday"] in WEEKDAYS else 7, row["weekday"]))
    elif fields in (("pizza_type",), ("pizza_type", "customer_type", "weekday")):
        result.sort(key=lambda row: (-row["recorded_sales"], -row["revenue_minor"], tuple(row[field] for field in fields)))
    else:
        result.sort(key=lambda row: tuple(row[field] for field in fields))
    return result


def rank_pizzas(rows: list[dict[str, Any]]) -> dict[str, Any]:
    ranking = grouped(rows, ("pizza_type",))
    previous = None
    rank = 0
    for index, row in enumerate(ranking):
        if row["recorded_sales"] != previous:
            rank = index + 1
            previous = row["recorded_sales"]
        row["rank"] = rank
    return {"leaders": [row["pizza_type"] for row in ranking if row["rank"] == 1], "ranking": ranking}


def rankings(rows: list[dict[str, Any]]) -> dict[str, Any]:
    customer_types = sorted({row["customer_type"] for row in rows})
    days = WEEKDAYS + tuple(sorted({row["weekday"] for row in rows} - set(WEEKDAYS)))
    return {
        "method_fr": "Classement par nombre de lignes de vente décroissant ; les ex æquo partagent un rang. Le CA puis le libellé stabilisent leur affichage.",
        "global": rank_pizzas(rows),
        "by_customer_type": [dict(customer_type=kind, **rank_pizzas([row for row in rows if row["customer_type"] == kind])) for kind in customer_types],
        "by_weekday": [dict(weekday=day, **rank_pizzas([row for row in rows if row["weekday"] == day])) for day in days],
        "by_customer_type_weekday": [
            dict(customer_type=kind, weekday=day, **rank_pizzas([row for row in rows if row["customer_type"] == kind and row["weekday"] == day]))
            for kind in customer_types for day in days
        ],
    }


def write_table(path: Path, entries: list[dict[str, Any]], fields: tuple[str, ...]) -> None:
    with path.open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=fields + METRIC_FIELDS, delimiter=";", lineterminator="\n", extrasaction="ignore")
        writer.writeheader()
        writer.writerows(entries)


def approximately_equal(actual: str | None, expected: str | None) -> bool:
    if actual in (None, "") or expected in (None, ""):
        return actual in (None, "") and expected in (None, "")
    return abs(Decimal(actual) - Decimal(expected)) <= Decimal("1e-11")


def compare_native_metrics(native: dict[str, str], independent: dict[str, Any]) -> None:
    exact = {
        "Recorded sales": "recorded_sales", "Satisfaction observed": "satisfaction_observed", "Satisfaction missing": "satisfaction_missing",
    }
    for column, field in exact.items():
        assert int(native[column]) == independent[field], (column, native, independent)
    for column, field in (("Revenue", "revenue_minor"), ("Cost", "cost_minor"), ("Profit", "profit_minor")):
        assert Decimal(native[column]) * 100 == independent[field], (column, native, independent)
    optional = {
        "Average revenue per recorded sale": "average_revenue", "Profit rate (fraction)": "profit_rate", "Satisfaction average": "satisfaction_average",
    }
    for column, field in optional.items():
        if column in native:
            assert approximately_equal(native[column], independent[field]), (column, native, independent)


def validate_native(tables: dict[str, list[dict[str, Any]]], rows: list[dict[str, Any]], study: Path, kpis: list[dict[str, Any]]) -> dict[str, Any]:
    results = []

    def checked(path: Path, comparison: Any) -> None:
        try:
            details = comparison()
            results.append({"path": path_text(path), "status": "matched", "success": True, **details})
        except (AssertionError, KeyError, ValueError, InvalidOperation) as error:
            results.append({"path": path_text(path), "status": "mismatch", "success": False, "detail": str(error) or repr(error)})

    for name in ("channels", "weekdays", "customers", "locations", "pizzas", "rankings"):
        path = study.parent / f"{name}.csv"
        if not path.exists():
            if study.resolve() == DEFAULT_STUDY.resolve():
                results.append({"path": path_text(path), "status": "mismatch", "success": False, "detail": "Missing required native comparison table"})
            else:
                results.append({"path": path_text(path), "status": "not_available", "success": None})
            continue

        def compare_table() -> dict[str, int]:
            native = list(csv.DictReader(io.StringIO(path.read_text(encoding="utf-8-sig")), delimiter=";"))
            dimensions = DIMENSIONS[name]
            expected = {tuple(row[field] for field in dimensions): row for row in tables[name]}
            assert len(native) == len(expected), (name, len(native), len(expected))
            seen = set()
            for row in native:
                key = tuple(row[next(column for column, field in NATIVE_DIMENSIONS.items() if field == dimension)] for dimension in dimensions)
                assert key not in seen, (name, "duplicate group", key)
                seen.add(key)
                compare_native_metrics(row, expected[key])
            assert seen == set(expected)
            return {"groups": len(native)}

        checked(path, compare_table)
    series_path = next((study.parent / name for name in ("weekday-series.csv", "weekday_series.csv") if (study.parent / name).exists()), None)
    if series_path:

        def compare_series() -> dict[str, int]:
            native_series = list(csv.DictReader(io.StringIO(series_path.read_text(encoding="utf-8-sig")), delimiter=";"))
            types = sorted({row["customer_type"] for row in rows if row["weekday"] in WEEKDAYS})
            expected_keys = {("Total", "", day) for day in WEEKDAYS}
            expected_keys.update(("Customer type", kind, day) for kind in types for day in WEEKDAYS)
            expected_keys.update(("Unrecognized weekday", "", row["weekday"]) for row in rows if row["weekday"] not in WEEKDAYS)
            seen = set()
            for row in native_series:
                key = (row["Series"], row["Customer Type"], row["Weekday"])
                assert key not in seen, ("duplicate point", key)
                seen.add(key)
                subset = [record for record in rows if record["weekday"] == row["Weekday"] and (row["Series"] != "Customer type" or record["customer_type"] == row["Customer Type"])]
                compare_native_metrics(row, metrics(subset))
            assert seen == expected_keys, ("weekday series coverage", sorted(expected_keys - seen), sorted(seen - expected_keys))
            return {"points": len(native_series)}

        checked(series_path, compare_series)
    else:
        results.append({"path": path_text(study.parent / "weekday-series.csv"), "status": "not_available", "success": None})
    kpi_path = next((study.parent / name for name in ("kpis.csv", "kpi.csv") if (study.parent / name).exists()), None)
    if kpi_path:

        def compare_kpis() -> dict[str, int]:
            native_kpis = list(csv.DictReader(io.StringIO(kpi_path.read_text(encoding="utf-8-sig")), delimiter=";"))
            assert len(native_kpis) == len(kpis)
            expected_names = ("Recorded margin rate", "Satisfaction observation rate")
            for name, native, independent in zip(expected_names, native_kpis, kpis):
                assert native["KPI"] == name
                assert approximately_equal(native["Value (fraction)"], independent["value_fraction"])
                assert Decimal(native["Numerator"]) == Decimal(independent["numerator"])
                assert Decimal(native["Denominator"]) == Decimal(independent["denominator"])
            return {"indicators": len(native_kpis)}

        checked(kpi_path, compare_kpis)
    else:
        results.append({"path": path_text(study.parent / "kpis.csv"), "status": "not_available", "success": None})
    mismatches = [row for row in results if row["success"] is False]
    return {
        "success": not mismatches, "mismatches": mismatches, "checked_native_csv_count": sum(row["success"] is True for row in results),
        "method_fr": "Recalcul Python indépendant en entiers et Decimal, puis comparaison aux exports natifs ; tolérance 10⁻¹¹ pour les moyennes et ratios flottants natifs.",
        "native_csvs": results,
    }


def make_kpis(totals: dict[str, Any]) -> list[dict[str, Any]]:
    with localcontext() as context:
        context.prec = 34
        coverage = Decimal(totals["satisfaction_observed"]) / totals["recorded_sales"]
        margin = Decimal(totals["profit_rate"]) if totals["profit_rate"] is not None else None
        return [
            {
                "id": "recorded_margin_rate", "label_fr": "Part du bénéfice dans le CA enregistré",
                "formula_fr": "Bénéfice enregistré ÷ chiffre d'affaires enregistré",
                "value_fraction": decimal_text(margin), "value_percent": decimal_text(margin * 100) if margin is not None else None,
                "numerator": totals["profit"], "denominator": totals["revenue"],
                "interpretation_fr": "Part du chiffre d'affaires qui reste après les coûts enregistrés ; indéfinie si le CA est nul.",
                "action_fr": "Comparer canaux et pizzas pour identifier les ventes peu rentables avant d'ajuster prix, assortiment ou coûts.",
            },
            {
                "id": "satisfaction_observation_rate", "label_fr": "Taux de satisfaction renseignée",
                "formula_fr": "Ventes avec une note ÷ lignes de vente enregistrées",
                "value_fraction": decimal_text(coverage), "value_percent": decimal_text(coverage * 100),
                "numerator": str(totals["satisfaction_observed"]), "denominator": str(totals["recorded_sales"]),
                "interpretation_fr": "Couverture des moyennes de satisfaction, en conservant les notes zéro et en excluant seulement les notes absentes.",
                "action_fr": "Distinguer les notes éventuellement non applicables aux scénarios d'annulation des véritables absences de retour avant d'agir sur la collecte ; cette couverture ne prouve pas la représentativité des réponses.",
            },
        ]


def workbook_fingerprint(raw: bytes) -> str:
    value = 0xCBF29CE484222325
    for byte in raw:
        value = ((value ^ byte) * 0x100000001B3) & ((1 << 64) - 1)
    return f"fnv1a64:{value:016x}"


def normalized_header(value: str) -> str:
    return value.strip().lower().replace(" ", "").replace("_", "").replace("-", "")


def worksheet_with_columns(worksheets: list[dict[str, Any]], required: set[str]) -> tuple[dict[str, Any], dict[str, str], int]:
    candidates = []
    for worksheet in worksheets:
        for row in worksheet["rows"]:
            header = {normalized_header(value): column for column, value in row["cells"].items()}
            if required <= set(header):
                candidates.append((worksheet, header, row["row"]))
                break
    assert len(candidates) == 1, ("ambiguous complementary worksheet", required, [candidate[0]["name"] for candidate in candidates])
    return candidates[0]


def scenario_diagnostics(rows: list[dict[str, Any]], study: Path, evidence_path: Path) -> dict[str, Any]:
    evidence_bytes = evidence_path.read_bytes()
    evidence = json.loads(evidence_bytes)
    templates = []
    workbooks = []
    cancellation_rows = []
    matched = set()
    field_headers = {
        "id": "id", "structure_id": "structureid", "pizza_type": "pizzatype", "pizza_size": "pizzasize",
        "weekday": "weekday", "customer_location": "customerlocation", "customer_type": "customertype",
    }
    for source in evidence["inputs"]:
        workbook_path = ROOT / source["complementary_workbook"]
        workbook_bytes = workbook_path.read_bytes()
        assert workbook_fingerprint(workbook_bytes) == source["workbook_fingerprint"], workbook_path
        source_name = workbook_path.name
        channel = source["channel"]
        local_rows = [row for row in rows if row["source_name"] == source_name and row["channel"] == channel]
        if not local_rows:
            continue
        matched.add((source_name, channel))
        worksheets = source["all_workbook_worksheets_cached_values"]
        structure_sheet, structure_header, structure_start = worksheet_with_columns(worksheets, {"structureid", "activity"})
        sales_sheet, sales_header, sales_start = worksheet_with_columns(worksheets, {"id", "structureid", "revenue", "cost", "satisfaction", "pizzatype"})
        structures: dict[str, list[str]] = defaultdict(list)
        structure_lines: dict[str, list[int]] = defaultdict(list)
        for worksheet_row in structure_sheet["rows"]:
            if worksheet_row["row"] <= structure_start:
                continue
            cells = worksheet_row["cells"]
            identifier = cells.get(structure_header["structureid"])
            activity = cells.get(structure_header["activity"])
            if identifier is not None and activity is not None:
                structures[identifier].append(activity)
                structure_lines[identifier].append(worksheet_row["row"])
        for audited in source["case_structures"]:
            assert structures[audited["structure_id"]] == audited["raw_worksheet_trace"], (source_name, audited["structure_id"])
        assert all(row["structure_id"] in structures for row in local_rows), source_name
        cancellation_ids = sorted(identifier for identifier, trace in structures.items() if trace and trace[-1].strip().casefold() == "cancel order")
        worksheet_sales: dict[tuple[str, str], list[dict[str, Any]]] = defaultdict(list)
        for worksheet_row in sales_sheet["rows"]:
            if worksheet_row["row"] > sales_start:
                cells = worksheet_row["cells"]
                if sales_header["id"] in cells and sales_header["structureid"] in cells:
                    worksheet_sales[(cells[sales_header["id"]], cells[sales_header["structureid"]])].append(worksheet_row)
        for identifier in cancellation_ids:
            templates.append({
                "source_name": source_name, "channel": channel, "structure_id": identifier,
                "case_structure_worksheet": structure_sheet["name"], "worksheet_rows": structure_lines[identifier],
                "raw_template_trace": structures[identifier],
            })
        for row in local_rows:
            candidates = worksheet_sales[(row["id"], row["structure_id"])]
            assert len(candidates) == 1, (source_name, "local Sales.Id/StructureId lookup", row["id"], row["structure_id"])
            worksheet_row = candidates[0]
            cells = worksheet_row["cells"]
            for field, header in field_headers.items():
                assert cells.get(sales_header[header], "") == row[field], (source_name, row["id"], field)
            for field in ("revenue", "cost"):
                assert Decimal(cells[sales_header[field]]) * 100 == row[field], (source_name, row["id"], field)
            cached_score = cells.get(sales_header["satisfaction"])
            score = Decimal(cached_score) if cached_score not in (None, "") else None
            assert score == row["satisfaction"], (source_name, row["id"], "satisfaction")
            if row["structure_id"] in cancellation_ids:
                cancellation_rows.append(dict(row, worksheet_row=worksheet_row["row"], sales_worksheet=sales_sheet["name"]))
        local_cancel = [row for row in local_rows if row["structure_id"] in cancellation_ids]
        local_metrics = metrics(local_cancel)
        native_audit = source.get("cancellation_template_diagnostic")
        if native_audit:
            assert native_audit["source_name"] == source_name
            assert native_audit["case_structure_worksheet"] == structure_sheet["name"]
            assert native_audit["sales_worksheet"] == sales_sheet["name"]
            assert native_audit["all_sale_structure_ids_resolve"]
            assert set(native_audit["cancellation_structure_ids"]) == set(cancellation_ids)
            assert set(native_audit["recorded_sale_ids"]) == {row["id"] for row in local_cancel}
            assert native_audit["record_count"] == local_metrics["recorded_sales"]
            for native_field, metric_field in (
                ("recorded_revenue_minor_units", "revenue_minor"), ("recorded_cost_minor_units", "cost_minor"),
                ("recorded_profit_minor_units", "profit_minor"), ("missing_satisfaction_records", "satisfaction_missing"),
            ):
                assert native_audit[native_field] == local_metrics[metric_field], (source_name, native_field)
        workbooks.append({
            "source_name": source_name, "channel": channel, "path": path_text(workbook_path),
            "sha256": hashlib.sha256(workbook_bytes).hexdigest(), "native_fingerprint_verified": True,
            "case_structure_worksheet": structure_sheet["name"], "sales_worksheet": sales_sheet["name"],
            "template_count": len(structures), "all_sale_structure_ids_resolve": True, "cancellation_structure_ids": cancellation_ids,
            "recorded_sales": len(local_rows), "native_cancellation_diagnostic_compared": native_audit is not None,
        })
    assert matched == {(row["source_name"], row["channel"]) for row in rows}, "Every dataset needs its own workbook evidence"
    cancellation_keys = {(row["source_name"], row["channel"], row["id"], row["structure_id"]) for row in cancellation_rows}
    retained = [row for row in rows if (row["source_name"], row["channel"], row["id"], row["structure_id"]) not in cancellation_keys]
    cancellation_metrics = metrics(cancellation_rows)
    retained_metrics = metrics(retained)
    diagnostics_rows = [
        {
            "source_name": row["source_name"], "channel": row["channel"], "sale_id": row["id"], "structure_id": row["structure_id"],
            "sales_worksheet": row["sales_worksheet"], "worksheet_row": row["worksheet_row"],
            "pizza_type": row["pizza_type"], "pizza_size": row["pizza_size"], "weekday": row["weekday"],
            "customer_type": row["customer_type"], "customer_location": row["customer_location"],
            "revenue_minor": row["revenue"], "cost_minor": row["cost"], "profit_minor": row["revenue"] - row["cost"],
            "revenue": amount(row["revenue"]), "cost": amount(row["cost"]), "profit": amount(row["revenue"] - row["cost"]),
            "satisfaction": decimal_text(row["satisfaction"]),
        }
        for row in cancellation_rows
    ]
    subset_path = study.parent / "internet-structure-7.csv"
    subset_validation: dict[str, Any] = {"path": path_text(subset_path), "status": "not_available", "success": None}
    if subset_path.exists():
        native = list(csv.DictReader(io.StringIO(subset_path.read_text(encoding="utf-8-sig")), delimiter=";"))
        try:
            assert len(native) == 1 and native[0]["Channel"] == "Internet"
            expected_subset = metrics([row for row in rows if row["channel"] == "Internet" and row["structure_id"] == "7"])
            compare_native_metrics(native[0], expected_subset)
            assert expected_subset == cancellation_metrics, "The native diagnostic filter must select the audited cancellation templates"
            subset_validation.update(status="matched", success=True, groups=1, recorded_sales=expected_subset["recorded_sales"])
        except (AssertionError, KeyError, ValueError, InvalidOperation) as error:
            subset_validation.update(status="mismatch", success=False, detail=str(error) or repr(error))
    all_missing = sum(row["satisfaction"] is None for row in rows)
    cancellation_missing = cancellation_metrics["satisfaction_missing"]
    if study.resolve() == DEFAULT_STUDY.resolve():
        assert sorted(row["sale_id"] for row in diagnostics_rows) == ["135", "18", "61"]
        assert (cancellation_metrics["recorded_sales"], cancellation_metrics["revenue_minor"], cancellation_metrics["cost_minor"], cancellation_metrics["profit_minor"]) == (3, 3600, 2800, 800)
        assert (retained_metrics["recorded_sales"], retained_metrics["revenue_minor"], retained_metrics["cost_minor"], retained_metrics["profit_minor"]) == (460, 675900, 539200, 136700)
    return {
        "success": True, "evidence_path": path_text(evidence_path), "evidence_sha256": hashlib.sha256(evidence_bytes).hexdigest(),
        "link_rule_fr": "Sales.StructureId référence CasesStructure/CaseStructure dans le même classeur complémentaire uniquement. Les Sales.Id servent à identifier les lignes de cette feuille, jamais à joindre un CaseId.",
        "workbooks": workbooks, "templates": templates, "cancel_template_rows": diagnostics_rows,
        "cancel_template_totals": cancellation_metrics,
        "conditional_exclusion": {
            "label_fr": "Sensibilité conditionnelle : exclusion des lignes associées à un scénario d'annulation",
            "assumption_fr": "Uniquement si l'on suppose que les montants des lignes rattachées à un scénario se terminant par Cancel order ne doivent pas être comptabilisés, ces trois lignes sont exclues.",
            "not_asserted_fr": "Ce calcul n'établit pas un chiffre d'affaires encaissé : la reconnaissance du revenu, l'encaissement et les règles d'annulation ou de remboursement ne sont pas documentés.",
            "totals": retained_metrics,
        },
        "rating_alignment": {
            "missing_all_records": all_missing, "missing_cancel_template_records": cancellation_missing,
            "missing_other_records": all_missing - cancellation_missing, "exact_alignment": all_missing == cancellation_missing,
            "interpretation_fr": "Les trois notes absentes coïncident avec les lignes rattachées au scénario d'annulation. Elles peuvent être non applicables ; les données ne prouvent pas un défaut de collecte.",
        },
        "native_subset_validation": subset_validation,
    }


def figure_catalog() -> list[dict[str, str]]:
    descriptions = [
        ("01_weekday_evolution", "Profils des ventes par jour de la semaine", "Nombre de lignes de vente par jour déclaré, puis par type de clientèle enregistré. Les sept jours sont des catégories agrégées : aucune date, transaction ou identité client unique n'est reconstituée."),
        ("02_channel_financials", "Résultats financiers par canal", "Chiffre d'affaires, coûts et bénéfice calculés en centièmes entiers de l'unité enregistrée. Aucun symbole de devise n'est supposé."),
        ("03_weekday_sales", "Volume et chiffre d'affaires par jour déclaré", "Comparaison des lignes de vente et du chiffre d'affaires pour les sept jours de la semaine ; il ne s'agit pas d'une série quotidienne datée."),
        ("04_geography", "Répartition géographique des ventes enregistrées", "Répartition des lignes selon le district déclaré. Les libellés sources sont conservés ; aucune position cartographique ou population locale n'est inférée."),
        ("05_satisfaction", "Satisfaction par clientèle, pizza et district", "Moyennes des scores présents uniquement. Les notes zéro restent des observations, les notes absentes sont exclues et les effectifs notés sont indiqués. L'échelle de notation n'est pas supposée."),
        ("06_pizza_rankings", "Palmarès des pizzas et bénéfice enregistré", "Classement par nombre de lignes de vente, accompagné du bénéfice par pizza. Une ligne n'est assimilée ni à un client unique ni à une quantité physique non documentée."),
        ("07_customer_profiles", "Types de clientèle enregistrés et canaux", "Comparaison des occurrences de chaque type de clientèle, globalement et par canal. Ces effectifs ne dénombrent pas des personnes distinctes."),
    ]
    return [{"id": ident, "title_fr": title, "caption_fr": caption, "svg": f"figures/{ident}.svg", "png": f"figures/{ident}.png"} for ident, title, caption in descriptions]


def render_figures(output: Path, rows: list[dict[str, Any]], tables: dict[str, list[dict[str, Any]]], catalog: list[dict[str, str]]) -> None:
    if "MPLCONFIGDIR" not in os.environ:
        config = ROOT / "target/takeaway-matplotlib"
        config.mkdir(parents=True, exist_ok=True)
        os.environ["MPLCONFIGDIR"] = str(config)
    import matplotlib
    matplotlib.use("Agg")
    import matplotlib.pyplot as plt
    from matplotlib.ticker import FuncFormatter, MaxNLocator

    plt.rcParams.update({
        "font.family": "DejaVu Sans", "font.size": 10, "axes.titlesize": 12, "axes.labelsize": 10,
        "axes.spines.top": False, "axes.spines.right": False, "figure.facecolor": "white", "axes.facecolor": "white",
        "savefig.facecolor": "white", "svg.hashsalt": "open-process-takeaway-analysis-v1",
    })
    colors = ["#126c63", "#3579b3", "#b96d24", "#8057a4", "#b2466a", "#4b8762"]
    (output / "figures").mkdir(parents=True, exist_ok=True)
    weekdays = {row["weekday"]: row for row in tables["weekdays"]}
    weekday_rows = [weekdays.get(day, metrics([])) for day in WEEKDAYS]
    customer_types = sorted({row["customer_type"] for row in rows})
    channels = tables["channels"]

    def polish(ax: Any, integer: bool = False) -> None:
        ax.grid(axis="y", color="#dfe5e9", linewidth=0.7)
        ax.set_axisbelow(True)
        if integer:
            ax.yaxis.set_major_locator(MaxNLocator(integer=True))

    def save(fig: Any, item: dict[str, str]) -> None:
        svg_path = output / item["svg"]
        fig.savefig(svg_path, bbox_inches="tight", metadata={"Date": None, "Creator": "Open Process TakeAway analysis"})
        # Normalize Matplotlib path formatting for clean repository diffs.
        svg_path.write_text("\n".join(line.rstrip() for line in svg_path.read_text(encoding="utf-8").splitlines()) + "\n", encoding="utf-8")
        fig.savefig(output / item["png"], bbox_inches="tight", dpi=220, metadata={"Software": "Open Process TakeAway analysis"})
        plt.close(fig)

    fig, axes = plt.subplots(2, 1, figsize=(9, 7), layout="constrained", sharex=True)
    axes[0].plot(WEEKDAYS, [row["recorded_sales"] for row in weekday_rows], color=colors[0], marker="o", linewidth=2.3)
    axes[0].set(title="Toutes les lignes de vente enregistrées", ylabel="Lignes de vente")
    for index, kind in enumerate(customer_types):
        counts = [sum(row["customer_type"] == kind and row["weekday"] == day for row in rows) for day in WEEKDAYS]
        axes[1].plot(WEEKDAYS, counts, label=kind or "Type non renseigné", color=colors[index % len(colors)], marker="o", linewidth=1.8)
    axes[1].set(title="Comparaison par type de clientèle enregistré", xlabel="Jour de la semaine déclaré", ylabel="Lignes de vente")
    axes[1].legend(ncols=2, frameon=False)
    for ax in axes:
        polish(ax, True)
        ax.set_ylim(bottom=0)
    fig.suptitle(catalog[0]["title_fr"], fontsize=14, fontweight="bold")
    save(fig, catalog[0])

    fig, ax = plt.subplots(figsize=(8, 4.8), layout="constrained")
    x = list(range(len(channels)))
    for offset, field, label, color in [(-0.25, "revenue", "Chiffre d'affaires", colors[1]), (0, "cost", "Coûts", "#9ba7b1"), (0.25, "profit", "Bénéfice", colors[0])]:
        bars = ax.bar([value + offset for value in x], [float(row[field]) for row in channels], width=0.24, label=label, color=color)
        ax.bar_label(bars, fmt="%.0f", fontsize=8, padding=3)
    ax.set(xticks=x, xticklabels=[row["channel"] for row in channels], ylabel="Unité monétaire enregistrée", title=catalog[1]["title_fr"])
    ax.legend(frameon=False)
    ax.margins(y=0.17)
    polish(ax)
    save(fig, catalog[1])

    fig, axes = plt.subplots(2, 1, figsize=(9, 6.4), layout="constrained", sharex=True)
    bars = axes[0].bar(WEEKDAYS, [row["recorded_sales"] for row in weekday_rows], color=colors[1])
    axes[0].bar_label(bars, padding=3)
    axes[0].set(ylabel="Lignes de vente", title="Volume enregistré")
    bars = axes[1].bar(WEEKDAYS, [float(row["revenue"]) for row in weekday_rows], color=colors[0])
    axes[1].bar_label(bars, fmt="%.0f", padding=3)
    axes[1].set(ylabel="Unité monétaire enregistrée", xlabel="Jour de la semaine déclaré", title="Chiffre d'affaires enregistré")
    for ax in axes:
        polish(ax)
        ax.margins(y=0.15)
    fig.suptitle(catalog[2]["title_fr"], fontsize=14, fontweight="bold")
    save(fig, catalog[2])

    geography = sorted(tables["locations"], key=lambda row: row["recorded_sales"])
    fig, ax = plt.subplots(figsize=(9, 4.8), layout="constrained")
    bars = ax.barh([row["customer_location"] for row in geography], [row["recorded_sales"] for row in geography], color=colors[0])
    ax.bar_label(bars, padding=4)
    ax.set(xlabel="Lignes de vente enregistrées", title=catalog[3]["title_fr"])
    ax.xaxis.set_major_locator(MaxNLocator(integer=True))
    ax.margins(x=0.15)
    save(fig, catalog[3])

    fig, axes = plt.subplots(3, 1, figsize=(8.5, 11.0), layout="constrained")
    for ax, table, field, title in zip(axes, (tables["customers"], tables["pizzas"], tables["locations"]), ("customer_type", "pizza_type", "customer_location"), ("Type de clientèle", "Pizza", "District déclaré")):
        rated = sorted(table, key=lambda row: (Decimal(row["satisfaction_average"] or "0"), row[field]))
        values = [float(row["satisfaction_average"]) if row["satisfaction_average"] is not None else math.nan for row in rated]
        labels = [f"{row[field]}  ({row['satisfaction_observed']}/{row['recorded_sales']})" for row in rated]
        bars = ax.barh(labels, values, color=colors[1])
        ax.bar_label(bars, fmt=lambda value: f"{value:.2f}".replace(".", ","), padding=4, fontsize=9)
        ax.set(title=title, xlabel="Score moyen observé")
        ax.xaxis.set_major_formatter(FuncFormatter(lambda value, _: format(value, "g").replace(".", ",")))
        finite = [value for value in values if math.isfinite(value)]
        low, high = min([0, *finite]), max([0, *finite])
        padding = (high - low) * 0.2 or 1
        ax.set_xlim(left=low - padding if low < 0 else 0, right=high + padding)
    fig.suptitle(catalog[4]["title_fr"] + "\nEffectifs notés / enregistrés", fontsize=14, fontweight="bold")
    save(fig, catalog[4])

    pizzas = list(reversed(tables["pizzas"]))
    fig, axes = plt.subplots(1, 2, figsize=(8.8, 4.8), layout="constrained", sharey=True)
    labels = [row["pizza_type"] for row in pizzas]
    bars = axes[0].barh(labels, [row["recorded_sales"] for row in pizzas], color=colors[1])
    axes[0].bar_label(bars, padding=4)
    axes[0].set(xlabel="Lignes de vente enregistrées", title="Classement en volume enregistré")
    axes[0].xaxis.set_major_locator(MaxNLocator(integer=True))
    bars = axes[1].barh(labels, [float(row["profit"]) for row in pizzas], color=colors[0])
    axes[1].bar_label(bars, fmt="%.0f", padding=4)
    axes[1].set(xlabel="Unité monétaire enregistrée", title="Bénéfice associé")
    for ax in axes:
        ax.margins(x=0.18)
    fig.suptitle(catalog[5]["title_fr"], fontsize=14, fontweight="bold")
    save(fig, catalog[5])

    fig, axes = plt.subplots(1, 2, figsize=(9, 4.8), layout="constrained")
    customer_table = tables["customers"]
    bars = axes[0].bar([row["customer_type"] for row in customer_table], [row["recorded_sales"] for row in customer_table], color=colors[0])
    axes[0].bar_label(bars, padding=3)
    axes[0].set(title="Occurrences enregistrées par type", ylabel="Lignes de vente")
    positions = list(range(len(customer_types)))
    width = 0.75 / max(len(channels), 1)
    for index, channel in enumerate(channels):
        values = [sum(row["channel"] == channel["channel"] and row["customer_type"] == kind for row in rows) for kind in customer_types]
        axes[1].bar([value + (index - (len(channels) - 1) / 2) * width for value in positions], values, width=width, label=channel["channel"], color=colors[index % len(colors)])
    axes[1].set(xticks=positions, xticklabels=customer_types, title="Comparaison des canaux", ylabel="Lignes de vente")
    axes[1].legend(frameon=False)
    for ax in axes:
        polish(ax, True)
        ax.margins(y=0.15)
    fig.suptitle(catalog[6]["title_fr"], fontsize=14, fontweight="bold")
    save(fig, catalog[6])


def build(study_path: Path, output: Path, evidence_path: Path = DEFAULT_EVIDENCE) -> dict[str, Any]:
    source, rows = load_rows(study_path)
    totals = metrics(rows)
    tables = {name: grouped(rows, dimensions) for name, dimensions in DIMENSIONS.items()}
    customer_types = sorted({row["customer_type"] for row in rows})
    days = WEEKDAYS + tuple(sorted({row["weekday"] for row in rows} - set(WEEKDAYS)))
    tables["weekday_customer_types"] = [
        dict(weekday=day, customer_type=kind, **metrics([row for row in rows if row["weekday"] == day and row["customer_type"] == kind]))
        for day in days for kind in customer_types
    ]
    for name, entries in tables.items():
        assert sum(row["recorded_sales"] for row in entries) == totals["recorded_sales"], name
        for field in ("revenue_minor", "cost_minor", "profit_minor", "satisfaction_observed", "satisfaction_missing"):
            assert sum(row[field] for row in entries) == totals[field], (name, field)
    if study_path.resolve() == DEFAULT_STUDY.resolve():
        assert (totals["recorded_sales"], totals["revenue_minor"], totals["cost_minor"], totals["profit_minor"]) == (463, 679500, 542000, 137500)
        assert (totals["satisfaction_observed"], totals["satisfaction_missing"], Decimal(totals["satisfaction_sum"])) == (460, 3, Decimal(1258))
    kpis = make_kpis(totals)
    validation = validate_native(tables, rows, study_path, kpis)
    scenarios = scenario_diagnostics(rows, study_path, evidence_path)
    validation["scenario_subset"] = scenarios["native_subset_validation"]
    validation["scenario_diagnostics_success"] = scenarios["success"]
    if scenarios["native_subset_validation"]["success"] is False:
        validation["success"] = False
        validation["mismatches"].append(scenarios["native_subset_validation"])
    output.mkdir(parents=True, exist_ok=True)
    (output / "tables").mkdir(exist_ok=True)
    csv_files = {}
    for name, entries in tables.items():
        dimensions = DIMENSIONS.get(name, ("weekday", "customer_type"))
        relative = f"tables/{name}.csv"
        write_table(output / relative, entries, dimensions)
        csv_files[name] = relative
    ranked = rankings(rows)
    relative = "tables/rankings_global.csv"
    write_table(output / relative, ranked["global"]["ranking"], ("rank", "pizza_type"))
    csv_files["rankings_global"] = relative
    for key, fields in (("by_customer_type", ("customer_type",)), ("by_weekday", ("weekday",)), ("by_customer_type_weekday", ("customer_type", "weekday"))):
        entries = [dict({field: group[field] for field in fields}, **item) for group in ranked[key] for item in group["ranking"]]
        relative = f"tables/rankings_{key}.csv"
        write_table(output / relative, entries, fields + ("rank", "pizza_type"))
        csv_files[f"rankings_{key}"] = relative
    with (output / "tables/kpis.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.DictWriter(stream, fieldnames=list(kpis[0]), delimiter=";", lineterminator="\n")
        writer.writeheader()
        writer.writerows(kpis)
    csv_files["kpis"] = "tables/kpis.csv"
    with (output / "tables/weekday_series.csv").open("w", encoding="utf-8", newline="") as stream:
        writer = csv.writer(stream, delimiter=";", lineterminator="\n")
        writer.writerow(["Series", "Customer Type", "Weekday", "Recorded sales", "Revenue", "Cost", "Profit", "Satisfaction average", "Satisfaction observed", "Satisfaction missing"])

        def write_series_row(series: str, kind: str, day: str, records: list[dict[str, Any]]) -> None:
            values = metrics(records)
            writer.writerow([series, kind, day, values["recorded_sales"], values["revenue"], values["cost"], values["profit"], values["satisfaction_average"], values["satisfaction_observed"], values["satisfaction_missing"]])

        for day in WEEKDAYS:
            write_series_row("Total", "", day, [row for row in rows if row["weekday"] == day])
        for kind in customer_types:
            for day in WEEKDAYS:
                write_series_row("Customer type", kind, day, [row for row in rows if row["customer_type"] == kind and row["weekday"] == day])
        for day in days[7:]:
            write_series_row("Unrecognized weekday", "", day, [row for row in rows if row["weekday"] == day])
    csv_files["weekday_series"] = "tables/weekday_series.csv"
    with (output / "tables/scenario_diagnostics.csv").open("w", encoding="utf-8", newline="") as stream:
        fields = ["source_name", "channel", "sale_id", "structure_id", "sales_worksheet", "worksheet_row", "pizza_type", "pizza_size", "weekday", "customer_type", "customer_location", "revenue_minor", "cost_minor", "profit_minor", "revenue", "cost", "profit", "satisfaction"]
        writer = csv.DictWriter(stream, fieldnames=fields, delimiter=";", lineterminator="\n")
        writer.writeheader()
        writer.writerows(scenarios["cancel_template_rows"])
    csv_files["scenario_diagnostics"] = "tables/scenario_diagnostics.csv"
    sensitivity = [dict(scenario="all_recorded_rows", **totals), dict(scenario="cancellation_template_rows", **scenarios["cancel_template_totals"]), dict(scenario="conditional_exclusion", **scenarios["conditional_exclusion"]["totals"])]
    write_table(output / "tables/scenario_sensitivity.csv", sensitivity, ("scenario",))
    csv_files["scenario_sensitivity"] = "tables/scenario_sensitivity.csv"
    catalog = figure_catalog()
    render_figures(output, rows, tables, catalog)
    summary = {
        "schema_version": 1, "generated_by": "scripts/takeaway_analysis.py", "study": source, "totals": totals,
        "tables": tables, "rankings": ranked, "kpis": kpis, "figures": catalog, "csv_files": csv_files,
        "limitations_fr": LIMITATIONS_FR, "validation": validation,
        "scenario_diagnostics": scenarios,
    }
    (output / "summary.json").write_text(json.dumps(summary, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
    return summary


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--study", type=Path, default=DEFAULT_STUDY)
    parser.add_argument("--output", type=Path, default=DEFAULT_OUTPUT)
    parser.add_argument("--evidence", type=Path, default=DEFAULT_EVIDENCE)
    args = parser.parse_args()
    summary = build(args.study.resolve(), args.output.resolve(), args.evidence.resolve())
    totals = summary["totals"]
    print(f"{totals['recorded_sales']} lignes ; CA {totals['revenue']} ; coûts {totals['cost']} ; bénéfice {totals['profit']} ; {totals['satisfaction_observed']} notes et {totals['satisfaction_missing']} absences.")
    print(f"Résumé : {args.output / 'summary.json'} ; {len(summary['figures'])} figures SVG/PNG.")
    if not summary["validation"]["success"]:
        raise SystemExit("Échec de comparaison aux exports natifs ; détails dans summary.json.")


if __name__ == "__main__":
    main()
