#!/usr/bin/env python3
"""Does another craft deform a Wipeout HD engine trail's ribbon?

The evidence reproducer for
`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, "does another craft
disturb the ribbon". Reads a dump made by

    uv run --with evdev python3 scripts/rpcs3-trail-dump.py <out_dir>

    python3 scripts/hd-trail-wash-check.py <out_dir> [s0|s1]


The discriminator is *spatial correlation*, not an absolute threshold: if a
craft pushes the ribbon aside, the ribbon's departure from what its own ring
predicts must spike at the rings nearest that craft and be flat elsewhere. A
uniform residual is a systematic offset in this reconstruction, not a wash.
"""
import struct, sys
from pathlib import Path

D = Path(sys.argv[1] if len(sys.argv) > 1 else "data/traces/hd-trail-wash")
TAG = sys.argv[2] if len(sys.argv) > 2 else "s0"
f32 = lambda b, o: struct.unpack_from(">f", b, o)[0]
u32 = lambda b, o: struct.unpack_from(">I", b, o)[0]
vec = lambda b, o: tuple(f32(b, o + 4 * i) for i in range(3))
sub = lambda a, b: tuple(a[i] - b[i] for i in range(3))
norm = lambda a: sum(x * x for x in a) ** 0.5

blocks, verts = [], []
for i in range(8):
    blk = (D / f"{TAG}_trail{i}.bin").read_bytes()
    blocks.append(blk)
    live = u32(blk, 0x11D4)
    suffix = "A" if live == u32(blk, 0x1214) else "B"
    verts.append((D / f"{TAG}_vtx{i}{suffix}.bin").read_bytes())

# Layout sanity: every ring sample's colour row must be (1,1,1,1).
bad = sum(1 for b in blocks for s in range(54)
          for c in range(4) if abs(f32(b, s * 0x50 + 0x40 + 4 * c) - 1.0) > 1e-6)
print(f"ring-layout check: {bad} colour components != 1.0 (expect 0)")
print(f"craft-context count at +0x11e8: {[u32(b, 0x11E8) for b in blocks]}")

# Each craft's current nozzle: its own ring's newest sample. The head index is
# whatever alignment makes the ribbon match the ring best, solved per trail.
heads, rings = [], []
for t in range(8):
    blk, v = blocks[t], verts[t]
    mids = []
    for k in range(54):
        a, b = vec(v, (2 * k) * 0x24), vec(v, (2 * k + 1) * 0x24)
        mids.append(tuple((a[i] + b[i]) / 2 for i in range(3)))
    best = min(range(54), key=lambda h: max(
        norm(sub(mids[k], vec(blk, ((h - k) % 54) * 0x50 + 0x30))) for k in range(54)))
    heads.append(best)
    rings.append([vec(blk, ((best - k) % 54) * 0x50 + 0x30) for k in range(54)])

craft = [rings[t][0] for t in range(8)]
print("\ncraft nozzles:")
for t, c in enumerate(craft):
    print(f"  {t}: ({c[0]:9.2f},{c[1]:8.2f},{c[2]:9.2f})")

print("\nper trail: residual against its own ring, split by proximity to any OTHER craft")
for t in range(8):
    v = verts[t]
    rows = []
    for k in range(54):
        a, b = vec(v, (2 * k) * 0x24), vec(v, (2 * k + 1) * 0x24)
        mid = tuple((a[i] + b[i]) / 2 for i in range(3))
        resid = norm(sub(mid, rings[t][k]))
        width = norm(sub(a, b))
        near = min(norm(sub(rings[t][k], craft[o])) for o in range(8) if o != t)
        rows.append((k, resid, width, near))
    close = [r for r in rows if r[3] < 6.0]
    far = [r for r in rows if r[3] >= 20.0]
    def stat(rs, what):
        if not rs:
            return "none"
        return (f"n={len(rs):2d}  max|resid|={max(r[1] for r in rs):.4f}  "
                f"max|w-1|={max(abs(r[2]-1.0) for r in rs):.4f}")
    nearest = min(r[3] for r in rows)
    print(f"  trail {t}: nearest other craft to the ribbon {nearest:6.2f} units")
    print(f"     rings within  6 units of a craft: {stat(close,'close')}")
    print(f"     rings beyond 20 units of a craft: {stat(far,'far')}")
