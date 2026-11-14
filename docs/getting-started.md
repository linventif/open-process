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

Use **File → Save** or **Ctrl/Cmd+S** to save an editable `.opx` document.
The desktop app writes to a chosen file; the browser downloads a file, which
must finish downloading to be available outside the browser. Subsequent browser
saves create downloads rather than updating the original file in place.
The browser retains the modified marker and close confirmation after starting a
download. Verify the downloaded file before choosing **Close without saving**.

Empty documents close directly from their tab or **File → Close tab**, even if
they were renamed or all their elements were deleted. A blank source/log or a
default decision table without rules also closes directly. Unsaved content,
customized decision clauses, exercise answers and recoverable original data
retain the save confirmation.

Use **File → Open** or **Ctrl/Cmd+O** to reopen a saved file or import one of the
supported formats. Files can also be dragged onto the application.

The application can restore a previous local session. This is separate from a
saved file: keep `.opx` copies of work you need to retain or move to another
device. See [data and storage](data-and-storage.md) for details and deletion.

## Export and interpret results

**File → Export** lists the formats supported by the active document. Image
exports are useful for sharing; save `.opx` as well to retain editable content.
An export or conversion may not preserve every feature of the original notation.
PNG export captures the visible diagram region.

Some analyses have supported model classes and exploration limits. Read the
diagnostics and any truncation notices before interpreting a result; incomplete
exploration is not proof of a property. In particular, BPMN process evaluation
supports structured, acyclic XOR/AND models as described in the README.

The project is under active development and does not claim complete feature
parity with other modelling tools. For help or to report a problem, use
[SUPPORT.md](../SUPPORT.md). Contributions of minimal examples and clearer
documentation are welcome.
