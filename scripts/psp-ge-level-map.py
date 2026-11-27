#!/usr/bin/env python3
"""Show which texture level each pixel of a recorded GE frame samples, in PPSSPP's software renderer.

    uv run --with zstandard --with pillow python3 scripts/psp-ge-level-map.py DUMP OUTDIR NAME \\
        --mode levels|levels-bias0|depth|plain [--depth 1024] [--lane L] [--display 93] [--port 45093]

The frame is a `scripts/psp-ge-dump.py dump` replayed with its words edited (`scripts/psp-ge-replay.py`'s method).
Only the 3D scene is left: PRIMs that are blended, in through mode, or under a projection other than the frame's most
common one are removed (clear-mode PRIMs stay).

    levels        every CLUT is replaced by one colour table and every texel of level N by index N, texture function
                  REPLACE, no fog, no lighting: red 0, green 1, blue 2, yellow 3, magenta 4, grey = a texture with
                  no mip chain. A trilinear fraction reads as the blend of two neighbours. TEXLEVEL as recorded.
    levels-bias0  the same with every TEXLEVEL word set to slope mode, bias 0: the picture of the slope law alone.
                  `--slope S` also rewrites TEXLODSLOPE, which moves the level boundaries to other depths.
    depth         white textures and a linear fog ramp to black: brightness is 255 * (1 - w / DEPTH). The software
                  rasterizer interpolates fog and the slope law's `w` the same way (affine in screen space), so this
                  is the `w` the level was picked from.
    plain         the unedited replay.

The window is a 2x copy of the 480x272 framebuffer; the instance is started with nearest buffer filtering and
no slow-emulator notice, so the saved picture is the framebuffer's own pixels.

The output is derived game data: keep it under data/, never commit it.
"""

import argparse
import collections
import importlib.util
import os
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Level 0..4, then the colour of a texture with no chain.
LEVEL_COLOURS = [(255, 0, 0), (0, 255, 0), (0, 0, 255), (255, 255, 0), (255, 0, 255)]
NO_CHAIN = 15
NO_CHAIN_COLOUR = (64, 64, 64)


def _load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def float24(value):
    return struct.unpack("<I", struct.pack("<f", value))[0] >> 8


def scan(ge, cmds, push):
    """Every PRIM as `(word offset, blended, through, clearing, projection, texture addresses, max level)`."""
    state = collections.defaultdict(int)
    proj = [0.0] * 16
    number = 0
    prims = []
    for kind, size, off in cmds:
        if kind != 1:
            continue
        for i in range(size // 4):
            word = struct.unpack_from("<I", push, off + 4 * i)[0]
            op, val = word >> 24, word & 0xFFFFFF
            if op == 0x3E:
                number = val & 0xF
            elif op == 0x3F:
                if number < 16:
                    proj[number] = ge._f(word)
                number += 1
            else:
                state[op] = val
            if op == 0x04:
                addresses = [((state[0xA8 + n] & 0x0F0000) << 8) | state[0xA0 + n] for n in range(8)]
                prims.append(
                    (
                        off + 4 * i,
                        bool(state[0x21] & 1),
                        bool((state[0x12] >> 23) & 1),
                        bool(state[0xD3] & 1),
                        tuple(proj),
                        addresses,
                        (state[0xC2] >> 16) & 7,
                    )
                )
    return prims


def keep_scene_only(push, prims):
    """NOP every PRIM that is not opaque 3D geometry under the frame's main projection."""
    main = collections.Counter(p[4] for p in prims if not p[2] and p[4][11] == -1.0).most_common(1)[0][0]
    removed = 0
    for offset, blended, through, clearing, proj, _, _ in prims:
        if not clearing and (blended or through or proj != main):
            struct.pack_into("<I", push, offset, 0)
            removed += 1
    return main, removed


def set_state(push, cmds, values):
    """Rewrite every word of the given opcodes, in the initial state block and in the command stream."""
    for kind, size, off in cmds:
        if kind not in (0, 1):
            continue
        for i in range(size // 4):
            op = push[off + 4 * i + 3]
            if op in values:
                struct.pack_into("<I", push, off + 4 * i, (op << 24) | values[op])


def paint_textures(push, cmds, prims, white):
    """Fill each recorded texture level with its level number and each palette with the colour table."""
    chains = {}
    for _, _, _, _, _, addresses, max_level in prims:
        for level in range(max_level + 1):
            chains[addresses[level] & 0x3FFFFFFF] = max_level
    table = {n: (*c, 255) for n, c in enumerate(LEVEL_COLOURS)}
    table[NO_CHAIN] = (*NO_CHAIN_COLOUR, 255)
    for kind, size, off in cmds:
        if kind == 4:  # CLUT, RGBA8888
            for i in range(size // 4):
                colour = (255, 255, 255, 255) if white else table.get(i % 16, (255, 255, 255, 255))
                push[off + 4 * i : off + 4 * i + 4] = bytes(colour)
        elif 16 <= kind < 32:  # a texture level: 16..23 from RAM, 24..31 from VRAM with a 16-byte header
            vram = kind >= 24
            level = kind - (24 if vram else 16)
            start = off + 16 if vram else off
            length = size - 16 if vram else size
            address = struct.unpack_from("<I", push, off)[0] & 0x3FFFFFFF if vram else None
            index = level if chains.get(address, 1) > 0 else NO_CHAIN
            # CLUT4 and CLUT8 both read the right entry of a palette whose entries repeat every 16.
            push[start : start + length] = bytes([index | index << 4]) * length


def edit(ge, data, cmds, push, mode, depth, slope=None):
    out = bytearray(push)
    if mode == "plain":
        return out
    prims = scan(ge, cmds, push)
    main, removed = keep_scene_only(out, prims)
    paint_textures(out, cmds, prims, white=mode == "depth")
    # TEXFUNC replace with alpha; lighting, dither, blend, stencil, colour test and logic op off.
    state = {0xC9: 0x000103, 0x17: 0, 0x20: 0, 0x21: 0, 0x24: 0, 0x27: 0, 0x28: 0}
    if mode == "depth":
        # Fog factor (end + view z) * slope: 1 at the camera, 0 at DEPTH.
        state.update({0x1F: 1, 0xCD: float24(depth), 0xCE: float24(1.0 / depth), 0xCF: 0})
    else:
        state[0x1F] = 0
    if mode == "levels-bias0":
        state[0xC8] = 0x000002
    if slope is not None:
        state[0xD0] = float24(slope)
    set_state(out, cmds, state)
    print(f"projection m00 {main[0]:.6f} m11 {main[5]:.6f}; {removed} of {len(prims)} PRIMs removed", file=sys.stderr)
    return out


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("src")
    ap.add_argument("outdir")
    ap.add_argument("name")
    ap.add_argument("--mode", choices=["levels", "levels-bias0", "depth", "plain"], required=True)
    ap.add_argument("--depth", type=float, default=1024.0)
    ap.add_argument("--slope", type=float)
    ap.add_argument("--lane", default="replay")
    ap.add_argument("--port", type=int, default=45093)
    ap.add_argument("--display", type=int, default=93)
    args = ap.parse_args()
    ge = _load("ge", ROOT / "scripts/psp-ge-dump.py")
    replay = _load("replay", ROOT / "scripts/psp-ge-replay.py")
    src = Path(args.src)
    data = src.read_bytes()
    cmds, push = ge.load(src)
    out = Path(args.outdir).resolve()
    out.mkdir(parents=True, exist_ok=True)
    dump = out / f"{args.name}.ppdmp"
    replay.build(data, edit(ge, data, cmds, push, args.mode, args.depth, args.slope), dump)
    ini = out / "native-pixels.ini"
    ini.write_text("[CPU]\nHideSlowWarnings = True\n[DisplayLayout.Landscape]\nBufferFiltering = 2\n")
    os.environ["PPSSPP_EXTRA_INI"] = str(ini)
    replay.replay_shot(dump, out / f"{args.name}.png", args.lane, args.display, args.port)
    replay.subprocess.run([ROOT / "scripts/ppsspp-start.sh", args.lane, "stop"], stdout=sys.stderr)
    print(out / f"{args.name}.png")


if __name__ == "__main__":
    sys.exit(main())
