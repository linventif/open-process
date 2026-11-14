# Open Process icon

**Status: implemented platform integration.**

The master is [open-process.svg](../crates/op-app/assets/icons/open-process.svg),
a vector of circles and paths without a background or embedded image. It uses
the navy/amber O → P motif with transparent gaps. The historical raster proposal
remains in [source.png](../crates/op-app/assets/icons/source.png), with
[generation prompts](../crates/op-app/assets/icons/source-prompt.txt), but no
longer generates platform icons.

## Implemented uses

| Use | Integration |
| --- | --- |
| Native window/taskbar | PNG embedded in [main.rs](../crates/op-app/src/main.rs) through `ViewportBuilder::with_icon`. |
| Menus, home and About | [brand.rs](../crates/op-app/src/brand.rs), SVG-derived transparent PNG with light strokes for dark mode. |
| Windows Explorer executable | Multi-resolution ICO compiled by [build.rs](../crates/op-app/build.rs) and linked into the binary. |
| macOS Finder/Dock | ICNS in `Open Process.app/Contents/Resources`, referenced by `Info.plist`. |
| Linux application menu | Scalable SVG, PNG and `.desktop` launcher installed for the user. |
| Website/localized pages/404 | Transparent SVG logo, browser-theme-aware SVG favicon, ICO/PNG fallbacks and Apple icon. |
| WebAssembly app | Favicons and manifest with 192/512 px icons; relative paths support `/app/` and root Trunk builds. |
| Repository | Logo in the [README](../README.md). |

The web manifest enables standalone opening where supported; it does not add
offline operation.

## Regenerate formats

Install Python 3, Pillow and an SVG renderer (CairoSVG, or system Rsvg, PyGObject
and Pycairo), then run from the repository root:

```sh
python3 scripts/generate-icons.py
```

The script renders the SVG transparently, writes PNG/ICO/ICNS under
`crates/op-app/assets/icons/` and synchronizes website copies. It also produces
light strokes for egui dark mode; it does not run AI generation. Generated assets
are committed, so conversion tools are unnecessary for normal Rust/site CI builds.
No additional Rust dependency was introduced for the icon.

Windows MSVC uses `rc.exe` from `PATH` or the Windows SDK; `RC` can override its
path. Windows GNU can use `WINDRES`. Compilation explicitly fails when the
resource compiler is unavailable rather than publishing an iconless executable.

## Native packages

The release workflow calls [package_release.py](../scripts/package_release.py)
with Python 3.11 or later. After compilation, for example:

```sh
python3 scripts/package_release.py --target x86_64-unknown-linux-gnu --binary target/release/open-process --output out
```

Standalone executables remain available alongside Linux/macOS archives. The
website prefers archives with fallbacks for old releases. On macOS, extract and
move `Open Process.app` into Applications; `cargo run` alone does not create a
Finder bundle.

On Linux, extract and run `sh install.sh`. The binary goes in `~/.local/bin`, the
launcher in `~/.local/share/applications`, and SVG/PNG assets in the user's icon
theme. `XDG_DATA_HOME` is respected; `OP_INSTALL_PREFIX` overrides `~/.local`.

## Historical local validation

The recorded icon work checked SVG paths/circles without a background or embedded
image, PNG/ICO/ICNS alpha transparency, format generation/reading and the 32 px
favicon. Two Python tests using dummy binaries covered four distribution targets,
archive permissions, macOS metadata and temporary-directory Linux installation.
Website tests covered archive selection and legacy binary fallbacks.

Native/WebAssembly `cargo check --locked -p op-app` passed. Astro/Trunk produced
the combined web build and nine Bun tests passed, including matching app/site
icons and manifest paths under `/app/`. A root Trunk HTML build checked relative
paths. Home/menu icons were inspected in light/dark mode. Local links, the desktop
launcher, installer syntax and `git diff --check` were checked.

After the transparent SVG change, the sequential workspace suite passed 23 app
and 49 core tests. Previous failures from parallel refactoring were absent in
that recorded checkout. These historical local checks do not replace CI or
validate the current checkout. Windows/macOS display and package publication
still require platform-specific verification.
