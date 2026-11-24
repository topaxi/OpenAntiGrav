"""A PNG encoder with no dependencies, so an exported image is the same bytes on every
Blender version (Blender's own `Image.save` output is not stable across them).

Nothing in this module imports `bpy`.
"""

import struct
import zlib

SIGNATURE = b"\x89PNG\r\n\x1a\n"


def is_png(data):
    return data[:8] == SIGNATURE


def _chunk(kind, data):
    return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))


def encode_rgba(width, height, pixels):
    """An 8-bit RGBA PNG of `pixels` (`width * height * 4` bytes, top row first)."""
    if width <= 0 or height <= 0 or len(pixels) != width * height * 4:
        raise ValueError(f"{len(pixels)} bytes do not make a {width} x {height} RGBA image")
    stride = width * 4
    rows = b"".join(b"\x00" + bytes(pixels[y * stride : (y + 1) * stride]) for y in range(height))
    return (
        SIGNATURE
        + _chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0))
        + _chunk(b"IDAT", zlib.compress(rows, 9))
        + _chunk(b"IEND", b"")
    )
