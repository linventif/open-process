# Troubleshooting

Start with a small example of the same document type. If it works, compare its
structure, input format and size with your document. Preserve a separate `.opx`
copy before attempting recovery or simplifying a model.

## A document is missing

**Resume your work** refers to documents still open in the workspace.
**Saved diagrams** lists closed automatic copies. On desktop, also check
**Documents/Open Process** and any location chosen through **Save as**.

Browser automatic copies belong to the current browser profile and application
origin. Another device, profile or URL does not share them. Clearing site data
removes those copies; private browsing can make them temporary. Open a manual
`.opx` download to restore an independent copy. See [data and storage](data-and-storage.md).

## Saving fails or a tab stays open

A nonempty document finishes automatic saving before closing. A storage error
keeps it open so the content is not silently discarded. Save a manual `.opx`
copy, then check disk access or browser storage capacity. A browser download
must finish before it becomes an independent file. Do not clear site data while
the only copy of a document remains there.

## An imported file does not open

Confirm the active format in the [format guide](export-formats.md). Export
support does not automatically mean import support: PlantUML output, for example,
does not establish a PlantUML importer. Mermaid Markdown must contain exactly
one complete fenced Mermaid block. DMN XML must fit the supported single-table
subset. An older application may need updating to understand a newer model.

For `.opx` compatibility and recovery behaviour, see
[documents and compatibility](documents-et-compatibilite.md). Keep the original
bytes when reporting a failed migration rather than overwriting the file.

## A result, preview or command is unavailable

- Check **exam mode**. It suppresses computed assistance, simulations and
  generated-answer actions while preserving authored documents.
- Read the current diagnostic. Invalid drafts can remain saveable even when
  analysis, evaluation or preview is unavailable.
- Check the notation's supported subset. Some Mermaid families support source
  editing and exports without a local preview. BPMN evaluation requires a
  supported structured, acyclic model.
- Wait for the current background job. Editing its input invalidates older
  results; an outdated preview cannot be exported as the current SVG.
- Simplify large input or reduce exploration limits. Web jobs are capped at
  30 seconds and transfers at 8 MiB. Truncated exploration cannot prove absence
  of behaviours outside the explored state space.

## A panel or element seems hidden

Use zoom **Fit** and inspect the current document tab. Open the inspector from
the configuration bar; in narrow windows it may appear above the canvas, and
options move into a compact menu. Multiple views can have different cameras
while sharing one model. A keyboard shortcut can belong to a focused text field;
leave text editing before using a canvas shortcut.

## Report a reproducible problem

Use the [support guide](../SUPPORT.md) with the build identifier from About,
desktop OS or browser, document type, exact steps, expected result and error text.
Attach a minimal, non-sensitive example when appropriate. For a vulnerability,
follow [private security reporting](../SECURITY.md) instead of posting details
in a public issue.
