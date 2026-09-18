#!/usr/bin/env python3
"""Sweep every `SHO` shader block on Wipeout HD/Fury for a named engine
parameter, by its `~crc32` name hash in the block's own parameter table.

    hd-pointlight-sweep.py                              # the three pointLight0* params
    hd-pointlight-sweep.py --param constantAmbientColour:0x81db67ea
    hd-pointlight-sweep.py --image data/images/hdfury-ps3-eu-dec.iso
    hd-pointlight-sweep.py --eboot data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf

Built to answer one question reproducibly: does *any* shipped shader program
- in a `.rcsmaterial` or compiled straight into `EBOOT.elf` - ever reference
a given engine parameter? `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s
per-material microcode sweep answered this by hand for four known materials
and `constantAmbientColour`; this generalises it to every material on the
disc (plus the EBOOT's own resident blocks) and to any parameter, so the next
sweep of one of the 81-entry engine parameter table's rows is a flag, not a
rewrite.

`block_header`'s `counts[1]`/`offsets[1]` parameter table is shared by
fragment and vertex blocks alike (`show_tables` already reads both the same
way) - one entry per declared parameter, `(hash, vreg, fslot)`.
`vreg != 0xffff` means register-bound: a vertex constant, read directly as
`c[vreg - 256]` by the code (fragment programs have no constant registers at
all, so this case is vertex-only). `vreg == 0xffff` means patched: the
fragment program has the parameter's value written straight into its own
code at the slots `fp_patch_slots` follows from `fslot`, which is what
`fp_patch_map` already does and this script reuses.

**A register number alone proves nothing - the name hash is the only sound
check.** An early version of this script tried the opposite: decode every
vertex instruction's `CONST` field and compare the raw register against
`row + 256` for each candidate row, on the (wrong) assumption that an
engine parameter table row's register is the same number in every
material. It
isn't - each material's own parameter table assigns its own registers, and
nothing stops two materials, or a material and the shared per-frame table,
using the same number for different data. Concretely, on
`amphiseum/fe/materials/cf_fetracks.rcsmaterial` block 121: the code reads
`c[19]`, and `19 + 256 = 275` is not the shared table's `pointLight0Position
WorldSpace` there - it is the fourth row of that block's *own* locally
bound `c272` 4x4 matrix (272-275), a coincidence of allocation, not a
point-light read. A second check confirmed the method's failure rather than
one block's: swept for `positionScale`/`positionBias` (rows 210/211, which
renderer.md's own reading says real vertex code reads as `c[210]`/`c[211]`)
the same way, disc-wide - zero hits, on a parameter proven to be read. Two
independent negatives on a parameter known to be positive is the method,
not the data. Keep this history in the docstring: the trap is exactly the
"self-consistent and wrong" shape this project already names in
`vex-classes.md` and `memory.md`, and the next person tempted to sweep by
register number should read this before doing it again.

Every block that fails to parse (header short, patch chain runs off the end)
is counted and reported separately - a parse failure must never look like a
proven negative.
"""

import argparse
import importlib.util
import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))
import psarc  # noqa: E402

_spec = importlib.util.spec_from_file_location("mc", ROOT / "scripts/ps3-microcode.py")
mc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mc)

DEFAULT_IMAGE = "data/images/hdfury-ps3-eu-dec.iso"
DEFAULT_EBOOT = "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf"

DEFAULT_PARAMS = {
    "pointLight0PositionWorldSpace": 0x69A6BA16,
    "pointLight0Colour": 0xEAD721A1,
    "pointLight0Falloff": 0x407290F8,
}


def parse_param(spec: str) -> tuple[str, int]:
    name, _, hexhash = spec.partition(":")
    if not hexhash:
        raise SystemExit(f"--param needs name:0xHASH, got {spec!r}")
    return name, int(hexhash, 16)


def block_hits(
    raw: bytes, at: int, hashes: dict[int, str]
) -> tuple[list[tuple[str, str]], bool]:
    """`([(param name, "register-bound"|"patched"), ...], parsed_ok)` for one block.

    "patched" hits are cross-checked against `fp_patch_slots` so a table
    entry whose patch chain runs off the end (declared, but not actually
    reaching live code) does not count - the same rigor `fp_patch_map`
    already applies for the fragment-side sweep this reuses.
    """
    try:
        counts, offsets, program_at = mc.block_header(raw, at)
    except struct.error:
        return [], False
    hits = []
    try:
        for i in range(counts[1]):
            h, _ty, _count, vreg, fslot = struct.unpack_from(
                ">IHHHH", raw, at + offsets[1] + 12 * i
            )
            if h not in hashes:
                continue
            if vreg != 0xFFFF:
                hits.append((hashes[h], "register-bound"))
            elif mc.fp_patch_slots(raw, at, program_at, fslot):
                hits.append((hashes[h], "patched"))
    except struct.error:
        return hits, False
    return hits, True


def sweep_bytes(raw: bytes, hashes: dict[int, str]):
    hits = []
    parsed = failed = 0
    for fragment in (True, False):
        for bidx, at in mc.blocks_in(raw, fragment):
            block, ok = block_hits(raw, at, hashes)
            if ok:
                parsed += 1
            else:
                failed += 1
            for name, kind in block:
                hits.append((bidx, "fp" if fragment else "vp", name, kind))
    return hits, parsed, failed


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--image", default=DEFAULT_IMAGE)
    ap.add_argument("--eboot", default=DEFAULT_EBOOT)
    ap.add_argument("--no-eboot", action="store_true", help="skip the EBOOT-resident blocks")
    ap.add_argument(
        "--param",
        action="append",
        default=[],
        metavar="name:0xHASH",
        help="add a parameter to sweep for, beyond the three pointLight0* defaults",
    )
    ap.add_argument(
        "--only",
        action="store_true",
        help="sweep only the --param entries given, not the pointLight0* defaults",
    )
    args = ap.parse_args()

    hashes: dict[int, str] = {} if args.only else {h: n for n, h in DEFAULT_PARAMS.items()}
    for spec in args.param:
        name, h = parse_param(spec)
        hashes[h] = name

    mat_parsed = mat_failed = 0
    all_hits: list[tuple[str, str, int, str, str, str]] = []
    materials_seen = 0

    for r in psarc.open_archives(args.image):
        for path, _size, index in r.files():
            if not path.lower().endswith(".rcsmaterial"):
                continue
            materials_seen += 1
            raw = r.read(index)
            hits, parsed, failed = sweep_bytes(raw, hashes)
            mat_parsed += parsed
            mat_failed += failed
            for bidx, side, name, kind in hits:
                all_hits.append((r.label, path, bidx, side, name, kind))

    eboot_parsed = eboot_failed = 0
    if not args.no_eboot and hashes:
        eboot_path = ROOT / args.eboot
        if eboot_path.exists():
            raw = eboot_path.read_bytes()
            hits, eboot_parsed, eboot_failed = sweep_bytes(raw, hashes)
            for bidx, side, name, kind in hits:
                all_hits.append(("EBOOT.elf", "EBOOT.elf", bidx, side, name, kind))
        else:
            print(f"note: {eboot_path} not found, EBOOT-resident blocks not swept")

    print(f"{materials_seen} .rcsmaterial entries scanned (archives may repeat a path)")
    print(f"material blocks: {mat_parsed} parsed, {mat_failed} failed to parse")
    print(f"EBOOT blocks:    {eboot_parsed} parsed, {eboot_failed} failed to parse")
    print(f"total blocks:    {mat_parsed + eboot_parsed} parsed, {mat_failed + eboot_failed} failed")
    print(f"hits: {len(all_hits)}")
    for hit in all_hits:
        print(" ", hit)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
