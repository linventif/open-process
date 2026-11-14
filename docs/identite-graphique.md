# Appearance themes

**Status: eight selectable themes in the application and website. Classic is the default.**

Open Process offers several visual directions so they can be compared in the
running product. The previous application and presentation styling is restored
as **Classic**. The paper/graphite/vermilion direction remains an optional
**Workshop** theme.

| Theme | Direction |
| --- | --- |
| Classic | Original blue application controls, familiar rounded cards and the original navy/amber website. |
| Cloud | Airy blue, soft shadows, pill actions and generously rounded cards. |
| Lilac | Lavender, expressive headings and a two-column website catalogue. |
| Blossom | Soft pink, large curves, serif website headings and a centered opening. |
| Lagoon | Teal, rounded frames and a reversed illustrated website opening. |
| Midnight | Deep blue and cyan; selecting it starts in dark mode. |
| Graphite | Neutral monochrome, compact corners and an editorial website grid. |
| Workshop | Paper, graphite, vermilion and small angular frames. |

Every family supports light and dark application and website colors. The website
also varies composition and heading typography. Inter remains the application's
bundled UI typeface, and handwriting remains an explicit drawing style.

## Select and restore

The website has a sticky theme bar with a dropdown, quick swatches, a light/dark
selector and **Reset**. Reset restores Classic in light mode. The bar appears on
the English, French and Spanish presentation and legal pages. Choices survive
reloads and language navigation through the browser-local
`open-process-site-appearance-v1` preference. Storage failures leave the selector
usable for the current page. Theme selection does not change the language.

The application exposes the same choices in the navigation bar when space
allows, and always in **View**. Its sun/moon control remains independent. The
`open-process-appearance-v1` setting stores only the selected family; eframe
continues to own the light/dark preference. Unknown or malformed saved theme
values fall back to Classic. A preference is scoped to the application context,
not a process-global color setting.

Opening the web application from a themed website passes `theme` and `mode` URL
parameters. Explicit valid parameters override the app's stored selection.
Selecting another theme updates an existing preview URL so reloading preserves
the latest selection. No server-side account or synchronization is involved.

## Implementation boundary

[appearance.json](../crates/op-app/assets/appearance.json) is the shared catalogue
of light/dark colors and control/card geometry. The Rust adapter in
[theme/presets.rs](../crates/op-app/src/theme/presets.rs) and the website adapter in
[appearance.ts](../website/src/lib/appearance.ts) consume it. Classic retains its
original application palette and a website-specific navy/amber override.

[theme.rs](../crates/op-app/src/theme.rs), the shared canvas primitives and home
cards apply geometry centrally. Graphical editors retain the common layout and
notation-specific symbols. Theme selection changes neither models, histories,
cameras, saved language preferences nor `.opx` document schemas. Authored drawing
styles and semantic success/error/warning colors keep their existing meaning.
The original O → P mark remains the shared application/website icon.

## Real gallery previews

The website switches between real captures for the selected theme and color
mode; it does not recolor a screenshot with CSS. Eight gallery views across
eight families and two modes produce 128 images, loaded individually on demand.
To regenerate them from the repository root:

```sh
cargo test --locked -p op-app appearance_gallery_captures_keep_document_snapshots_unchanged -- --test-threads=1
python3 scripts/export-theme-previews.py
```

The exporter requires Pillow and validates all source images before writing the
website output. Stable fallback image URLs remain for metadata and rendering
without JavaScript. See [the verification record](verification.md) for checks
actually performed on this implementation.
