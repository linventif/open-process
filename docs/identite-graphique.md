# Classic visual identity

**Status: Classic is the sole application and website theme, with light/dark color modes.**

Open Process keeps its original rounded interface, Inter typography and navy/amber
O → P mark. Application actions use the familiar blue accent. The presentation
site retains its light paper background, navy text and amber details. Dark mode
uses the corresponding Classic palette.

The alternative theme families, comparison bar, swatches and theme catalogue have
been removed. The website has a compact sun/moon button alongside its language
menu; the application keeps its existing light/dark controls. Saved language
preferences remain independent.

## Implementation

[theme.rs](../crates/op-app/src/theme.rs) owns application colors and shared
control/card radii. Canvas primitives and home cards consume these constants;
notation-specific symbols and authored drawing styles retain their own meaning.
There is no selectable theme registry or separate application family setting.
Previously saved family values are ignored. Eframe continues to own the app's
light/dark preference.

The website uses [global.css](../website/src/styles/global.css) and
[color-mode.css](../website/src/styles/color-mode.css). Without an explicit
choice, it follows `prefers-color-scheme` on load and listens for live system
changes. Automatic mode never writes a color preference. Logos, the selected
gallery view, application links and browser theme-color metadata update together.

A sun/moon button choice or valid `mode` URL is saved in
`open-process-color-mode-v2` and takes priority over the system across reloads
and language navigation. Other tabs follow changes to that preference; removing
it resumes system tracking. Storage failures still allow automatic updates and
a manual choice for the current page.

The former `open-process-color-mode-v1` and
`open-process-site-appearance-v1` settings are removed once. They saved defaults
as well as manual choices, so their values cannot establish an explicit override.
This also resets old manual color choices; the sun/moon button can save a new one.
Saved language preferences are preserved.

Explicit `mode=light` or `mode=dark` links select a web-app color mode. Old `theme`
parameters cannot select another family; website links remove that parameter.
Theme cleanup changes neither documents, histories, cameras nor `.opx` schemas.

## Gallery and assets

The gallery has eight real application views in each color mode: 16 Classic
captures, loaded individually. Regenerate them with:

```sh
cargo test --locked -p op-app classic_gallery_captures_keep_document_snapshots_unchanged -- --test-threads=1
python3 scripts/export-theme-previews.py
```

The exporter requires Pillow and validates all source captures before writing
`website/public/assets/classic/`. Stable root image URLs remain for metadata and
the no-JavaScript gallery. Alternative-theme images are not shipped.

The [icon guide](icone.md) describes the original SVG master and native/web
formats. See [the verification record](verification.md) for checks actually run.
