#!/usr/bin/env python3
"""How Wipeout HD's `time`-driven fragment programs combine their result.

    hd_time_shapes.py [image]

`docs/formats/rcsmaterial.md` reads one material's one fragment block and finds
`albedo += diffuseAlpha * (unit1 * tint)` - an **add**, where `mesh.wgsl`
*selects* between its two textures. Whether wiring that is one coherent change
or a classifier problem depends on how many of the ~300 materials declaring
`time` combine the same way, and this is the sweep that answers it. Reading one
block and generalising is exactly the mistake `output_lit_by` made.

Two questions per block, both asked of the decoded instruction stream rather
than of the material's name:

1. **Where does `time` go?** The parameter is patched into an inline constant,
   so the patch chain (`fp_patch_map`) says which 16-byte code slot holds it,
   and the instruction before that slot is the one reading it. `time` reaching
   a register that later addresses a `TEX` is a scroll; `time` reaching a
   colour is something else.
2. **How is the sampled result combined?** An accumulate - a `MAD`/`ADD` whose
   destination is also one of its sources - is the add-shape. A `MUL`, an
   `LRP`, or neither is not.

Both are reported as counts, and the exceptions are named rather than folded
into a percentage.
"""

from __future__ import annotations

import importlib.util
import struct
import sys
from collections import Counter
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

#: `~crc32("time")`, recovered by preimage over `EBOOT.elf`.
TIME = 0x906B67BA

#: The archives holding circuits, craft and billboards.
ARCHIVES = [f"PS3_GAME/USRDIR/DATA0{n}.PSARC" for n in range(7)]

#: The sampling opcodes, by the reference decoder's own names.
SAMPLERS = ("TEX", "TXP", "TXB", "TXL", "TXD")


def load(name: str, path: Path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


mc = load("ps3_microcode", ROOT / "scripts/ps3-microcode.py")
psarc = load("psarc", ROOT / "scripts/psarc.py")


def instructions(raw: bytes, at: int):
    """`(slot, name, dst, srcs, const_slot)` for one fragment block.

    A register is `(ordinal, half)`: `R0` and `H0` are different registers with
    the same ordinal, and conflating them lets a taint cross between them.

    Field extraction mirrors `ps3-microcode.py`'s own `fp_disasm` line for
    line, deliberately - an independent re-derivation here is a second place
    for the encoding to be got wrong, and the first draft of this sweep got two
    of them wrong at once: `TEX` as `0x11` (which is `FLR`) and the patched
    constant at the instruction's own slot rather than the next one. Both
    failed silently, as an empty answer that reads exactly like a real
    negative.
    """
    _counts, _offsets, program_at = mc.block_header(raw, at)
    try:
        code_len = struct.unpack_from(">I", raw, at + program_at)[0]
        code_off = struct.unpack_from(">I", raw, at + program_at + 0x10)[0]
    except struct.error:
        return
    start = at + program_at + code_off
    pos = start
    while pos < start + code_len:
        try:
            d0, d1, d2, d3 = (mc.fp_word(raw, pos + 4 * k) for k in range(4))
        except (struct.error, IndexError):
            return
        name = mc.FP_OPS.get((d0 >> 24) & 0x3F, "?")
        arity = mc.FP_SRC_ARITY.get(name, 2)
        srcs, has_const = [], False
        for bits in [d1, d2, d3][:arity]:
            if bits & 3 == 0:
                srcs.append(((bits >> 2) & 0x3F, bool(bits & (1 << 8))))
            elif bits & 3 == 2:
                has_const = True
        dst = ((d0 >> 1) & 0x3F, bool(d0 & (1 << 7)))
        slot = (pos - start) // 16
        # The constant sits in the *next* 16-byte slot, and that is the one a
        # parameter's patch list names.
        yield slot, name, dst, srcs, (slot + 1 if has_const else None)
        pos += 32 if has_const else 16
        if d0 & 1:
            return


def classify(raw: bytes, at: int) -> tuple[str, str] | None:
    """`(where time goes, how the sample is combined)` for one block."""
    _counts, _offsets, program_at = mc.block_header(raw, at)
    patches = mc.fp_patch_map(raw, at, program_at)
    time_slots = {slot for slot, hashes in patches.items() if TIME in hashes}
    if not time_slots:
        return None

    decoded = list(instructions(raw, at))
    if not decoded:
        return ("undecoded", "undecoded")

    # Which registers the `time` value reaches, forward through the stream.
    tainted = set()
    for _slot, _name, dst, srcs, const_slot in decoded:
        if const_slot in time_slots or any(s in tainted for s in srcs):
            tainted.add(dst)

    scroll = any(
        name in SAMPLERS and any(s in tainted for s in srcs)
        for _slot, name, _dst, srcs, _c in decoded
    )
    where = "texture coordinate" if scroll else "not a coordinate"

    sampled = {
        dst for _slot, name, dst, _srcs, _c in decoded if name in SAMPLERS
    }
    accumulate = any(
        name in ("MAD", "ADD") and dst in srcs and any(s in sampled for s in srcs)
        for _slot, name, dst, srcs, _c in decoded
    )
    interpolated = any(
        name == "LRP" and any(s in sampled for s in srcs)
        for _slot, name, _dst, srcs, _c in decoded
    )
    multiplied = any(
        name == "MUL" and any(s in sampled for s in srcs)
        for _slot, name, _dst, srcs, _c in decoded
    )
    if accumulate:
        how = "accumulate (add-shape)"
    elif interpolated:
        how = "LRP"
    elif multiplied:
        how = "multiply only"
    else:
        how = "neither"
    return (where, how)


def main(argv: list[str]) -> int:
    image = argv[1] if len(argv) > 1 else "data/images/hdfury-ps3-eu-dec.iso"
    where_counts: Counter[str] = Counter()
    how_counts: Counter[str] = Counter()
    materials = 0
    add_shape_materials = 0
    scroll_materials = 0
    exceptions: Counter[str] = Counter()

    seen: set[str] = set()
    for archive in ARCHIVES:
        try:
            readers = psarc.open_archives(f"{image}:{archive}")
        except SystemExit:
            continue
        for reader in readers:
            for path, _size, index in reader.files():
                if not path.endswith(".rcsmaterial") or path in seen:
                    continue
                try:
                    raw = reader.read(index)
                except Exception:
                    continue
                verdicts = [
                    v
                    for _i, at in mc.blocks_in(raw, fragment=True)
                    if (v := classify(raw, at)) is not None
                ]
                if not verdicts:
                    continue
                seen.add(path)
                materials += 1
                for where, how in verdicts:
                    where_counts[where] += 1
                    how_counts[how] += 1
                shapes = {how for _w, how in verdicts}
                if any(w == "texture coordinate" for w, _h in verdicts):
                    scroll_materials += 1
                if shapes == {"accumulate (add-shape)"}:
                    add_shape_materials += 1
                else:
                    name = path.rsplit("/", 1)[-1]
                    exceptions[f"{name}: {sorted(shapes)}"] += 1

    print(f"{materials} materials have a fragment block reading `time`")
    print(f"{scroll_materials} of them take it into a texture coordinate")
    print(f"{add_shape_materials} of them combine every such block by accumulate")
    print("\nwhere `time` goes, per block:")
    for name, n in where_counts.most_common():
        print(f"  {n:>5}  {name}")
    print("\nhow the sampled result is combined, per block:")
    for name, n in how_counts.most_common():
        print(f"  {n:>5}  {name}")
    print(f"\n{len(exceptions)} distinct materials that are not purely add-shape:")
    for line, n in exceptions.most_common(25):
        print(f"  x{n:<4} {line}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
