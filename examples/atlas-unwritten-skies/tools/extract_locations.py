#!/usr/bin/env python3
"""Reproducibly export independent 16:9 environment studies from source art.

The location source is a generated concept board; the player uses only the
pre-rendered, compact PNG files. No runtime dependencies are introduced.
These are first-pass paintings, not the final full-resolution location originals.
"""
from pathlib import Path
from PIL import Image, ImageFilter, ImageEnhance

ROOT = Path(__file__).resolve().parents[1]
SOURCE_LEFT = ROOT / 'assets/source/locations_concept_board_left.png'
SOURCE_RIGHT = ROOT / 'assets/source/locations_concept_board_right.png'
OUT = ROOT / 'assets/backgrounds'
LOCATIONS = {
    'lost_city': (327, 422, 624, 576),
    'cloud_coast': (910, 424, 1209, 577),
    'hidden_market': (20, 800, 364, 978),
    'name_garden': (1222, 422, 1532, 577),
    'archive_fire': (1222, 612, 1530, 754),
    'undersea_lens': (448, 795, 761, 980),
    'shattered_lens': (783, 798, 1135, 976),
    'journey_port': (628, 424, 898, 576),
}

def render_locations() -> None:
    left = Image.open(SOURCE_LEFT).convert('RGB')
    right = Image.open(SOURCE_RIGHT).convert('RGB')
    assert left.size == right.size == (768, 1024)
    source = Image.new('RGB', (1536, 1024))
    source.paste(left, (0, 0))
    source.paste(right, (768, 0))
    OUT.mkdir(parents=True, exist_ok=True)
    for name, region in LOCATIONS.items():
        crop = source.crop(region)
        width, height = crop.size
        target_ratio = 16/9
        if width / height > target_ratio:
            target_width = round(height * target_ratio)
            left = (width - target_width) // 2
            crop = crop.crop((left, 0, left + target_width, height))
        else:
            target_height = round(width / target_ratio)
            top = (height - target_height) // 2
            crop = crop.crop((0, top, width, top + target_height))
        crop = crop.resize((1280, 720), Image.Resampling.LANCZOS)
        crop = ImageEnhance.Contrast(crop).enhance(1.05)
        crop = crop.filter(ImageFilter.UnsharpMask(radius=1.35, percent=70, threshold=2))
        crop = crop.quantize(colors=192, method=Image.Quantize.MEDIANCUT,
                             dither=Image.Dither.FLOYDSTEINBERG)
        crop.save(OUT / (name + '.png'), optimize=True)

if __name__ == '__main__':
    render_locations()
