#!/usr/bin/env python3
"""Record one frame of Wipeout Pulse's GE list from PPSSPP and census its draws.

PPSSPP's websocket debugger answers `gpu.record.dump` with a `.ppdmp` of the
frame the GPU is drawing: every GE command word, the vertex and index bytes each
draw reads, and the textures and palettes it binds. Nothing else on this
project's emulator tooling sees *which batches the original submits*: the
original replays compiled call lists, so a breakpoint on `Gu_DrawArray` never
fires per draw. See `docs/reverse-engineering/ppsspp-debugger.md`, "Reading the
frame's GE list".

    uv run --with websocket-client --with zstandard scripts/psp-ge-dump.py \\
        dump --port 45101 --out data/scratch/<lane>/ge/frame.ppdmp
    uv run --with zstandard scripts/psp-ge-dump.py census \\
        data/scratch/<lane>/ge/frame.ppdmp --out frame.prims.json

`census` writes one record per PRIM: its vertex count, the world-space bounding
box of its vertex buffer under the world matrix in force, the longest
vertex-to-vertex distance (`diam`, which a rotation cannot change), the vertex
type and the level-0 texture address. A batch is identified by **(vertex count, sorted
bounding-box extents)** rather than by an address, because the extents survive
the world matrix (the GE's fixed-point positions are scaled by it) and the
vertex count is the batch's own. Against this project's own draws that signature
matched 88 % of the static batches of one frame (135 of 153) and 97 % of a
second (151 of 157); a batch it misses is one the two sides split differently.

The output is **derived game data**: keep it under `data/`, never commit it.

The `.ppdmp` layout (version 6) as read here: a 21-byte header (`PPSSPPGE`, a
u32 version, the game id), three pad bytes, then u32 command-count, u32 push
buffer size, u32 compressed command-table size and a zstd frame of 9-byte
commands `(type u8, size u32, offset u32)`; then a u32 and a zstd frame of the
push buffer those offsets index. Types read: `0` init, `1` GE command words,
`2` the vertex bytes of the next PRIM, `3` its indices, `4` a palette, `24..28`
texture levels 0..4.
"""

import argparse
import base64
import json
import struct
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

FLOAT24 = 0xFFFFFF


def _f(word):
    return struct.unpack("<f", struct.pack("<I", (word & FLOAT24) << 8))[0]


def load(path):
    import zstandard

    data = Path(path).read_bytes()
    if data[:8] != b"PPSSPPGE":
        raise SystemExit(f"{path}: not a PPSSPP GE dump")
    stream = zstandard.ZstdDecompressor().decompressobj()
    table = stream.decompress(data[36:])
    rest = stream.unused_data
    push = zstandard.ZstdDecompressor().decompressobj().decompress(rest[4:])
    cmds = [struct.unpack_from("<BII", table, i * 9) for i in range(len(table) // 9)]
    return cmds, push


def _layout(vtype):
    tex = (vtype >> 0) & 3
    col = (vtype >> 2) & 7
    nrm = (vtype >> 5) & 3
    pos = (vtype >> 7) & 3
    wgt = (vtype >> 9) & 3
    wcount = ((vtype >> 14) & 7) + 1
    size = {0: 0, 1: 1, 2: 2, 3: 4}
    tsz = {0: 0, 1: 2, 2: 4, 3: 8}[tex]
    csz = {0: 0, 4: 2, 5: 2, 6: 2, 7: 4}.get(col, 0)
    nsz = {0: 0, 1: 3, 2: 6, 3: 12}[nrm]
    psz = {0: 0, 1: 3, 2: 6, 3: 12}[pos]

    def align(offset, to):
        return (offset + to - 1) // to * to if to else offset

    offset = size[wgt] * wcount if wgt else 0
    offset = align(offset, max(size[tex], 1)) + tsz
    offset = align(offset, max(csz, 1)) + csz
    offset = align(offset, max(size[nrm], 1)) + nsz
    offset = align(offset, max(size[pos], 1))
    pos_offset = offset
    offset += psz
    stride = align(offset, max(size[pos], size[nrm], size[tex], csz, 1))
    return stride, pos_offset, pos


def _read_pos(buf, at, fmt):
    if fmt == 1:
        return tuple(v / 128.0 for v in struct.unpack_from("<bbb", buf, at))
    if fmt == 2:
        return tuple(v / 32768.0 for v in struct.unpack_from("<hhh", buf, at))
    if fmt == 3:
        return struct.unpack_from("<fff", buf, at)
    return (0.0, 0.0, 0.0)


def diameter(pts):
    """The longest vertex-to-vertex distance: unchanged by a rotation.

    A moving batch's world box depends on the animation phase, this does not,
    so it is what a moving batch is matched on (`docs/rendering/frame-audit.md`
    section 3). Exact for up to 3,000 points, a double sweep (a lower bound,
    usually exact) above that.
    """
    try:
        import numpy as np
    except ImportError:
        return None
    a = np.asarray(pts, dtype=np.float64)
    if len(a) < 2:
        return 0.0
    if len(a) <= 3000:
        d = a[:, None, :] - a[None, :, :]
        return float(np.sqrt((d * d).sum(-1).max()))
    far = a[np.argmax(((a - a[0]) ** 2).sum(-1))]
    far2 = a[np.argmax(((a - far) ** 2).sum(-1))]
    return float(np.sqrt(((far2 - far) ** 2).sum()))


def census(path):
    cmds, push = load(path)
    world = [0.0] * 12
    view = [0.0] * 12
    proj = [0.0] * 16
    number = 0
    view_number = 0
    proj_number = 0
    state = {}
    vertices = None
    prims = []
    for index, (kind, size, offset) in enumerate(cmds):
        if kind == 2:
            vertices = push[offset : offset + size]
        elif kind == 1:
            for word in struct.unpack_from("<%dI" % (size // 4), push, offset):
                op = word >> 24
                if op == 0x3A:
                    number = word & 0xF
                elif op == 0x3B:
                    if number < 12:
                        world[number] = _f(word)
                    number += 1
                elif op == 0x3C:
                    view_number = word & 0xF
                elif op == 0x3D:
                    if view_number < 12:
                        view[view_number] = _f(word)
                    view_number += 1
                elif op == 0x3E:
                    proj_number = word & 0xF
                elif op == 0x3F:
                    if proj_number < 16:
                        proj[proj_number] = _f(word)
                    proj_number += 1
                elif op == 0x12:
                    state["vtype"] = word & FLOAT24
                elif op == 0xA0:
                    state["tex"] = word & FLOAT24
                elif op == 0x04 and vertices:
                    vtype = state.get("vtype", 0)
                    stride, pos_offset, fmt = _layout(vtype)
                    if not stride:
                        continue
                    count = len(vertices) // stride
                    pts = []
                    for i in range(count):
                        x, y, z = _read_pos(vertices, i * stride + pos_offset, fmt)
                        pts.append(
                            tuple(
                                world[k] * x + world[3 + k] * y + world[6 + k] * z + world[9 + k]
                                for k in range(3)
                            )
                        )
                    if pts:
                        prims.append(
                            {
                                "ci": index,
                                "prim": (word >> 16) & 7,
                                "cnt": word & 0xFFFF,
                                "nv": count,
                                "vt": vtype,
                                "tex": state.get("tex", 0),
                                "mn": [min(p[k] for p in pts) for k in range(3)],
                                "mx": [max(p[k] for p in pts) for k in range(3)],
                                "diam": diameter(pts),
                                # The matrices in force, for the clip-space test of
                                # `scripts/pvs-cull-check.py`: world and view are
                                # 4x3 (rows x, y, z, translation), proj 4x4, all
                                # row-major as the game keeps them.
                                "world": list(world),
                                "view": list(view),
                                "proj": list(proj),
                            }
                        )
                    vertices = None
    return prims


def dump(port, out):
    from ppsspp_debugger import Debugger

    with Debugger(port) as d:
        reply = d.call("gpu.record.dump", timeout=120)
    _, b64 = reply["uri"].split(",", 1)
    Path(out).parent.mkdir(parents=True, exist_ok=True)
    Path(out).write_bytes(base64.b64decode(b64))
    print(f"wrote {out}")


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("dump", help="record the next frame from a running PPSSPP")
    p.add_argument("--port", type=int, required=True)
    p.add_argument("--out", required=True)
    p = sub.add_parser("census", help="one record per PRIM, as JSON")
    p.add_argument("file")
    p.add_argument("--out")
    args = parser.parse_args()
    if args.cmd == "dump":
        dump(args.port, args.out)
        return
    prims = census(args.file)
    text = json.dumps(prims)
    if args.out:
        Path(args.out).write_text(text)
        print(f"{len(prims)} prims -> {args.out}")
    else:
        print(text)


if __name__ == "__main__":
    main()
