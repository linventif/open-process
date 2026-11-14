# Open Process visual identity

**Status: implemented in the application and presentation website.**

Open Process uses the visual language of a modeling workshop: paper, graphite,
vermilion, construction lines and explicit connections. The O → P mark links an
open circle to a process. The identity belongs to the editing tool as well as its
website; the website shows actual application captures.

## Palette

| Role | Light surface | Dark surface |
| --- | --- | --- |
| Main background | Paper `#F3F1E8` | Graphite `#232722` |
| Canvas / deep background | Sheet `#FCFBF7` | Deep graphite `#1B1F1B` |
| Raised surface | Sheet `#FCFBF7` | `#2B3029` |
| Text | Ink `#252923` | Paper `#F3F1E8` |
| Secondary text | `#61675F` | `#B6BCAE` |
| Rules and borders | `#C7C9BC` | `#4C5548` |
| Interaction accent | Vermilion `#C4472D` | Light vermilion `#F58B6F` |

The application defines these colors in [theme.rs](../crates/op-app/src/theme.rs).
The website mirrors the tokens in
[global.css](../website/src/styles/global.css). The shared canvas palette derives
its neutral surfaces, lines and selection accent from the theme. Existing
notation colors, model-owned drawing styles and semantic success/error/warning
colors retain their meaning. Recoloring the interface does not recolor saved
user drawings.

Primary actions use vermilion with sheet-colored text on light surfaces, or light
vermilion with ink text on dark surfaces. Selected tabs and keyboard focus also
have outlines or rules, so their state is not communicated by color alone.

## Typography and geometry

Inter is bundled locally in both products. Semibold establishes headings and
controls; monospace identifies section references, diagram badges and technical
information. Handwriting remains available as an explicit diagram style.

Interface frames have small 2–3 px corners, thin visible rules and compact
controls. Popups use short, unblurred offset shadows. Node shapes still follow
their notation: interface geometry must not replace BPMN events, gateways or
other domain symbols.

Home and creation cards share ruled frames, neutral icon fields and a short
vermilion registration line. The website uses the same treatment for its
catalogue, controls and gallery tabs. Its graphite header and process drawing
pair with the light paper page. Construction grids and numbered references
support the subject; they do not introduce fictitious product controls.

## Assets and maintenance

The [SVG master](../crates/op-app/assets/icons/open-process.svg) remains the
source of the O → P mark. Run `python3 scripts/generate-icons.py` to synchronize
native icons, web app icons, website icons and the light header logo. See the
[icon guide](icone.md) for platform integration.

Keep the website gallery synchronized with real captures when interface styling
changes; [website/README.md](../website/README.md#seo-and-content) lists their
sources. Keep localized routes, keyboard focus, reduced-motion preferences and
light/dark application themes available when extending this identity.
