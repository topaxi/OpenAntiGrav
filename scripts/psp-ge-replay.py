#!/usr/bin/env python3
"""Replay a recorded GE frame (`.ppdmp`) in PPSSPP with words edited, and keep the picture it draws.

    python3 scripts/psp-ge-replay.py SRC.ppdmp OUTDIR NAME [--nop-prims LO:HI[:VTYPE]] [--nop-after CI]
                                     [--set OP=VAL@LO:HI] [--port 9317] [--display 91]

`scripts/psp-ge-dump.py dump` saves the frame the original drew; `PPSSPPSDL file.ppdmp` plays it back with no game and
no debugger (so no memory reads and no `gpu.buffer.screenshot`; pixels come from `import -window root` on the Xvfb the
instance runs in). Because a replay draws exactly the recorded list, editing a word of it and re-drawing is a bisection
the live game cannot give: a draw can be removed, a state word rewritten, or every draw after one index dropped, and the
frame is otherwise bit-identical. Control: with the ribbons' PRIMs removed, the replay's difference to the unedited
replay reproduces the live A/B of `psp-ribbon-ab.py` (tail box 6.9 against 6.8, nozzle disc 47.6 against 47.7).

Edits are in place, so the command table and every size stay valid; the push buffer is recompressed.

    --nop-prims LO:HI[:VTYPE]  replace the PRIM words of command chunks LO..HI (optionally only vertex type VTYPE, hex) with NOP
    --nop-after CI             NOP every PRIM of chunks CI and later
    --set OP=VAL@LO:HI         rewrite the payload of every word with opcode OP (hex) in chunks LO..HI to VAL (hex)

`census` (`psp-ge-dump.py census`) gives each PRIM's chunk index (`ci`). The picture is the window's 960x544 area scaled
to 480x272; compare two of them with a difference, never as an absolute. Starts and stops its own PPSSPP through
`scripts/ppsspp-start.sh <lane> ...` (lane `replay` by default, `--lane` to change); the debugger port must be free, a
just-stopped instance can hold it for about 20 s. Output is derived game data: keep it under data/, never commit it.
"""

import argparse
import importlib.util
import os
import struct
import subprocess
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent


def _load_ge():
    spec = importlib.util.spec_from_file_location("ge", ROOT / "scripts/psp-ge-dump.py")
    ge = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(ge)
    return ge


def build(data, push2, out):
    """Write `data`'s `.ppdmp` with the push buffer replaced by `push2` (same length)."""
    import zstandard

    table_len = struct.unpack_from("<I", data, 32)[0]
    comp = zstandard.ZstdCompressor(level=3).compress(bytes(push2))
    out.write_bytes(data[: 36 + table_len] + struct.pack("<I", len(comp)) + comp)


def prims(cmds, push, lo, hi, vtype=None):
    """`(chunk, word offset)` of every PRIM in chunks `lo..hi`, optionally of one vertex type."""
    cur = 0
    for ci, (kind, size, off) in enumerate(cmds):
        if kind != 1:
            continue
        for i in range(size // 4):
            w = struct.unpack_from("<I", push, off + 4 * i)[0]
            if w >> 24 == 0x12:
                cur = w & 0xFFFFFF
            if w >> 24 == 4 and lo <= ci <= hi and (vtype is None or cur == vtype):
                yield ci, off + 4 * i


def set_words(cmds, push2, op, val, lo, hi):
    for ci in range(lo, hi + 1):
        kind, size, off = cmds[ci]
        if kind != 1:
            continue
        for i in range(size // 4):
            if push2[off + 4 * i + 3] == op:
                struct.pack_into("<I", push2, off + 4 * i, (op << 24) | val)


def replay_shot(ppdmp, png, lane, display, port):
    from PIL import Image

    start = ROOT / "scripts/ppsspp-start.sh"
    subprocess.run([start, lane, "stop"], stdout=subprocess.DEVNULL)
    time.sleep(1)
    subprocess.run([start, lane, str(display), str(port), str(ppdmp)], stdout=subprocess.DEVNULL, check=True)
    time.sleep(9)
    env = dict(os.environ, DISPLAY=f"127.0.0.1:{display}")
    subprocess.run(["import", "-window", "root", str(png)], env=env, check=True)
    Image.open(png).convert("RGB").crop((160, 128, 1120, 672)).resize((480, 272), Image.BOX).save(png)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("src")
    ap.add_argument("outdir")
    ap.add_argument("name")
    ap.add_argument("--nop-prims", action="append", default=[])
    ap.add_argument("--nop-after", type=int)
    ap.add_argument("--set", action="append", default=[])
    ap.add_argument("--lane", default="replay")
    ap.add_argument("--port", type=int, default=9317)
    ap.add_argument("--display", type=int, default=91)
    a = ap.parse_args()
    ge = _load_ge()
    src = Path(a.src)
    data = src.read_bytes()
    cmds, push = ge.load(src)
    push2 = bytearray(push)
    for spec in a.nop_prims:
        parts = spec.split(":")
        vt = int(parts[2], 16) if len(parts) > 2 else None
        for _, off in prims(cmds, push, int(parts[0]), int(parts[1]), vt):
            struct.pack_into("<I", push2, off, 0)
    if a.nop_after is not None:
        for _, off in prims(cmds, push, a.nop_after, 10**9):
            struct.pack_into("<I", push2, off, 0)
    for spec in a.set:
        head, rng = spec.split("@")
        op, val = head.split("=")
        lo, hi = rng.split(":")
        set_words(cmds, push2, int(op, 16), int(val, 16), int(lo), int(hi))
    out = Path(a.outdir)
    out.mkdir(parents=True, exist_ok=True)
    f = out / f"{a.name}.ppdmp"
    build(data, push2, f)
    replay_shot(f, out / f"{a.name}.png", a.lane, a.display, a.port)
    print(out / f"{a.name}.png")


if __name__ == "__main__":
    sys.exit(main())
