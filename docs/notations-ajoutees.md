# DMN, EPC and UML activities: initial integration

**Implemented: three document types in the home screen and creation picker, with
examples, `.opx` saves, history and session restoration.** These initial versions
have the limits below and do not establish Looping/Bizagi parity.

**File → New…** and the **+** button open the shared type grid. The exports below
are available through **File → Export**.

## DMN decision tables

- Document ID: `dmn.table`, alias `dmn`.
- Named, typed input/output columns: `string`, `number`, `boolean`.
- Add/remove columns and rules, with per-rule annotations.
- `UNIQUE`, `FIRST`, `ANY` and non-aggregating `COLLECT` hit policies.
- Evaluation against JSON inputs, showing matching rules and results.
- Supported unary tests: `-`, literals, numeric comparisons, intervals,
  comma-separated alternatives and `not(...)`.
- Outputs limited to string, number, boolean and `null` literals.
- DMN XML and CSV export; single-table XML import within the representable subset.
  Multiple decisions, dependency graphs, aggregations and unsupported structural
  constructs are rejected.
- Invalid names/expressions can remain in editable/saved drafts, but diagnostics
  block evaluation. Cell dimensions must match the columns to open a document.

This is **not a full FEEL interpreter**. Functions, contexts, dates, arithmetic,
DRD, BKM and decision services remain future work. Limits are 128 inputs, 128
outputs and 10,000 rules; test expressions are capped at 4,096 bytes and 32 nesting
levels. Numeric comparisons use `f64`, without full FEEL decimal precision.

The editor is a scrollable table without artificial canvas zoom. Its inspector
and actions reuse common layout components. The reference is
[OMG DMN 1.5](https://www.omg.org/spec/DMN/1.5/About-DMN); full conformance is not claimed.

## EPC

- ID: `epc`.
- Events, functions and AND/OR/XOR connectors.
- Creation, single/multiple/rubber-band selection, movement, resizing,
  `Ctrl/Cmd+D` duplication, deletion and inspector-based name editing.
- Two-click or drag connections, flow labels and input/output-degree diagnostics.
  Direct event → event and function → function links are rejected.
- SVG, Graphviz DOT and canvas PNG export.

Validation is structural. Transitive event/function alternation through connectors,
choice rules after events, organizational/eEPC extensions and simulation are not
implemented. EPML import/export is not available.

## UML activities

- ID: `uml.activity`.
- Initial, action, decision, merge, fork, join, activity-final and flow-final nodes.
- The same graphical editing mechanisms as EPC, with separate domain rules.
- Control flows and guard labels; no incoming links to initial nodes or outgoing
  links from finals, with node-specific degree diagnostics.
- SVG, Graphviz DOT and canvas PNG export.

Guards are documentary text, not evaluated expressions. Object flows, partitions,
pins, interruptible regions, structured activities, simulation and XMI exchange
are not implemented. Reference: [OMG UML 2.5.1](https://www.omg.org/spec/UML/2.5.1/About-UML).

## Code boundaries

| Responsibility | Implementation |
| --- | --- |
| DMN model, evaluation and XML | [dmn.rs](../crates/op-core/src/dmn.rs) |
| Typed graph mechanisms | [flow.rs](../crates/op-core/src/flow.rs) |
| EPC rules | [epc.rs](../crates/op-core/src/epc.rs) |
| Activity rules | [activity.rs](../crates/op-core/src/activity.rs) |
| DMN editor/session | [dmn_editor.rs](../crates/op-app/src/dmn_editor.rs) |
| Graphical editor/sessions | [flow_editor.rs](../crates/op-app/src/flow_editor.rs) |
| Static composition | [composition.rs](../crates/op-app/src/composition.rs) |

`Diagram<EpcKind>` and `Diagram<ActivityKind>` share graph mechanisms but remain
distinct Rust model types. The shell uses the registry without notation-specific
branches. Camera, selection and inspector state belong to each view; models and
histories are shared per document. Cached DMN evaluation is invalidated when the
table or inputs change, including across panels.

BPMN XML detection distinguishes `process` from DMN `decision` elements despite
both formats using a `definitions` root. The `.opx` envelope remains v2 and each
new model starts at v1; historical schemas are unchanged.

See [verification](verification.md#dmn-epc-and-activity-notations) for checks actually
performed. The next product priority is the
[Looping data-design pipeline](parite-looping-bizagi.md).
