# Adding a notation

This guide describes implemented contracts. A module may provide one diagram,
several related types or a canvas-free document such as a table.

## 1. Model and codec in op-core

Define a typed model, invariants and operations independently of egui. Choose a
stable identifier, such as `state-machine`, and a model version. `DocumentCodec<M>`
defines encoding/decoding; `JsonCodec<M>` provides an initial JSON representation
and rejects unknown model versions.

For schema changes, implement notation-owned migrations before incrementing the
version and retain historical fixtures. Algorithms consume typed models; the JSON
envelope is an integration/storage boundary.

## 2. Adapter in op-app

Implement `EditorBackend` beside the editor. Existing adapters include
`*_editor/session.rs`. Responsibilities are:

- Expose the persistent model, its name and rename operations.
- Define typed view state for camera, inspector and local interactions.
- Capture/restore it, serializing only what is useful for session restoration.
- Declare commands, availability, secondary views and export formats.
- Produce semantic/graphical keys; geometry-derived relationships such as UML
  system containment must remain in the semantic key.
- Group mutations in history and allow transaction cancellation.

`Session::boxed(editor)` supplies the application interface, view handling and
revision tracking. Do not put the model or history into view state. A text editor
may use `()` as view state and ignore canvas mechanisms.

Graphical editors use `editor::layout::DiagramLayout` for the inspector,
`inspector_tabs`, scrollable `inspector_body`, configuration and canvas. Supply
`CanvasControls`, `FooterStatus` and notation-specific options. Bar heights,
spacing, zoom and compact-menu thresholds stay in the common component. Extra
business actions use the `tools` slot.

For home-screen previews, `EditorBackend::thumbnail_rect` may return the rendered
canvas rectangle. The fallback is the whole panel, useful for text/table views.
The shell owns capture and PNG encoding; modules do not manage preview files or
copy their models.

Keep snapping, inspector state and optional issue indicators in captured/restored
view state, serialized independently of the model. Historical boolean inspector
values remain readable. Drawing styles already stored in models remain persistent.

Standard exports use `ExportData`. Override `exports()` and `export_as(id)` for
multiple formats; the app builds buttons from descriptors. PNG is a canvas
capture command when supported.

## 3. Registration

Expose `register(&mut DiagramRegistry)` to add:

- A `DiagramDescriptor` per type with empty/opening factories and an `icon` function.
- External importers, if any; an export does not imply an import.
- Examples and optional home-screen workflow launchers. `ExampleDescriptor::kind`
  names the canonical type used by its card's **Examples** button in both grids.

Declare the Rust module and call `register` in
[composition.rs](../crates/op-app/src/composition.rs). Validation checks type IDs,
aliases and examples for duplicates, and rejects examples naming unknown types.

Commands/views use explicit IDs such as `state-machine.validate`. Labels use
stable translation keys, e.g. `tr("state_machine.validate")` or
`key("state_machine.validate")` in descriptors translated later by the shell.
Write original text in English and declare the same key in `en.rs`, `fr.rs` and
`es.rs` in `op-core/src/i18n/`. `trf` fills translated placeholders. See
[translations](traductions.md). Registered importers provide file filters.

Home and picker cards use descriptor icons independently of `home_order`.
`WorkflowDescriptor` also declares an icon. Reusable drawing primitives live in
[icons.rs](../crates/op-app/src/icons.rs); `icons::select` shares the BPMN selection
cursor with other graphical editors.

## 4. Workflows and computations

A conversion produces a new `Document` and diagnostics. The adapter emits
`HostEvent::Open` and, when needed, `HostEvent::Message`; it does not manipulate
application tabs or the target editor.

Long computations start from immutable snapshots and identify every input:
domain content, parameters and language when diagnostics depend on it. The
`platform/jobs.rs` service supplies transport/cancellation. Wire browser dispatch
in composition while keeping typed processing in the module. Never replace a
current result with a response for stale inputs.

## 5. Verify the extension

Test invariants, codec versions, transactions and undo to saved state. Check
multiple views and any conversions. Compile native and web targets. If adding a
worker, test the actual Trunk build at the root and under its publication path.

The `extension_without_canvas_uses_the_same_document_shell` test in
[editor/registry.rs](../crates/op-app/src/editor/registry.rs) demonstrates minimal
integration of a type outside the shipped catalogue.
