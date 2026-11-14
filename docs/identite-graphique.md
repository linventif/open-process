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
[color-mode.css](../website/src/styles/color-mode.css). Its mode-only preference
is `open-process-color-mode-v1`. When the former
`open-process-site-appearance-v1` preference exists, only its light/dark choice is
retained, then the retired setting is removed if browser storage is writable.
Storage failures leave the current page usable. Reloads and language navigation
retain the selected color mode.

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
