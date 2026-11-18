#!/usr/bin/env python3
"""Rasterize the apx typography sizes from the library's existing font faces.
Run with: uv run --with pillow python crates/aeris-ui/scripts/build-typography.py
"""
from pathlib import Path
from struct import pack
from PIL import ImageFont

ROOT = Path(__file__).resolve().parents[1]
ASCII = ''.join(map(chr, range(32, 127))) + '\u00a0\u00b0\u00b7\u2013\u2212'

def write(face, size, name, characters=ASCII, line=None, descent=None):
    font = ImageFont.truetype(str(ROOT / 'fonts' / f'LinovalSansAeris-{face}.ttf'), size,
                              layout_engine=ImageFont.Layout.BASIC)
    glyphs = []
    for char in characters:
        mask, (x, y) = font.getmask2(char, 'L', anchor='ls')
        glyphs.append((round(font.getlength(char)), x, y, mask))
    if descent is None:
        descent = max(y + mask.size[1] for _, _, y, mask in glyphs)
    if line is None:
        line = max(-y for _, _, y, mask in glyphs if mask.size[0]) + descent + 1
    metrics, bitmap = bytearray([line, descent]), bytearray()
    for advance, x, y, mask in glyphs:
        width, height = mask.size
        metrics += pack('<HBBBbb', len(bitmap), advance, width, height, x, -y-height)
        bitmap += bytes(mask)
    (ROOT / 'assets' / f'linoval-{name}.font').write_bytes(metrics + bitmap)

for size in [31, 34, 38]:
    write(475, size, f'medium-{size}')
for size in [39, 53]:
    write('RoundedTitle650', size, f'rounded-{size}', " 'TUeimps")
for size in [36, 41, 49]:
    write('Rounded650', size, f'number-{size}', '0123456789:')
write('Rounded650', 22, 'clock-22', '-0123456789:', line=28, descent=5)
