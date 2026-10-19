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

Each record also carries the texture and target state in force at the PRIM, raw
24-bit payloads: `fbptr` (the target being drawn into), `texsize0`, `texmode`
(max mip level in bits 16..18), `texfilter` (min bits 0..2, mag bits 8..10),
`texwrap` (bit 0 U clamp, bit 8 V clamp, 0 = repeat), `texscale_*`, `blendenable`,
and the vertex UV range `uv_mn`/`uv_mx` (before TEXSCALE). A draw that samples
a render target is one whose `tex` equals an `fbptr` other draws used.

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


TEX_STATE_OPS = {
    0x9C: "fbptr",  # FRAMEBUFPTR: which target the pass draws into
    0x9D: "fbwidth",  # FRAMEBUFWIDTH (high address bits in 16..23)
    0x1E: "texen",  # TEXTUREMAPENABLE
    0xB8: "texsize0",  # TEXSIZE0: width log2 bits 0..3, height log2 bits 8..11
    0xC2: "texmode",  # TEXMODE: bits 16..18 = max mip level
    0xC3: "texformat",
    0xC6: "texfilter",  # min filter bits 0..2, mag filter bits 8..10
    0xC7: "texwrap",  # bit 0 = U clamp, bit 8 = V clamp (0 = repeat)
    0xC0: "texmapmode",
    0xC8: "texlevel",
    0xC9: "texfunc",
    0x48: "texscale_u",
    0x49: "texscale_v",
    0x4A: "texoffset_u",
    0x4B: "texoffset_v",
    0x17: "lighting",  # LIGHTINGENABLE
    0x18: "light0",
    0x19: "light1",
    0x1A: "light2",
    0x1B: "light3",
    0x53: "materialupdate",
    0x55: "materialdiffuse",
    0x21: "blendenable",  # ALPHABLENDENABLE
    0xDF: "blendmode",
    0x1D: "cullenable",  # CULLFACEENABLE
    0x22: "atestenable",  # ALPHATESTENABLE
    0x23: "ztestenable",  # ZTESTENABLE
    0x24: "stenciltestenable",  # STENCILTESTENABLE
    0x9B: "cullmode",  # FACE: 0 clockwise, 1 counter-clockwise
    0xDB: "alphatest",  # func bits 0..2, reference bits 8..15, mask bits 16..23
    0xDC: "stenciltest",  # func bits 0..2, reference bits 8..15, mask bits 16..23
    0xDD: "stencilop",  # fail bits 0..2, zfail 8..10, zpass 16..18
    0xDE: "ztest",  # function, 0 never .. 7 always
    0xE7: "zmask",  # bit 0 set = depth writes off
    0xE8: "pmsk1",  # colour write mask, RGB
    0xE9: "pmsk2",  # colour write mask, alpha
}


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


def _read_uvs(vtype, buf, stride, count):
    """The (u, v) of every vertex, in texels-over-size units, or [] when untextured.

    u8 and u16 coordinates are normalised the way the GE does (/128, /32768);
    floats are read as is. The TEXSCALE/TEXOFFSET in force are not applied.
    """
    tex = vtype & 3
    if not tex:
        return []
    wgt = (vtype >> 9) & 3
    wcount = ((vtype >> 14) & 7) + 1
    size = {1: 1, 2: 2, 3: 4}
    offset = size[wgt] * wcount if wgt else 0
    unit = {1: 1, 2: 2, 3: 4}[tex]
    offset = (offset + unit - 1) // unit * unit
    fmt, div = {1: ("<BB", 128.0), 2: ("<HH", 32768.0), 3: ("<ff", 1.0)}[tex]
    return [
        tuple(c / div for c in struct.unpack_from(fmt, buf, i * stride + offset))
        for i in range(count)
    ]


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
                elif op in TEX_STATE_OPS:
                    state[TEX_STATE_OPS[op]] = word & FLOAT24
                elif op == 0x04 and vertices:
                    vtype = state.get("vtype", 0)
                    stride, pos_offset, fmt = _layout(vtype)
                    if not stride:
                        continue
                    count = len(vertices) // stride
                    uvs = _read_uvs(vtype, vertices, stride, count)
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
                                # Texture and target state in force at the PRIM, raw
                                # 24-bit command payloads (see TEX_STATE_OPS).
                                **{k: state.get(k) for k in TEX_STATE_OPS.values()},
                                "mn": [min(p[k] for p in pts) for k in range(3)],
                                "mx": [max(p[k] for p in pts) for k in range(3)],
                                "diam": diameter(pts),
                                "uv_mn": [min(c) for c in zip(*uvs)] if uvs else None,
                                "uv_mx": [max(c) for c in zip(*uvs)] if uvs else None,
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
