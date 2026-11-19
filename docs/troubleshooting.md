# Troubleshooting

Start with a small example of the same document type. If it works, compare its
structure, input format and size with your document. Preserve a separate `.opx`
copy before attempting recovery or simplifying a model.

## macOS blocks the application

Download the archive for your Mac from the project's
[GitHub Releases](https://github.com/linventif/open-process/releases), extract it,
and move **Open Process.app** into **Applications**. Use the Apple Silicon archive
for M-series Macs and the Intel archive for Intel Macs.

The current packaging script ad-hoc signs the complete application bundle and
verifies its signature. It does not provide Apple Developer ID signing or
notarization. Builds made before this packaging fix did not seal the complete
bundle, which can contribute to macOS reporting the app as "damaged". That alert
alone does not establish the cause; a corrupted download or modified app can
also trigger it. Prefer a new build containing the fix when one is available.

For an unidentified-developer or notarization alert, try opening the app once,
then use **System Settings → Privacy & Security → Open Anyway** if offered and
you trust the download. See [Apple's explanation of these alerts](https://support.apple.com/en-us/102445).

For an older official build still reported as damaged, first compare the archive's
SHA-256 with its entry in the same release's **SHA256SUMS** using
`shasum -a 256 /path/to/the/archive.tar.gz`. If they match and you trust that
release, this local workaround re-signs and verifies only this application, then
removes its download quarantine attribute:

```sh
codesign --force --sign - "/Applications/Open Process.app" &&
codesign --verify --deep --strict "/Applications/Open Process.app" &&
xattr -dr com.apple.quarantine "/Applications/Open Process.app"
```

Reopen the app from Finder. This is a local exception, not Apple notarization.
If signing or verification fails, the remaining commands do not run; keep the
error text when reporting the problem. Do not use this workaround for a checksum
mismatch or an alert explicitly reporting malware. The
[browser app](https://open-process.linv.dev/app/) is also available without a
desktop installation.

## No desktop update popup appears

Use **Help → Check for updates…** to retry and see the check's result, or open
[GitHub Releases](https://github.com/linventif/open-process/releases) directly.
Startup checks stay silent when offline, rate-limited, current, or without an
uploaded package for this system. A dismissed notice returns only on another
check or a later launch. Local builds without an embedded release tag do not
check automatically. Stable and automatic `main-*` builds use separate version
channels; see [desktop updates](getting-started.md#desktop-updates).

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
