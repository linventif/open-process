#!/usr/bin/env python3
"""Export the real Classic light/dark captures to the website gallery."""
from pathlib import Path
from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
VIEWS = ('draw', 'database', 'uml', 'bpmn', 'petri', 'mining', 'mermaid', 'split')
SOURCE = ROOT / 'target/screenshots/classic'
DESTINATION = ROOT / 'website/public/assets/classic'


def main():
    captures = []
    for mode in ('light', 'dark'):
        for view in VIEWS:
            relative = Path(mode) / (view + '.png')
            path = SOURCE / relative
            if not path.is_file():
                raise SystemExit(f'Missing capture: {path}. Run the Classic gallery UI test first.')
            with Image.open(path) as image:
                if image.size != (1400, 860):
                    raise SystemExit(f'Unexpected capture size: {path}: {image.size}')
            captures.append(relative)
    for relative in captures:
        destination = (DESTINATION / relative).with_suffix('.webp')
        destination.parent.mkdir(parents=True, exist_ok=True)
        with Image.open(SOURCE / relative) as image:
            image.convert('RGB').save(destination, 'WEBP', quality=90)
    # Existing metadata and the no-JavaScript gallery keep stable asset URLs.
    for view in VIEWS:
        mode = 'dark' if view == 'draw' else 'light'
        path = DESTINATION / mode / (view + '.webp')
        (ROOT / 'website/public/assets' / (view + '.webp')).write_bytes(path.read_bytes())
    print(f'Exported {len(captures)} Classic previews and eight fallback images.')


if __name__ == '__main__':
    main()
