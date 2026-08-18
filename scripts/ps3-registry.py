#!/usr/bin/env python3
"""Resolves Wipeout HD's shader registry: which named program is which SHO block.

    scripts/ps3-registry.py            # every resolvable registration, one per line

`ShaderRegistry_Register` (`0x005cd6d8`) is called once per program by static
initialisers, and the call's *third* argument is a pointer straight at the
program's `SHO` block - `Register(slot, name, block)`, established by reading
the call site at `0x003b3938` rather than assumed from the arity. This script
finds every `bl` to it, walks back for the `lwz r4/r5, d(r2)` operand setup,
resolves each displacement against the nearest preceding OPD function's own
TOC (the per-function-TOC trap `memory.md` documents), and prints
`site  name  block-address  first-words`. The first words printing `SHO\\x08`
on every row is the check that the resolution is right.

This is what tied `FunkLayerBloomGate_fp` to `0x0092d580` and made the bloom
chain readable by `scripts/ps3-microcode.py` - see
`docs/ghidra/functions/ps3-hdfury-eu/renderer.md`. The walker only follows the
plain `lwz`-then-`bl` pattern, so registrations built through moves it does
not model come out `None` rather than wrong; 62 of the 121 named programs
resolve, the whole post chain among them.
"""

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
ELF = ROOT / "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf"
REGISTER = 0x005CD6D8

raw = ELF.read_bytes()
count = struct.unpack_from(">H", raw, 0x38)[0]
phoff = struct.unpack_from(">Q", raw, 0x20)[0]
segs = []
for i in range(count):
    p_type, _, p_off, p_va = struct.unpack_from(">IIQQ", raw, phoff + 56 * i)
    p_filesz = struct.unpack_from(">Q", raw, phoff + 56 * i + 32)[0]
    if p_type == 1:
        segs.append((p_va, p_off, p_filesz))


def off_of(va):
    for seg_va, seg_off, size in segs:
        if seg_va <= va < seg_va + size:
            return seg_off + (va - seg_va)
    return None


def u32(va):
    o = off_of(va)
    return struct.unpack_from(">I", raw, o)[0] if o is not None else None


def cstr(va):
    o = off_of(va)
    if o is None:
        return None
    end = raw.index(b"\0", o)
    try:
        return raw[o:end].decode()
    except UnicodeDecodeError:
        return None


# The OPD: function descriptors (entry, toc). Walk the .opd-ish region by
# scanning all data for plausible (entry in text, toc in data) pairs is
# overkill; instead map entry -> toc lazily from the descriptor that names it.
# Cheaper: build entry->toc for every 8-byte pair in the file where both look
# like (code va, data va).
entry_toc = {}
for seg_va, seg_off, size in segs:
    for at in range(seg_off, seg_off + size - 8, 8):
        a, b = struct.unpack_from(">II", raw, at)
        if 0x10000 <= a < 0x700000 and 0x700000 <= b < 0x1000000:
            entry_toc.setdefault(a, b)

# Find every bl to REGISTER.
sites = []
for seg_va, seg_off, size in segs:
    # only executable-ish segment: the first PT_LOAD
    for at in range(seg_off, seg_off + min(size, 0x700000), 4):
        word = struct.unpack_from(">I", raw, at)[0]
        if (word & 0xFC000003) == 0x48000001:  # bl
            target = (seg_va + (at - seg_off) + ((word & 0x03FFFFFC) ^ 0x02000000) - 0x02000000) & 0xFFFFFFFF
            if target == REGISTER:
                sites.append(seg_va + (at - seg_off))
    break

print(f"{len(sites)} call site(s)", file=sys.stderr)

for site in sites:
    o = off_of(site)
    regs = {}
    # walk back up to 32 instructions collecting the last writes to r4/r5
    for back in range(4, 33 * 4, 4):
        word = struct.unpack_from(">I", raw, o - back)[0]
        op = word >> 26
        rt = (word >> 21) & 31
        ra = (word >> 16) & 31
        simm = word & 0xFFFF
        if simm >= 0x8000:
            simm -= 0x10000
        if rt in (4, 5) and rt not in regs:
            if op == 32 and ra == 2:  # lwz rt, d(r2)
                regs[rt] = ("lwz", simm)
            elif op == 14 and ra == 2:  # addi rt, r2, d
                regs[rt] = ("addi", simm)
            elif op in (14, 15, 32, 58):
                regs[rt] = ("other", word)
        if 4 in regs and 5 in regs:
            break
    toc = None
    # TOC: find via any OPD entry whose code precedes the site closest
    best = None
    for entry, t in entry_toc.items():
        if entry <= site and (best is None or entry > best[0]):
            best = (entry, t)
    if best:
        toc = best[1]
    def resolve(reg):
        got = regs.get(reg)
        if not got or toc is None:
            return None
        kind, val = got
        if kind == "lwz":
            return u32(toc + val)
        if kind == "addi":
            return (toc + val) & 0xFFFFFFFF
        return None
    name_ptr = resolve(4)
    payload = resolve(5)
    name = cstr(name_ptr) if name_ptr else None
    words = " ".join(
        f"{u32(payload + 4 * k):08x}" if payload and u32(payload + 4 * k) is not None else "????????"
        for k in range(4)
    ) if payload else ""
    print(f"{site:#010x}  name {name!r:40}  payload {payload and hex(payload)}  [{words}]")
