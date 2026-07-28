#!/usr/bin/env python3
"""Writes the AppImage's icon as a PNG.

Generated rather than committed for two reasons. The repository stays text
only - no binary is ever committed, which is the same rule that keeps game
content out - and the artwork stays inspectable: it is this file, not an opaque
blob nobody can diff.

The design is original: a chevron pointing along the direction of travel, over
the dark blue the front end draws on. Nothing here comes from the original
game's art.

Usage:
    scripts/appimage-icon.py out.png [--size 256]
"""

from __future__ import annotations

import argparse
import struct
import zlib

BACKGROUND = (0x0B, 0x10, 0x20)
CHEVRON = (0x3C, 0xE0, 0xFF)
ACCENT = (0xFF, 0x3C, 0x8C)


def chevron_distance(x: float, y: float) -> float:
    """Signed distance to a chevron centred on the origin, in icon units.

    The chevron is two mirrored line segments meeting at the tip, so the
    distance is the distance to the nearer of them.
    """
    y = abs(y)
    # The arm runs from the tip at (0.32, 0) back to (-0.34, 0.5).
    tip_x, tip_y = 0.32, 0.0
    end_x, end_y = -0.34, 0.5
    dx, dy = end_x - tip_x, end_y - tip_y
    along = ((x - tip_x) * dx + (y - tip_y) * dy) / (dx * dx + dy * dy)
    along = min(max(along, 0.0), 1.0)
    near_x, near_y = tip_x + dx * along, tip_y + dy * along
    return ((x - near_x) ** 2 + (y - near_y) ** 2) ** 0.5


def rounded_square(x: float, y: float, half: float, radius: float) -> float:
    """Signed distance to a rounded square, negative inside."""
    qx, qy = abs(x) - (half - radius), abs(y) - (half - radius)
    outside = (max(qx, 0.0) ** 2 + max(qy, 0.0) ** 2) ** 0.5
    inside = min(max(qx, qy), 0.0)
    return outside + inside - radius


def blend(under: tuple[int, int, int], over: tuple[int, int, int], alpha: float):
    return tuple(
        round(u + (o - u) * alpha) for u, o in zip(under, over, strict=True)
    )


def pixels(size: int) -> bytes:
    """One RGBA row per line, each prefixed with PNG's filter-type byte."""
    rows = bytearray()
    # Antialiasing width in icon units, about one pixel.
    edge = 1.5 / size

    for py in range(size):
        rows.append(0)
        for px in range(size):
            # Icon units: -0.5 to 0.5, y up.
            x = (px + 0.5) / size - 0.5
            y = 0.5 - (py + 0.5) / size

            plate = rounded_square(x, y, 0.5, 0.11)
            alpha = coverage(-plate, edge)
            if alpha <= 0.0:
                rows.extend((0, 0, 0, 0))
                continue

            colour = BACKGROUND
            # The trailing streak, a soft accent behind the chevron.
            streak = abs(y) - 0.045 * (1.0 - (x + 0.5))
            colour = blend(
                colour,
                ACCENT,
                0.55 * coverage(-streak, 0.06) * coverage(-(x + 0.05), 0.25),
            )
            # Two chevrons, the second smaller and further back.
            for offset, scale, width in ((0.06, 1.0, 0.075), (-0.28, 0.6, 0.055)):
                distance = chevron_distance((x - offset) / scale, y / scale) * scale
                colour = blend(colour, CHEVRON, coverage(width - distance, edge))

            rows.extend((*colour, round(alpha * 255)))

    return bytes(rows)


def coverage(distance: float, width: float) -> float:
    """A soft step: 1 well inside, 0 well outside, linear across `width`."""
    if width <= 0.0:
        return 1.0 if distance >= 0.0 else 0.0
    return min(max(distance / width + 0.5, 0.0), 1.0)


def png(size: int, raw: bytes) -> bytes:
    def chunk(kind: bytes, payload: bytes) -> bytes:
        return (
            struct.pack(">I", len(payload))
            + kind
            + payload
            + struct.pack(">I", zlib.crc32(kind + payload) & 0xFFFFFFFF)
        )

    header = struct.pack(">IIBBBBB", size, size, 8, 6, 0, 0, 0)
    return (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", header)
        + chunk(b"IDAT", zlib.compress(raw, 9))
        + chunk(b"IEND", b"")
    )


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("out", help="PNG to write")
    parser.add_argument("--size", type=int, default=256, help="edge length in pixels")
    args = parser.parse_args()

    with open(args.out, "wb") as handle:
        handle.write(png(args.size, pixels(args.size)))


if __name__ == "__main__":
    main()
