#!/usr/bin/env python3
"""Reads the PS Vita `GXP` shader containers Wipeout 2048 ships.

    vita-gxp.py census <source> [...]     N of M programs decode, per source
    vita-gxp.py list   <source>           one line per program
    vita-gxp.py dump   <source> <index>   header, region map and parameters
    vita-gxp.py grep   <source> <text>    programs binding a parameter by name

A `<source>` is a loose file scanned for the magic (`eboot.elf`), a `.psarc`
whose every entry is scanned, or `<psarc>:<substring>` to narrow the walk to
entry paths containing that substring. `scripts/ps3-microcode.py` is the PS3
half of this pair and the shape it copies; that one decodes RSX *instructions*,
this one stops at the container - see the "instructions" note at the bottom.

Everything below was derived by sweeping the whole corpus (the patch-v104
`eboot.elf` and every entry of `data.psarc`, `dlc1.psarc` and `dlc2.psarc`),
never from a published header. A candidate field assignment had to survive
every program or it was rejected; the counts are in `docs/formats/gxp.md`.

The container, little-endian throughout:

    +0x00 u32  magic 'GXP\\0'
    +0x04 u8   major version           1 on every program here
    +0x05 u8   minor version           4 on every program here
    +0x06 u16  toolchain version       0x0165 on every program here
    +0x08 u32  size, the whole program
    +0x0c u32  binary guid
    +0x10 u32  source guid
    +0x14 u32  flags; bit 0 set = fragment program, clear = vertex
    +0x20 u32  unread; zero on all but 24 programs, which carry 1 or 0x112
    +0x24 u32  parameter count
    +0x28 u32  parameter table offset, relative to this field
    +0x3c u32  primary instruction count
    +0x40 u32  primary code offset,    relative to this field
    +0x44 u32  secondary instruction count
    +0x48 u32  secondary code offset,  relative to this field
    +0x4c u32  secondary code end,     relative to this field
    +0x58 u32  uniform-image word count (4-byte words)
    +0x70 u32  literal count
    +0x74 u32  literal table offset,   relative to this field
    +0x80 u32  count of the 4-byte records at +0x84
    +0x84 u32  that table's offset,    relative to this field
    +0x88 u32  count of the records at +0x8c (always 0 here)
    +0x8c u32  that table's offset,    relative to this field
    +0x90 u32  container count
    +0x94 u32  container table offset, relative to this field

Fields not listed are left unread rather than guessed. `+0x20` looked like a
constant zero over `eboot.elf` and `data.psarc`'s materials, 44,603 programs,
and is **not**: sweeping the other two archives as well turns up 24 programs
carrying 1 or 0x112 there. It stays unread. That is the whole reason the
census walks every entry of every archive rather than a sample.

A parameter is 16 bytes:

    +0x00 i32  name offset, relative to this field; 0 means unnamed
    +0x04 u16  packed nibbles: category | ? | component_count | container_index
    +0x06 u16  unread
    +0x08 u32  array size
    +0x0c u32  resource index

The second nibble takes 0, 1 and 9 across the corpus and is left unread. The
categories are named from the names they carry, which is as direct as evidence
gets: 0 holds `position`/`normal`/`uv1`, 1 holds `viewProj`/`fogColour`, 2
holds `lightmap`/`DiffuseMap`/`shadowMap`.

**Instructions are not decoded.** The USSE stream is located exactly - both
programs' extents close on their own declared counts, see `Program.regions` -
and left as bytes. Naming its opcodes is a separate piece of work; there is no
half-read disassembly here to mistake for one.
"""

from __future__ import annotations

import os
import struct
import sys
from pathlib import Path

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from psarc import Reader  # noqa: E402

MAGIC = b"GXP\x00"
HEADER_MIN = 0x98
PARAM_STRIDE = 16
CATEGORY = {0: "attribute", 1: "uniform", 2: "sampler", 4: "uniform-buffer"}


class Broken(Exception):
    """A program that does not close. Never repaired, only reported."""


class NotAContainer(Broken):
    """A `GXP\0` in the bytes that is not a program header at all.

    The magic is four bytes and a large enough corpus contains it by accident:
    three `.probes` files hit it, and each declares a "size" that runs past the
    file holding it. Counted apart from a program that fails to decode, because
    conflating the two would let a real regression hide inside a false positive.
    """


def u32(b: bytes, at: int) -> int:
    return struct.unpack_from("<I", b, at)[0]


def rel(b: bytes, at: int) -> int:
    """A self-relative offset: the field's own position plus its value."""
    return at + u32(b, at)


class Parameter:
    __slots__ = ("name", "name_end", "category", "components", "container", "array_size", "resource", "unread")

    def __init__(self, b: bytes, at: int):
        delta = struct.unpack_from("<i", b, at)[0]
        # A zero offset is not a name at position zero, it is *no name*. The
        # only parameters that carry one are the `uniform-buffer` category, and
        # a skinned material's 6,144-element palette buffer is the case in
        # hand: unnamed, while the `skinPalette` uniform that indexes it is
        # named right above it. Read as an offset it points back at the
        # parameter's own entry and the name region stops closing.
        if delta == 0:
            self.name = ""
            self.name_end = None
        else:
            off = at + delta
            if not 0 < off < len(b):
                raise Broken(f"parameter name offset {off:#x} outside the program")
            end = b.find(b"\x00", off)
            if end < 0:
                raise Broken("parameter name is not terminated")
            self.name = b[off:end].decode("ascii", "replace")
            self.name_end = end + 1
        w = struct.unpack_from("<H", b, at + 4)[0]
        self.category = w & 0xF
        self.unread = (w >> 4) & 0xF
        self.components = (w >> 8) & 0xF
        self.container = (w >> 12) & 0xF
        self.array_size = u32(b, at + 8)
        self.resource = u32(b, at + 12)

    def kind(self) -> str:
        return CATEGORY.get(self.category, f"category{self.category}")


class Program:
    """One GXP container, decoded only as far as the evidence reaches."""

    def __init__(self, raw: bytes, at: int, source: str = "", index: int = 0):
        self.source = source
        self.index = index
        self.at = at
        if raw[at : at + 4] != MAGIC:
            raise Broken("no GXP magic")
        size = u32(raw, at + 8)
        if size < HEADER_MIN or at + size > len(raw):
            raise NotAContainer(f"declared size {size:#x} runs past the containing file")
        b = raw[at : at + size]
        self.raw = b
        self.size = size
        self.major = b[4]
        self.minor = b[5]
        self.toolchain = struct.unpack_from("<H", b, 6)[0]
        self.binary_guid = u32(b, 0x0C)
        self.source_guid = u32(b, 0x10)
        self.flags = u32(b, 0x14)
        self.fragment = bool(self.flags & 1)
        self.always_zero = u32(b, 0x20)
        self.param_count = u32(b, 0x24)
        self.param_offset = rel(b, 0x28)
        self.primary_count = u32(b, 0x3C)
        self.primary_offset = rel(b, 0x40)
        self.secondary_count = u32(b, 0x44)
        self.secondary_offset = rel(b, 0x48)
        self.secondary_end = rel(b, 0x4C)
        self.uniform_words = u32(b, 0x58)
        self.literal_count = u32(b, 0x70)
        self.literal_offset = rel(b, 0x74)
        self.t0_count = u32(b, 0x78)
        self.t0_offset = rel(b, 0x7C)
        self.t1_count = u32(b, 0x80)
        self.t1_offset = rel(b, 0x84)
        self.t2_count = u32(b, 0x88)
        self.t2_offset = rel(b, 0x8C)
        self.container_count = u32(b, 0x90)
        self.container_offset = rel(b, 0x94)
        self.parameters = self._parameters()
        self._check()

    def _parameters(self) -> list[Parameter]:
        n, off = self.param_count, self.param_offset
        if n > 4096 or off < HEADER_MIN or off + PARAM_STRIDE * n > self.size:
            raise Broken(f"{n} parameters at {off:#x} do not fit the program")
        return [Parameter(self.raw, off + PARAM_STRIDE * i) for i in range(n)]

    @property
    def primary_end(self) -> int:
        return self.primary_offset + 8 * self.primary_count

    def tables(self) -> list[tuple[int, int, str]]:
        """The six spans between the code and the parameter table.

        Their *order* is not fixed - the table at `+0x7c` follows the
        containers on the 72 skinned programs that have one and precedes them
        (empty) everywhere else - so the closure sorts them rather than
        assuming a layout.
        """
        image = self.literal_offset + 8 * self.literal_count
        return sorted(
            [
                (self.literal_offset, image, "literals"),
                (image, image + 4 * self.uniform_words, "uniform image"),
                (self.t0_offset, self.t0_offset + 8 * self.t0_count, "table@0x7c"),
                (self.t1_offset, self.t1_offset + 4 * self.t1_count, "table@0x84"),
                (self.t2_offset, self.t2_offset, "table@0x8c"),
                (
                    self.container_offset,
                    self.container_offset + 8 * self.container_count,
                    "containers",
                ),
            ]
        )

    def regions(self) -> list[tuple[int, int, str]]:
        """Every byte the header accounts for, in order, as (start, end, what)."""
        code_start = min(self.primary_offset, self.secondary_offset)
        out = [(0, code_start, "header")]
        if self.secondary_count:
            out.append((self.secondary_offset, self.secondary_end, "secondary code"))
        out.append((self.primary_offset, self.primary_end, "primary code"))
        out.extend(self.tables())
        out.append(
            (self.param_offset, self.param_offset + PARAM_STRIDE * self.param_count, "parameters")
        )
        names_at = self.param_offset + PARAM_STRIDE * self.param_count
        out.append((names_at, self.size, "names"))
        return out

    def _check(self) -> None:
        """The closure. Each of these held on every program in the corpus."""
        if self.secondary_offset + 8 * self.secondary_count != self.secondary_end:
            raise Broken("the secondary program's count does not reach its declared end")
        if self.literal_offset != self.primary_end:
            raise Broken("the literal table does not begin where the primary code ends")
        at = self.primary_end
        for start, end, what in self.tables():
            if start != at:
                raise Broken(f"{what} starts at {start:#x}, not at {at:#x}")
            at = end
        if at != self.param_offset:
            raise Broken(f"the tables end at {at:#x}, not at the parameter table {self.param_offset:#x}")
        named = [p.name_end for p in self.parameters if p.name_end is not None]
        if named:
            names_at = self.param_offset + PARAM_STRIDE * self.param_count
            if min(named) < names_at:
                raise Broken("a parameter name overlaps the parameter table")
            if max(named) != self.size:
                raise Broken("the last parameter name does not end at the declared size")
        for (_, end, what), (start, _, nxt) in zip(self.regions(), self.regions()[1:]):
            if end > start:
                raise Broken(f"{what} overlaps {nxt}")

    def unaccounted(self) -> int:
        """Bytes inside the program that no header field claims."""
        covered = 0
        last = 0
        for start, end, _ in self.regions():
            covered += max(0, end - max(start, last))
            last = max(last, end)
        return self.size - covered

    def kind(self) -> str:
        return "fragment" if self.fragment else "vertex"

    def summary(self) -> str:
        return (
            f"{self.at:#010x} {self.size:#7x} {self.kind():8s} "
            f"pri={self.primary_count:4d} sec={self.secondary_count:3d} "
            f"lit={self.literal_count:3d} params={self.param_count:3d}"
        )


def blobs_in(raw: bytes) -> list[int]:
    """Every GXP magic in `raw`, at any offset - the framing is not assumed."""
    out, at = [], 0
    while True:
        at = raw.find(MAGIC, at)
        if at < 0:
            return out
        out.append(at)
        at += 4


def sources(spec: str):
    """Yield (label, bytes) for a loose file or the entries of a `.psarc`."""
    path, _, want = spec.partition(":")
    if not os.path.exists(path):
        raise SystemExit(f"no such file: {path}")
    if not path.lower().endswith(".psarc"):
        yield os.path.basename(path), Path(path).read_bytes()
        return
    with open(path, "rb") as f:
        arc = Reader(f, 0, os.path.basename(path))
        for entry, _size, index in arc.files():
            if want and want.lower() not in entry.lower():
                continue
            yield entry, arc.read(index)


def walk(spec: str):
    """Yield (label, Program | Broken, offset) for every magic in `spec`."""
    for label, raw in sources(spec):
        for i, at in enumerate(blobs_in(raw)):
            try:
                yield label, Program(raw, at, label, i), at
            except Broken as e:
                yield label, e, at


def cmd_census(specs: list[str]) -> int:
    grand = [0, 0, 0, 0]
    for spec in specs:
        files = set()
        found = decoded = residue = rejected = 0
        vert = frag = 0
        params = 0
        for label, p, at in walk(spec):
            files.add(label)
            if isinstance(p, NotAContainer):
                rejected += 1
                print(f"  not a container: {label} @{at:#x}: {p}")
                continue
            found += 1
            if isinstance(p, Broken):
                print(f"  BROKEN {label} @{at:#x}: {p}")
                continue
            decoded += 1
            residue += p.unaccounted()
            params += p.param_count
            if p.fragment:
                frag += 1
            else:
                vert += 1
        print(f"{spec}")
        print(f"  {decoded} of {found} programs decode, over {len(files)} file(s) walked")
        print(f"  {rejected} magic(s) rejected as not a container")
        print(f"  {vert} vertex, {frag} fragment, {params} parameters")
        print(f"  {residue} byte(s) unaccounted by any header field")
        grand[0] += decoded
        grand[1] += found
        grand[2] += len(files)
        grand[3] += residue
    if len(specs) > 1:
        print(f"total: {grand[0]} of {grand[1]} decode over {grand[2]} files, {grand[3]} unaccounted")
    return 0 if grand[0] == grand[1] else 1


def cmd_list(spec: str) -> int:
    for label, p, at in walk(spec):
        if isinstance(p, Broken):
            print(f"{label}\t@{at:#x}\tBROKEN {p}")
            continue
        print(f"{label}\t#{p.index}\t{p.summary()}")
    return 0


def cmd_dump(spec: str, index: int) -> int:
    seen = 0
    for label, p, at in walk(spec):
        if isinstance(p, Broken):
            continue
        if seen != index:
            seen += 1
            continue
        print(f"{label} program #{index} at {at:#x}")
        print(
            f"  version {p.major}.{p.minor} toolchain {p.toolchain:#06x} "
            f"size {p.size:#x} flags {p.flags:#010x} -> {p.kind()}"
        )
        print(f"  guid binary {p.binary_guid:#010x} source {p.source_guid:#010x}")
        print("  regions:")
        for start, end, what in p.regions():
            print(f"    {start:#06x}..{end:#06x}  {end - start:5d}  {what}")
        print(f"    {p.unaccounted()} byte(s) unaccounted")
        print("  parameters:")
        for q in p.parameters:
            print(
                f"    {q.kind():14s} {q.name:40s} comp={q.components} "
                f"array={q.array_size} container={q.container} resource={q.resource}"
            )
        return 0
    print(f"no program #{index} in {spec}")
    return 2


def cmd_grep(spec: str, text: str) -> int:
    want = text.lower()
    for label, p, at in walk(spec):
        if isinstance(p, Broken):
            continue
        hit = [q for q in p.parameters if want in q.name.lower()]
        if not hit:
            continue
        print(f"{label} #{p.index} @{at:#x} {p.kind()} size={p.size:#x}")
        for q in hit:
            print(f"    {q.kind():14s} {q.name} comp={q.components} resource={q.resource}")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) < 3:
        print(__doc__)
        return 2
    cmd = argv[1]
    if cmd == "census":
        return cmd_census(argv[2:])
    if cmd == "list":
        return cmd_list(argv[2])
    if cmd == "dump" and len(argv) > 3:
        return cmd_dump(argv[2], int(argv[3]))
    if cmd == "grep" and len(argv) > 3:
        return cmd_grep(argv[2], argv[3])
    print(__doc__)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
