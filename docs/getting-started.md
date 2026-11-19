# Getting started

Open Process is a free tool for learning, modelling and analysing processes.
It runs as a desktop application and in a browser. Features and limits vary by
notation; consult the [README](../README.md) and linked notation guides.

## Open the application

Use the [browser app](https://open-process.linv.dev/app/) without installing
anything, or download your platform's build from
[GitHub Releases](https://github.com/linventif/open-process/releases).
Desktop packaging and installation are described in the
[README](../README.md#automatic-releases).

## Desktop updates

Official desktop builds check GitHub for a newer release once at startup without
blocking the interface. When an update is available, a popup shows the installed
and available release identifiers, a download link for this computer, and a link
to release notes and other downloads. **Later** dismisses it for this session.
Download the new build, close Open Process, then replace the installed application.

Use **Help → Check for updates…** to retry or check manually. A network failure
at startup stays silent; a manual check reports failures and links to the releases.
Updates are downloaded through your browser and installed by you.

Stable `vX.Y.Z` builds check the latest stable release. Automatic `main-*` builds
check the 30 most recent releases for a higher run number with a different commit.
These channels do not suggest each other's builds. Tagged semantic preview builds
can also see newer semantic previews or stable releases in that recent list.
The release identifier appears in **Help → About Open Process** alongside the
package version and build identifier. Local builds without a recognized release
tag skip the startup check; their manual check uses the package version against
stable releases. The browser application does not run the desktop update check.

## Create a first diagram

The home screen and **New document** picker start with free drawing, relational
database design, UML class diagrams, BPMN and UML use cases, in that order. Other
notations follow; the home screen also offers guided workflows.

1. On the home screen, choose a diagram type, such as BPMN or a Petri net.
   The **Examples** button on a type card opens sample models for that type.
2. In a document, use **File → New…**, the tab bar's **+**, or **Ctrl/Cmd+N**
   to choose another document type.
3. Select a palette tool and place elements on the canvas. Use the notation's
   connection tool to link them, and the inspector to edit their properties.
4. Use the view configuration bar to adjust zoom, grid and inspector visibility.
   Compact windows group options into menus. Interface labels follow the selected
   language; this guide uses their English names.
5. Use **Edit → Undo** or **Ctrl/Cmd+Z** to undo an edit.

For a guided exploration, open a built-in example before creating your own
model. Analysis and simulation controls depend on the notation: Petri nets offer
a token game and analysis, while process discovery starts from an event log.

## Save and reopen your work

Work is saved automatically. Desktop versions create **Documents/Open Process**
and write editable `.opx` copies there. The browser keeps automatic copies in
local storage for the app's origin. **Saved diagrams** on the home screen reopens
documents after their tabs have been closed.

**File → Save**, **Save as** and **Ctrl/Cmd+S** remain available for manual copies:
choose a file on desktop or download an `.opx` file in the browser. Later native
**Save** writes that chosen file; automatic saving continues in the managed
folder. Browser downloads create independent files and must finish before they
are available outside the browser.

Empty documents close directly from their tab or **File → Close tab**, even if
they were renamed or all their elements were deleted. A blank source/log or a
default decision table without rules also closes directly. Closing a nonempty
document finishes its automatic save first. If saving fails, its content stays
open with a save confirmation and an error; a manual copy can then be saved.

Use **File → Open** or **Ctrl/Cmd+O** to reopen a saved file or import one of the
supported formats. Select several files at once to open each in its own tab.
Files can also be dragged onto the application. A file saved by a newer Open
Process asks whether to update first or open it anyway, listing what this
version does not support.

The application also restores open tabs and layout. Keep manual `.opx` copies of
work you need to move to another device or retain independently of browser data.
See [data and storage](data-and-storage.md) for timing, limits and deletion.

## Export and interpret results

**File → Export** lists the formats supported by the active document. Image
exports are useful for sharing; save `.opx` as well to retain editable content.
An export or conversion may not preserve every feature of the original notation.
PNG export captures the visible diagram region.

BPMN, Petri, EPC and UML diagrams also export Mermaid source (`.mmd`) or a
Markdown document with a Mermaid block (`.mermaid.md`). Reopening these files
opens a Mermaid source editor. See [export formats](export-formats.md) for the
available formats and the limits of each projection.

Some analyses have supported model classes and exploration limits. Read the
diagnostics and any truncation notices before interpreting a result; incomplete
exploration is not proof of a property. In particular, BPMN process evaluation
supports structured, acyclic XOR/AND models as described in the README.

The project is under active development and does not claim complete feature
parity with other modelling tools. For help or to report a problem, use
[SUPPORT.md](../SUPPORT.md). Contributions of minimal examples and clearer
documentation are welcome.
