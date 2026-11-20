# Reproducing the TakeAway deliverables

Run these commands from the Open Process repository root. The submission archive
contains portable reports, models and data; rebuilding the native algorithms
requires the repository checkout containing this increment. No ProM or BizAgi
installation is used.

## Native models and exports

```sh
cargo run --locked -p op-core --example takeaway_tp
cargo run --locked -p op-core --example takeaway_tp -- --check
```

The example uses the production event/sales importers, Alpha discovery,
Petri/BPMN conversions, typed native codecs, BPMN XML and shared SVG renderer.
It reads `tp/clean/*.csv` as the behavioral reference and the workbooks' cached
values for complementary evidence. Custom Excel functions are not executed.
The example leaves supplied inputs and existing log/Petri/study files unchanged.
It generates four OPX/BPMN/SVG models, evidence, validation, weekday series,
KPI CSVs and the Internet scenario-7 subset. `--check` repeats the native checks
and compares all 17 outputs byte-for-byte.

## Independent analysis and publication

The optional Python tooling is separate from native/WebAssembly application
dependencies. It was tested with Python 3.14 on Linux. Install the pinned report
packages in a virtual environment; CairoSVG also requires a working Cairo
library. Matplotlib supplies a portable DejaVu font fallback.

For example, with `uv`:

```sh
uv venv /tmp/open-process-takeaway-report
uv pip install --python /tmp/open-process-takeaway-report/bin/python -r scripts/takeaway-report-requirements.txt
/tmp/open-process-takeaway-report/bin/python scripts/takeaway_analysis.py
/tmp/open-process-takeaway-report/bin/python scripts/takeaway_report.py --date 2026-11-20
```

An ordinary Python virtual environment and `pip install -r` are also suitable
when the interpreter includes venv/ensurepip. No system Python package changes
are required.

The analysis reads raw rows in the native Sales study and computes money in
integer hundredths and averages with `Decimal`. It compares eight main native
CSVs and the separate native scenario subset, creates seven SVG/PNG figure pairs
and sixteen CSV tables, and records source SHA-256 fingerprints in
`report/analysis/summary.json`. The local `StructureId`/workbook template reference
identifies cancellation-scenario rows without joining any sales ID to a case ID.
Canonical record totals stay unchanged; a separate conditional financial
sensitivity is published rather than silently excluding those rows.

Final publication requires a passing `results/verification.json`; it does not
invent an application-check result. `--draft --output target/tp-report-preview`
creates a clearly marked preview while checks run. Final PDF/HTML, a manifest,
the ZIP archive and its SHA-256 checksum are generated together. ZIP entries use
fixed timestamps and preserve TP-relative paths, including HTML figure links.

## Verification scope

The checked commands and outcomes are recorded in
[verification.json](results/verification.json). Application tests are sequential:

```sh
cargo test --locked --workspace -- --test-threads=1
cargo build --locked --workspace --all-targets
cargo check --locked -p op-app
cargo check --locked -p op-app --target wasm32-unknown-unknown
cargo fmt --all -- --check
git diff --check
```

A Trunk debug build under `/app/` and the headless Firefox worker smoke check
exercise the actual WebAssembly job transport, including weekday/customer-type
series. Report and model checks are local evidence, not a remote multi-system CI
or release certification. The report's limitations also apply to its figures:
recorded rows are not proven unique people or joined transactions; weekdays do
not establish calendar chronology; source money has no specified currency.
