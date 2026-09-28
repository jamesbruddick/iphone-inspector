#!/usr/bin/env python3
"""Draws the app icon and the installer artwork from the favicon's shapes, and writes every file
the packages need. Standard library only; macOS for `iconutil`, which builds the .icns.

    python3 assets/generate.py

The outputs are committed, so this only needs running again when the design changes.
"""

import math
import shutil
import struct
import subprocess
import tempfile
import zlib
from pathlib import Path

HERE = Path(__file__).resolve().parent
ROOT = HERE.parent

# The favicon's indigo (#4f46e5) as the middle of a gentle top-to-bottom gradient.
TOP = (99, 102, 241)
BOTTOM = (67, 56, 202)
WHITE = (255, 255, 255)


def clamp(v, lo=0.0, hi=1.0):
    return lo if v < lo else hi if v > hi else v


def round_rect(px, py, x, y, w, h, r):
    """Rounded-rectangle signed distance: negative inside. Same as src/icon.rs."""
    qx = abs(px - (x + w / 2)) - (w / 2 - r)
    qy = abs(py - (y + h / 2)) - (h / 2 - r)
    return math.hypot(max(qx, 0), max(qy, 0)) + min(max(qx, qy), 0) - r


def segment(px, py, a, b):
    (ax, ay), (bx, by) = a, b
    dx, dy = bx - ax, by - ay
    t = clamp(((px - ax) * dx + (py - ay) * dy) / (dx * dx + dy * dy))
    return math.hypot(px - (ax + t * dx), py - (ay + t * dy))


def mark(x, y):
    """Distance to the phone outline and check mark, in the favicon's 32-unit space."""
    outline = abs(round_rect(x, y, 10, 6, 12, 20, 2.5)) - 0.9
    check = min(segment(x, y, (12.6, 16.4), (14.9, 18.8)), segment(x, y, (14.9, 18.8), (19.4, 13.8))) - 1.0
    return min(outline, check)


def render(size, inset, radius, shadow):
    """RGBA rows of the icon at `size` pixels: the 32-unit tile drawn `inset` units in from each
    edge (of a 32-unit canvas), with corner `radius`, and optionally the soft shadow a Mac icon sits on."""
    body = 32 - 2 * inset
    per_unit = size / 32
    rows = []
    for py in range(size):
        row = bytearray()
        v = (py + 0.5) / per_unit
        t = clamp((v - inset) / body)
        tile_rgb = [TOP[i] + (BOTTOM[i] - TOP[i]) * t for i in range(3)]
        for px in range(size):
            u = (px + 0.5) / per_unit
            d_tile = round_rect(u, v, inset, inset, body, body, radius)
            a_tile = clamp(0.5 - d_tile * per_unit)
            # The mark is laid out on the favicon's full 32-unit tile; scale it onto this one.
            mu, mv = (u - inset) * 32 / body, (v - inset) * 32 / body
            a_mark = clamp(0.5 - mark(mu, mv) * per_unit * body / 32) if a_tile > 0 else 0.0
            rgb = [c + (255 - c) * a_mark for c in tile_rgb]
            alpha = a_tile
            if shadow and a_tile < 1:
                d = round_rect(u, v - 0.35, inset, inset, body, body, radius)
                a_shadow = 0.28 * clamp(1 - max(d, 0) / 0.9) ** 2
                # Tile over shadow: the shadow is black, so it only adds alpha beneath the tile.
                out = a_tile + a_shadow * (1 - a_tile)
                rgb = [c * a_tile / out if out else 0 for c in rgb]
                alpha = out
            row += bytes((round(rgb[0]), round(rgb[1]), round(rgb[2]), round(alpha * 255)))
        rows.append(bytes(row))
    return rows


def mac_icon(size):
    # Apple's grid: an 824-pixel body in a 1024 canvas, corner radius about 22% of the body.
    inset = 32 * 100 / 1024
    return render(size, inset, (32 - 2 * inset) * 0.2237, shadow=True)


def flat_icon(size):
    # Windows and Linux: the favicon tile with a hair of margin, no shadow.
    return render(size, 1, 6.5, shadow=False)


def png(rows):
    size = len(rows)

    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    raw = b"".join(b"\x00" + r for r in rows)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")


def ico(images):
    """An .ico holding PNG images, which Windows has read since Vista."""
    header = struct.pack("<HHH", 0, 1, len(images))
    offset = 6 + 16 * len(images)
    entries, data = b"", b""
    for size, blob in images:
        dim = 0 if size >= 256 else size
        entries += struct.pack("<BBBBHHII", dim, dim, 0, 0, 1, 32, len(blob), offset + len(data))
        data += blob
    return header + entries + data


def bmp(width, height, pixel):
    """A 24-bit bottom-up .bmp, the format the Windows Installer dialogs take."""
    stride = (width * 3 + 3) & ~3
    body = bytearray()
    for y in range(height - 1, -1, -1):
        row = bytearray()
        for x in range(width):
            r, g, b = pixel(x, y)
            row += bytes((b, g, r))
        body += row + b"\x00" * (stride - len(row))
    header = struct.pack("<2sIHHI", b"BM", 54 + len(body), 0, 0, 54)
    info = struct.pack("<IiiHHIIiiII", 40, width, height, 1, 24, 0, len(body), 2835, 2835, 0, 0)
    return header + info + bytes(body)


def over(rows, x0, y0, background):
    """A pixel function that draws `rows` (RGBA) at (x0, y0) over `background`."""
    size = len(rows)

    def pixel(x, y):
        base = background(x, y)
        if x0 <= x < x0 + size and y0 <= y < y0 + size:
            i = (x - x0) * 4
            r, g, b, a = rows[y - y0][i : i + 4]
            a /= 255
            return tuple(round(c * a + base[k] * (1 - a)) for k, c in enumerate((r, g, b)))
        return base

    return pixel


def rtf(text):
    """The license as the rich text the installer's license page shows."""
    paragraphs = [" ".join(p.split()) for p in text.strip().split("\n\n")]
    escaped = [p.replace("\\", "\\\\").replace("{", "\\{").replace("}", "\\}") for p in paragraphs]
    body = "\\par\\par\n".join(escaped)
    return "{\\rtf1\\ansi\\deff0{\\fonttbl{\\f0\\fswiss Segoe UI;}}\\f0\\fs18\n" + body + "\\par\n}\n"


def main():
    # macOS: the .icns, from an iconset of every size Finder asks for.
    with tempfile.TemporaryDirectory() as tmp:
        iconset = Path(tmp) / "AppIcon.iconset"
        iconset.mkdir()
        for points in (16, 32, 128, 256, 512):
            for scale in (1, 2):
                name = f"icon_{points}x{points}{'@2x' if scale == 2 else ''}.png"
                (iconset / name).write_bytes(png(mac_icon(points * scale)))
        (HERE / "macos").mkdir(exist_ok=True)
        subprocess.run(["iconutil", "-c", "icns", str(iconset), "-o", str(HERE / "macos" / "AppIcon.icns")], check=True)
        shutil.copy(iconset / "icon_512x512@2x.png", HERE / "icon.png")

    # Windows: the .ico built into the .exe and shown for the installed app.
    (HERE / "windows").mkdir(exist_ok=True)
    (HERE / "windows" / "app.ico").write_bytes(ico([(s, png(flat_icon(s))) for s in (16, 20, 24, 32, 40, 48, 64, 256)]))

    # The installer's artwork: a 493x312 welcome/finish background (the left 164 pixels are the
    # picture, text goes on the white) and a 493x58 banner across the top of the other pages.
    def panel(x, y):
        if x >= 164:
            return WHITE
        t = y / 311
        return tuple(round(TOP[i] + (BOTTOM[i] - TOP[i]) * t) for i in range(3))

    icon = flat_icon(96)
    (HERE / "windows" / "dialog.bmp").write_bytes(bmp(493, 312, over(icon, (164 - 96) // 2, 60, panel)))
    (HERE / "windows" / "banner.bmp").write_bytes(bmp(493, 58, over(flat_icon(40), 493 - 40 - 10, 9, lambda x, y: WHITE)))
    (HERE / "windows" / "License.rtf").write_text(rtf((ROOT / "LICENSE").read_text()))

    # Linux: the hicolor theme sizes the app menu picks from.
    for size in (16, 24, 32, 48, 64, 128, 256, 512):
        out = HERE / "linux" / "icons" / f"{size}x{size}"
        out.mkdir(parents=True, exist_ok=True)
        (out / "iphone-inspector.png").write_bytes(png(flat_icon(size)))


if __name__ == "__main__":
    main()
