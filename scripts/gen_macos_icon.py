#!/usr/bin/env python3
"""
Generate Rusdle.iconset (all sizes) for use with `iconutil -c icns`.

Same design as the Windows ICO produced by build.rs:
  - Rusdle-green (#538D4E) background with subtle darker corners
  - White blocky "R" scaled to ~55 % of the icon height

Usage:
  python3 scripts/gen_macos_icon.py [output_dir]
  # output_dir defaults to "Rusdle.iconset"
"""

import os, struct, sys, zlib

# ── Design ────────────────────────────────────────────────────────────────────

GREEN = (83, 141, 78, 255)     # #538D4E  — Rusdle green  (RGBA)
WHITE = (255, 255, 255, 255)
DARK  = (56, 107, 60, 255)     # slightly darker for 2-px corner accent

# 6-column × 7-row block "R" — identical to the bitmap in build.rs
R_BITS = [
    [1, 1, 1, 1, 0, 0],
    [1, 0, 0, 0, 1, 0],
    [1, 0, 0, 0, 1, 0],
    [1, 1, 1, 1, 0, 0],
    [1, 0, 1, 0, 0, 0],
    [1, 0, 0, 1, 0, 0],
    [1, 0, 0, 0, 1, 0],
]
R_COLS, R_ROWS = 6, 7

# ── Pixel generation ──────────────────────────────────────────────────────────

def make_pixels(size: int) -> list:
    px = [GREEN] * (size * size)
    s = size

    # 2-px corner accent (matches build.rs)
    corners = [
        (0, 0), (1, 0), (0, 1),
        (s-1, 0), (s-2, 0), (s-1, 1),
        (0, s-1), (1, s-1), (0, s-2),
        (s-1, s-1), (s-2, s-1), (s-1, s-2),
    ]
    for x, y in corners:
        if 0 <= x < s and 0 <= y < s:
            px[y * s + x] = DARK

    # Scale R so it fills ~55 % of the icon height
    scale = max(1, (size * 55 // 100) // R_ROWS)
    rw, rh = R_COLS * scale, R_ROWS * scale
    x0, y0 = (size - rw) // 2, (size - rh) // 2

    for row, bits in enumerate(R_BITS):
        for col, bit in enumerate(bits):
            if bit:
                for dy in range(scale):
                    for dx in range(scale):
                        x = x0 + col * scale + dx
                        y = y0 + row * scale + dy
                        if 0 <= x < s and 0 <= y < s:
                            px[y * s + x] = WHITE

    return px

# ── Minimal PNG encoder (stdlib only) ────────────────────────────────────────

def _png_chunk(tag: bytes, data: bytes) -> bytes:
    crc = zlib.crc32(tag + data) & 0xFFFFFFFF
    return struct.pack(">I", len(data)) + tag + data + struct.pack(">I", crc)

def write_png(path: str, size: int, pixels: list) -> None:
    # IHDR: width, height, bit-depth=8, colour-type=6 (RGBA), comp=0, filter=0, interlace=0
    ihdr = struct.pack(">II", size, size) + bytes([8, 6, 0, 0, 0])

    # Scanlines: filter-byte 0 (None) followed by RGBA bytes
    raw = bytearray()
    for y in range(size):
        raw += b"\x00"
        for x in range(size):
            raw += bytes(pixels[y * size + x])

    png = (
        b"\x89PNG\r\n\x1a\n"
        + _png_chunk(b"IHDR", ihdr)
        + _png_chunk(b"IDAT", zlib.compress(bytes(raw), 9))
        + _png_chunk(b"IEND", b"")
    )
    os.makedirs(os.path.dirname(os.path.abspath(path)), exist_ok=True)
    with open(path, "wb") as f:
        f.write(png)

# ── iconutil filename table ───────────────────────────────────────────────────
# Each entry: (pixel_size, filename_inside_iconset)

ENTRIES = [
    (16,   "icon_16x16.png"),
    (32,   "icon_16x16@2x.png"),
    (32,   "icon_32x32.png"),
    (64,   "icon_32x32@2x.png"),
    (128,  "icon_128x128.png"),
    (256,  "icon_128x128@2x.png"),
    (256,  "icon_256x256.png"),
    (512,  "icon_256x256@2x.png"),
    (512,  "icon_512x512.png"),
    (1024, "icon_512x512@2x.png"),
]

# ── Main ──────────────────────────────────────────────────────────────────────

if __name__ == "__main__":
    out_dir = sys.argv[1] if len(sys.argv) > 1 else "Rusdle.iconset"

    cache: dict[int, list] = {}
    for size, name in ENTRIES:
        if size not in cache:
            cache[size] = make_pixels(size)
        write_png(os.path.join(out_dir, name), size, cache[size])
        print(f"  wrote {name}  ({size}×{size})")

    print(f"\nIconset ready at: {out_dir}/")
    print("Run:  iconutil -c icns", out_dir, "-o AppIcon.icns")
