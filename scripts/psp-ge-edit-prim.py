#!/usr/bin/env python3
"""Move or recolour the vertices of one particle PRIM in a recorded GE frame (`.ppdmp`).

    uv run --with zstandard scripts/psp-ge-edit-prim.py list  DUMP
    uv run --with zstandard scripts/psp-ge-edit-prim.py show  DUMP CI
    uv run --with zstandard scripts/psp-ge-edit-prim.py edit  DUMP CI OUT.ppdmp \\
        [--vertex N=SX,SY]... [--red N,N,...] [--solid]

`list` names every particle-shaped PRIM (vertex type `0x19f` sprite templates, `0x11e` pool strips) with a vertex
the GE has no screen coordinate for, i.e. outside [0, 4096) before the GE offset. `show` prints each vertex of the
PRIM of command chunk `CI`: screen position in pixels from the centre, view depth, in or out. `edit` writes a copy
of the dump for `scripts/psp-ge-replay.py` to draw:

    --vertex N=SX,SY  put vertex N at that screen position, keeping its stored z. A strip's join vertices that repeat
                      N's position move with it. Repeat for more vertices; four equal positions remove a quad.
    --red N,N,...     opaque red on those vertices (`0x11e` only), so a faint particle shows in the replay
    --solid           clear the depth, blend, texture, alpha and stencil tests in force at the PRIM

`--vertex` and `--red` take `s16` and 8888 layouts, so they apply to `0x11e` strips. Output is derived game data:
keep it under data/, never commit it. The method: `docs/ghidra/functions/psp-pulse-usa/particle-system.md`,
"The GE: a triangle with a vertex out of range is not drawn".
"""

import argparse
import importlib.util
import struct
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
VTYPES = (0x19F, 0x11E)
BOUND = 4095.0 + 15.5 / 16.0
STRIP, TRIANGLES = 4, 3
# Z test, blend, texture, alpha test and stencil test enables.
ENABLES = (0x23, 0x21, 0x1E, 0x22, 0x24)


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


ge = _load("ge", ROOT / "scripts/psp-ge-dump.py")
replay = _load("replay", ROOT / "scripts/psp-ge-replay.py")


def apply43(m, p):
    return tuple(m[k] * p[0] + m[3 + k] * p[1] + m[6 + k] * p[2] + m[9 + k] for k in range(3))


def apply44(m, p):
    return tuple(m[k] * p[0] + m[4 + k] * p[1] + m[8 + k] * p[2] + m[12 + k] for k in range(4))


def draws(cmds, push):
    """Every particle-shaped PRIM, with its vertices projected as the GE projects them."""
    matrices = {0x3B: [0.0] * 12, 0x3D: [0.0] * 12, 0x3F: [0.0] * 16}
    number = {0x3B: 0, 0x3D: 0, 0x3F: 0}
    vp, vtype, clamp, chunk = {}, 0, 0, None
    for ci, (kind, size, offset) in enumerate(cmds):
        if kind == 2:
            chunk = (offset, size)
        if kind != 1:
            continue
        for word in struct.unpack_from("<%dI" % (size // 4), push, offset):
            op = word >> 24
            if op in (0x3A, 0x3C, 0x3E):
                number[op + 1] = word & 0xF
            elif op in matrices:
                if number[op] < len(matrices[op]):
                    matrices[op][number[op]] = ge._f(word)
                number[op] += 1
            elif 0x42 <= op <= 0x47:
                vp[op] = ge._f(word)
            elif op == 0x12:
                vtype = word & 0xFFFFFF
            elif op == 0x1C:
                clamp = word & 1
            elif op == 0x04 and chunk:
                if vtype in VTYPES:
                    yield _draw(ci, vtype, (word >> 16) & 7, word & 0xFFFF, chunk[0], push, matrices, vp, clamp)
                chunk = None


def _draw(ci, vtype, prim, count, base, push, matrices, vp, clamp):
    stride, pos_offset, fmt = ge._layout(vtype)
    world, view, proj = list(matrices[0x3B]), list(matrices[0x3D]), list(matrices[0x3F])
    verts = []
    for i in range(count):
        raw = ge._read_pos(push, base + i * stride + pos_offset, fmt)
        eye = apply43(view, apply43(world, raw))
        clip = apply44(proj, eye)
        if clip[3] == 0:
            verts.append({"raw": raw, "depth": 0.0, "screen": (float("inf"), float("inf")), "out": True})
            continue
        sx = clip[0] / clip[3] * vp[0x42] + vp[0x45]
        sy = clip[1] / clip[3] * vp[0x43] + vp[0x46]
        out = not (0 <= sx < BOUND and 0 <= sy < BOUND)
        # With depth clamp on, a vertex inside the near plane is clipped, not range-tested.
        if clamp and not clip[2] > -clip[3]:
            out = False
        verts.append({"raw": raw, "depth": -eye[2], "screen": (sx - 2048, sy - 2048), "out": out})
    return {"ci": ci, "vtype": vtype, "prim": prim, "base": base, "stride": stride, "pos": pos_offset, "fmt": fmt,
            "verts": verts, "matrices": (world, view, proj), "vp": dict(vp)}


def triangles(draw):
    """The PRIM's triangles, a strip's degenerate joins left out."""
    n = len(draw["verts"])
    step = {STRIP: 1, TRIANGLES: 3}.get(draw["prim"])
    if step is None:
        return []
    tris = [(i, i + 1, i + 2) for i in range(0, n - 2, step)]
    return [t for t in tris if len({draw["verts"][i]["raw"] for i in t}) == 3]


def raw_for(draw, vertex, sx, sy):
    """The model-space `(x, y)` that puts `vertex` at screen `(sx, sy)` with its stored `z`."""
    world, view, proj = draw["matrices"]
    vp = draw["vp"]
    ndc = ((sx + 2048 - vp[0x45]) / vp[0x42], (sy + 2048 - vp[0x46]) / vp[0x43])
    z = draw["verts"][vertex]["raw"][2]

    def residual(x, y):
        c = apply44(proj, apply43(view, apply43(world, (x, y, z))))
        return c[0] - ndc[0] * c[3], c[1] - ndc[1] * c[3]

    # Both residuals are affine in (x, y), so three samples solve them exactly.
    r0, rx, ry = residual(0.0, 0.0), residual(1.0, 0.0), residual(0.0, 1.0)
    a, b = rx[0] - r0[0], ry[0] - r0[0]
    c, d = rx[1] - r0[1], ry[1] - r0[1]
    det = a * d - b * c
    return (-r0[0] * d + r0[1] * b) / det, (-r0[1] * a + r0[0] * c) / det, z


def clear_enables(cmds, push, edited, ci):
    """Zero the last word of each enable that runs before the PRIM of chunk `ci`."""
    for op in ENABLES:
        last = None
        for i, (kind, size, offset) in enumerate(cmds[: ci + 1]):
            if kind != 1:
                continue
            for w in range(size // 4):
                word = struct.unpack_from("<I", push, offset + 4 * w)[0]
                if i == ci and word >> 24 == 0x04:
                    break
                if word >> 24 == op:
                    last = offset + 4 * w
        if last is not None:
            struct.pack_into("<I", edited, last, op << 24)


def edit(draw, cmds, push, moves, red, solid):
    edited = bytearray(push)
    base, stride, pos = draw["base"], draw["stride"], draw["pos"]
    if (moves or red) and (draw["fmt"] != 2 or draw["vtype"] != 0x11E):
        raise SystemExit("--vertex and --red apply to 0x11e strips")
    for vertex, (sx, sy) in moves.items():
        old = draw["verts"][vertex]["raw"]
        packed = struct.pack("<hhh", *(round(v * 32768) for v in raw_for(draw, vertex, sx, sy)))
        for i, other in enumerate(draw["verts"]):
            if other["raw"] == old and (i == vertex or i not in moves):
                edited[base + i * stride + pos : base + i * stride + pos + 6] = packed
    for vertex in red:
        edited[base + vertex * stride + 4 : base + vertex * stride + 8] = bytes((255, 0, 0, 255))
    if solid:
        clear_enables(cmds, push, edited, draw["ci"])
    return edited


def vertex_move(text):
    index, at = text.split("=")
    sx, sy = at.split(",")
    return int(index), (float(sx), float(sy))


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("list").add_argument("dump", type=Path)
    for name in ("show", "edit"):
        command = sub.add_parser(name)
        command.add_argument("dump", type=Path)
        command.add_argument("ci", type=int)
    command.add_argument("out", type=Path)
    command.add_argument("--vertex", type=vertex_move, action="append", default=[], metavar="N=SX,SY")
    command.add_argument("--red", type=lambda v: [int(i) for i in v.split(",")], default=[], metavar="N,N")
    command.add_argument("--solid", action="store_true")
    args = parser.parse_args()
    cmds, push = ge.load(args.dump)
    if args.command == "list":
        for draw in draws(cmds, push):
            tris = triangles(draw)
            bad = sum(1 for t in tris if any(draw["verts"][i]["out"] for i in t))
            if not bad:
                continue
            xs = [v["screen"][0] for v in draw["verts"]]
            ys = [v["screen"][1] for v in draw["verts"]]
            depths = [v["depth"] for v in draw["verts"]]
            print(f"ci {draw['ci']} vtype {draw['vtype']:#x} verts {len(draw['verts'])} tris {len(tris)} out {bad} "
                  f"depth {min(depths):.2f}..{max(depths):.2f} sx {min(xs):.0f}..{max(xs):.0f} "
                  f"sy {min(ys):.0f}..{max(ys):.0f}")
        return
    draw = next((d for d in draws(cmds, push) if d["ci"] == args.ci), None)
    if draw is None:
        raise SystemExit(f"no particle-shaped PRIM in chunk {args.ci}")
    if args.command == "show":
        for i, v in enumerate(draw["verts"]):
            print(f"{i:3d} {v['screen'][0]:9.1f} {v['screen'][1]:9.1f} depth {v['depth']:7.2f} "
                  f"{'out' if v['out'] else 'in'}")
        return
    edited = edit(draw, cmds, push, dict(args.vertex), args.red, args.solid)
    args.out.parent.mkdir(parents=True, exist_ok=True)
    replay.build(args.dump.read_bytes(), edited, args.out)


if __name__ == "__main__":
    main()
