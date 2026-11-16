#!/usr/bin/env python3
"""Checks `ps3-microcode.py` decodes one known instruction: HD's lightmapped
`MUL H1.x, H6.w, R0.y` (`d0=02000282 d1=1c9dff18 d2=0100aa00`), whose bit 24 of
dword 2 is source 1's saturate precision modifier.

    python3 scripts/ps3-microcode-selftest.py
"""
import contextlib
import importlib.util
import io
import struct
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
spec = importlib.util.spec_from_file_location("mc", HERE / "ps3-microcode.py")
mc = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mc)


def stored(dwords):
    """Fragment dwords as the container stores them: halves swapped, big-endian."""
    out = b""
    for d in dwords:
        out += struct.pack(">HH", d & 0xFFFF, d >> 16)
    return out


def disasm(dwords):
    buf = io.StringIO()
    raw = stored(dwords)
    with contextlib.redirect_stdout(buf):
        mc.fp_code(raw, 0, len(raw), {})
    return buf.getvalue().strip()


mul = disasm([0x02000282, 0x1C9DFF18, 0x0100AA00, 0x0001C800])
print(mul)
assert "sat(" in mul and mul.count("sat(") == 1, mul
assert mul.split(",")[-1].strip().startswith("sat("), mul
plain = disasm([0x02000282, 0x1C9DFF18, 0x0000AA00, 0x0001C800])
assert "sat(" not in plain, plain
print("ok")
