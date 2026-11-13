#!/usr/bin/env python3
"""Rasterize the transparent SVG master (Pillow + CairoSVG or system Rsvg)."""

from io import BytesIO
from pathlib import Path
import shutil

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / "crates/op-app/assets/icons"
WEB = ICONS / "web"


def rasterize(svg, size=1024):
    try:
        import cairosvg
    except ImportError:
        import cairo
        import gi

        gi.require_version("Rsvg", "2.0")
        from gi.repository import Rsvg

        surface = cairo.ImageSurface(cairo.FORMAT_ARGB32, size, size)
        handle = Rsvg.Handle.new_from_data(svg.encode())
        viewport = Rsvg.Rectangle()
        viewport.width = viewport.height = size
        handle.render_document(cairo.Context(surface), viewport)
        png = BytesIO()
        surface.write_to_png(png)
        data = png.getvalue()
    else:
        data = cairosvg.svg2png(bytestring=svg.encode(), output_width=size, output_height=size)
    return Image.open(BytesIO(data)).convert("RGBA")


def main():
    WEB.mkdir(parents=True, exist_ok=True)
    svg = (ICONS / "open-process.svg").read_text()
    image = rasterize(svg)

    def png(path, size):
        image.resize((size, size), Image.Resampling.LANCZOS).save(path, optimize=True)

    png(ICONS / "open-process.png", 512)
    rasterize(svg.replace("#1B2A41", "#F7F7F2")).resize((512, 512), Image.Resampling.LANCZOS).save(
        ICONS / "open-process-light.png", optimize=True,
    )
    image.save(ICONS / "open-process.ico", sizes=[(s, s) for s in (16, 24, 32, 48, 64, 128, 256)])
    image.resize((1024, 1024), Image.Resampling.LANCZOS).save(ICONS / "open-process.icns")
    for name, size in (("favicon.png", 32), ("apple-touch-icon.png", 180), ("icon-192.png", 192), ("icon-512.png", 512)):
        png(WEB / name, size)
    image.save(WEB / "favicon.ico", sizes=[(s, s) for s in (16, 32, 48)])
    favicon = svg.replace('</svg>', '  <style>@media (prefers-color-scheme: dark) { g { stroke: #F7F7F2; } }</style>\n</svg>')
    (WEB / "favicon.svg").write_text(favicon)
    site_icons = ROOT / "website/public/icons"
    site_icons.mkdir(parents=True, exist_ok=True)
    for path in WEB.iterdir():
        if path.suffix in (".png", ".ico", ".svg"):
            shutil.copyfile(path, site_icons / path.name)
    shutil.copyfile(ICONS / "open-process.png", ROOT / "website/public/assets/logo.png")
    shutil.copyfile(ICONS / "open-process.svg", ROOT / "website/public/assets/logo.svg")


if __name__ == "__main__":
    main()
