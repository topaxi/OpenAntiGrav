#!/usr/bin/env python3
"""Reads the `SHO` shader blocks out of Wipeout HD, in the two places they live.

    ps3-sho.py census            # every block in EBOOT.elf, and the hash censuses
    ps3-sho.py block 0x0092fe80  # one block, fully framed
    ps3-sho.py hash fogFactors   # ~crc32 of a name, the form the tables store
    ps3-sho.py names             # the names recovered so far
    ps3-sho.py material <image> <path.rcsmaterial>   # one material's variants
    ps3-sho.py materials <image> [substring]         # the sampler census over many

Every number `docs/ghidra/functions/ps3-hdfury-eu/renderer.md` quotes about the
shader container comes out of this, so the page can be re-derived rather than
trusted. Nothing here is a parser this project ships: the blocks live in the
executable, which no part of the game reads at runtime.

**The block layout is validated rather than asserted.** Each of the three tables
starts exactly where the previous one ends - `params == attributes + 8*count`
and `samplers == params + 12*count` - which holds on all 124 blocks in the
shader run and is what makes the framing a reading rather than a guess. The
class table this project misframed by one field
(`docs/ghidra/functions/ps3-hdfury-eu/vex-classes.md`) is why that check is here.

The ELF must already be extracted; see the ps3-hdfury-eu README.
"""

from __future__ import annotations

import struct
import sys
import zlib
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_ELF = ROOT / "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf"

# The contiguous run of shader blocks. Two further `SHO\x08` words sit in
# `.rodata` among the strings and are not shader blobs - see renderer.md.
RUN_LO, RUN_HI = 0x00927800, 0x00936D80

# Names recovered by hashing candidates against the tables. `~crc32` is the
# hash - `renderer.md`'s "The name hash is CRC-32" - so a name that lands is a
# preimage and not a resemblance. Every one below also agrees in *shape* with
# what the table declares for it, which is the second, independent check: a
# matrix is declared `count 4`, a sampler carries a texture unit, and the three
# attributes named `inPos`, `inCol` and `inUV` sit at attribute slots 0, 3 and
# 8 - position, COLOR0 and TEXCOORD0 in NV40's own conventional numbering.
NAMES = {
    # Parameters
    0x2E7D5F33: "viewProj",
    0x9307F358: "worldView",
    0xADF3C36F: "proj",
    0x4C06F24F: "worldViewProj",
    0xE252323B: "kWorldViewProj",
    0xD04A156C: "fogFactors",
    0x906B67BA: "time",
    0x13B9DA7B: "scale",
    0x083FDB95: "size",
    0xA6F5352F: "offset",
    0x627BE8EA: "offsets",
    0x2F7FC242: "colourRamp",
    # Material-table parameters, recovered while reading the fog microcode -
    # see docs/ghidra/functions/ps3-hdfury-eu/renderer.md and
    # scripts/ps3-microcode.py. fogColour is the float4 whose .rgb the fog
    # lerp targets and whose .w is the curve's coefficient.
    0x3DC31258: "fogColour",
    0x9CC5AB3A: "positionScale",
    0xA4972B78: "positionBias",
    0x515E298E: "diffuse",
    # Attributes
    0xB9D31B0A: "position",
    0xDE7A971B: "normal",
    0x31D3E4B0: "inPos",
    0xA2BBF46C: "inCol",
    0x9F182390: "inUV",
    0x5508B4BA: "uv",
    0x05079A31: "colour",
    0x99A9B716: "color",
    0x47935368: "fAlpha",
    0x3DE66379: "fLength",
    # Samplers
    0x7D99F28D: "texture",
    0xB5AD56C3: "texture0",
    0xC2AA6655: "texture1",
    0x5BA337EF: "texture2",
    0x305C5D3F: "diffuseSampler",
    0x3F1E1460: "blurSampler",
    0xEAB179D1: "depthSampler",
    0x518D0E89: "srcTexture",
    0xF5FAB869: "dstTexture",
    0xA7AC21BC: "sourceImage",
    0xB6D27DA6: "texSampler",
    # Sampler names recovered from the `.rcsmaterial` side, where the same
    # container carries a whole material's shader variants. These are the names
    # that say what a texture slot *is* - see docs/formats/rcsmaterial.md.
    0x3BDC0403: "Texture1",
    0xA2D555B9: "Texture2",
    0xD5D2652F: "Texture3",
    0x37B5DB58: "lightmap",
    0x730DF9EE: "shadowMapTex",
    0x11CB4F74: "DiffuseTexture",
    0x06A77E50: "DiffuseTexture1",
    0xDCAF37AC: "diffuseTexture",
    0x8E6240CE: "diffuseTexture1",
    0x176B1174: "diffuseTexture2",
    0x9EE31012: "Diffuse",
    0x515E298E: "diffuse",
    0x739A786E: "NormalTexture",
    0x62AE87A7: "NormalTexture2",
    0x48F37F5A: "NormalMap",
    0xD9D6922D: "Normal",
    0x20C3E476: "SpecularTexture",
    0x576C4BF3: "SpecMap",
    0x9FC347FF: "Spec",
    0xB1F2A176: "EmissiveTexture",
    0xFA79B1CD: "Emissive",
    0x030FD39B: "emissive",
    0xB1600426: "ReflectionMap",
    0x2D5FC6A6: "EnvMap",
    0x6E465921: "EnvMap1",
    0x76FC220B: "Ramp",
    0x5B44594B: "RampTexture",
    0xC39746D2: "Noise",
    0x62D17F19: "Clouds",
    0xFAD8B460: "smokeTexture",
    0x1C96B9D6: "smokeTexture1",
    0xEEDEE991: "Alpha",
    0x05DFF912: "AlphaMask",
    0xA8262B61: "AlphaTexture",
    0x02AB9F07: "Colour",
    0xCFB83E06: "Colour1",
    0xC7AAB177: "Dirt",
    0x25C5C4D3: "BlendTexture",
}


def name_hash(text: str) -> int:
    """`~crc32(name)`, which is what the tables store."""
    return (~zlib.crc32(text.encode())) & 0xFFFFFFFF


class Image:
    """An ELF64 big-endian image addressed by virtual address."""

    def __init__(self, path: Path) -> None:
        self.raw = path.read_bytes()
        (e_phoff,) = struct.unpack_from(">Q", self.raw, 0x20)
        e_phentsize, e_phnum = struct.unpack_from(">HH", self.raw, 0x36)
        self.segments: list[tuple[int, int, int]] = []
        for i in range(e_phnum):
            base = e_phoff + i * e_phentsize
            p_type, _f, p_offset, p_vaddr, _pa, p_filesz, _m = struct.unpack_from(
                ">IIQQQQQ", self.raw, base
            )
            if p_type == 1 and p_filesz:
                self.segments.append((p_vaddr, p_offset, p_filesz))

    def offset(self, va: int) -> int | None:
        for vaddr, off, size in self.segments:
            if vaddr <= va < vaddr + size:
                return off + (va - vaddr)
        return None

    def address(self, off: int) -> int | None:
        for vaddr, base, size in self.segments:
            if base <= off < base + size:
                return vaddr + (off - base)
        return None

    def u32(self, va: int) -> int:
        return struct.unpack_from(">I", self.raw, self.offset(va))[0]


class Block:
    """One `SHO` block, framed.

    ```text
    +0x00  'SHO', 8
    +0x04  u32   1 for a fragment program, 0 for a vertex one
    +0x08  u16   2 on all 124
    +0x0a  u16   attribute count
    +0x0c  u16   parameter count
    +0x0e  u16   sampler count
    +0x10  u16   attribute table offset, 0x18 on all 124
    +0x12  u16   parameter table offset
    +0x14  u16   sampler table offset
    +0x16  u16   where the program data begins
    ```

    Attribute record, 8 bytes: `(name hash, attribute slot)`.
    Parameter record, 12 bytes:
    `(name hash, u16 type, u16 count, u16 vertex register, u16 fragment slot)`
    with `0xffff` in whichever of the last two does not apply - a vertex
    program's constants live in the constant file and a fragment program's are
    patched into its own microcode, which is why exactly one is ever set.
    Type is `0x0200 | components`, and `count` is rows, so a 4x4 matrix is
    `(0x0204, 4)`.
    Sampler record, 8 bytes: `(name hash, texture unit)`.
    """

    def __init__(self, image: Image, va: int) -> None:
        off = image.offset(va)
        self.va = va
        self.fragment = image.u32(va + 4) == 1
        fields = struct.unpack_from(">8H", image.raw, off + 8)
        self.version = fields[0]
        counts = fields[1], fields[2], fields[3]
        offsets = fields[4], fields[5], fields[6]
        self.program_at = fields[7]
        self.attributes = [
            struct.unpack_from(">II", image.raw, off + offsets[0] + 8 * i)
            for i in range(counts[0])
        ]
        self.parameters = [
            struct.unpack_from(">IHHHH", image.raw, off + offsets[1] + 12 * i)
            for i in range(counts[1])
        ]
        self.samplers = [
            struct.unpack_from(">II", image.raw, off + offsets[2] + 8 * i)
            for i in range(counts[2])
        ]
        # The framing check: each table starts where the previous one ended.
        self.consistent = (
            offsets[1] == offsets[0] + 8 * counts[0]
            and offsets[2] == offsets[1] + 12 * counts[1]
        )

    def show(self) -> None:
        kind = "fragment" if self.fragment else "vertex"
        print(f"{self.va:#010x}  {kind} program, version {self.version}")
        print(f"  program data at +{self.program_at:#05x}")
        for h, slot in self.attributes:
            print(f"  attribute  {h:#010x}  slot {slot:2}  {NAMES.get(h, '')}")
        for h, ty, count, vreg, fslot in self.parameters:
            where = f"c{vreg}" if vreg != 0xFFFF else f"patch +{fslot:#06x}"
            print(
                f"  parameter  {h:#010x}  float{ty & 0xFF} x{count}  "
                f"{where:14} {NAMES.get(h, '')}"
            )
        for h, unit in self.samplers:
            print(f"  sampler    {h:#010x}  unit {unit}    {NAMES.get(h, '')}")


def blocks(image: Image) -> list[Block]:
    """Every block in the shader run, in address order."""
    out = []
    at = 0
    while True:
        at = image.raw.find(b"SHO\x08", at)
        if at < 0:
            break
        va = image.address(at)
        if va is not None and RUN_LO <= va <= RUN_HI:
            out.append(Block(image, va))
        at += 4
    out.sort(key=lambda b: b.va)
    return out


def census(image: Image) -> None:
    found = blocks(image)
    bad = [b.va for b in found if not b.consistent]
    fragment = sum(1 for b in found if b.fragment)
    print(f"{len(found)} blocks in {RUN_LO:#010x}..{RUN_HI:#010x}")
    print(f"  {fragment} fragment, {len(found) - fragment} vertex")
    print(f"  {len(bad)} with a table that does not follow the previous one")
    if bad:
        print("  " + " ".join(f"{va:#010x}" for va in bad))
    # A program with attributes is a vertex program and one without is a
    # fragment program, on all 124 - which is what says `+0x04` is the kind and
    # not something that happens to correlate with it.
    mixed = [b.va for b in found if b.fragment != (not b.attributes)]
    print(f"  {len(mixed)} where the kind word disagrees with having attributes")
    for label, rows in (
        ("attributes", [(h, f"slot {s}") for b in found for h, s in b.attributes]),
        (
            "parameters",
            [
                (h, f"float{ty & 0xFF} x{c}")
                for b in found
                for h, ty, c, _v, _f in b.parameters
            ],
        ),
        ("samplers", [(h, f"unit {u}") for b in found for h, u in b.samplers]),
    ):
        seen: dict[int, tuple[int, set[str]]] = {}
        for h, shape in rows:
            count, shapes = seen.get(h, (0, set()))
            seen[h] = (count + 1, shapes | {shape})
        named = sum(1 for h in seen if h in NAMES)
        print(f"\n{len(seen)} distinct {label}, {named} named:")
        for h, (count, shapes) in sorted(seen.items(), key=lambda kv: -kv[1][0]):
            print(
                f"  {h:#010x} {count:4}  {', '.join(sorted(shapes)):24} "
                f"{NAMES.get(h, '')}"
            )


def material_blocks(data: bytes) -> list[tuple[int, Block]]:
    """Every framed `SHO` block in a `.rcsmaterial`, as `(offset, block)`.

    A `.rcsmaterial` is **not one shader**: it is a container of *variants*, and
    the count is in its own first word - 70 for
    `talons_junction/materials/etched_glass_tech.rcsmaterial`, whose 0x40-byte
    variant records start at the offset its second word gives. Each variant
    names a vertex program and a fragment one, which is why "the second texture
    slot's role" could not be read off a single sampler table: twenty fragment
    programs of one material disagree about which unit a sampler sits at.
    """
    out = []
    at = 0
    while True:
        at = data.find(b"SHO\x08", at)
        if at < 0:
            break
        block = RawBlock(data, at)
        if block.consistent:
            out.append((at, block))
        at += 4
    return out


class RawBlock(Block):
    """A [`Block`] framed against a plain buffer rather than an ELF image."""

    def __init__(self, data: bytes, at: int) -> None:  # noqa: D107
        class _Buffer:
            raw = data

            def offset(self, va: int) -> int:
                return va

            def u32(self, va: int) -> int:
                return struct.unpack_from(">I", data, va)[0]

        Block.__init__(self, _Buffer(), at)


def read_entry(image: str, path: str) -> bytes | None:
    """One archive entry, out of a decrypted PS3 image."""
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from psarc import open_archives  # noqa: PLC0415

    for archive in ("DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06"):
        spec = f"{image}:PS3_GAME/USRDIR/{archive}.PSARC"
        try:
            opened = open_archives(spec)
        except Exception:  # noqa: BLE001 - a missing archive is not an error here
            continue
        for handle in opened:
            index = handle.find(path)
            if index is not None:
                return handle.read(index)
    return None


def show_material(image: str, path: str) -> None:
    data = read_entry(image, path)
    if data is None:
        print(f"{path} is not in any archive on {image}", file=sys.stderr)
        return
    count = struct.unpack_from(">I", data, 0)[0]
    print(f"{path}: {len(data)} bytes, {count} variant(s)")
    blocks = material_blocks(data)
    fragment = [b for _, b in blocks if b.fragment]
    print(f"  {len(blocks)} framed block(s), {len(fragment)} fragment")
    by_unit: dict[int, dict[int, int]] = {}
    for block in fragment:
        for h, unit in block.samplers:
            by_unit.setdefault(unit, {})
            by_unit[unit][h] = by_unit[unit].get(h, 0) + 1
    for unit in sorted(by_unit):
        rows = sorted(by_unit[unit].items(), key=lambda kv: -kv[1])
        shown = ", ".join(
            f"{h:#010x}{'=' + NAMES[h] if h in NAMES else ''} x{n}" for h, n in rows[:4]
        )
        print(f"  unit {unit}: {shown}")


def show_materials(image: str, substring: str = "") -> None:
    """The sampler census over every `.rcsmaterial` on an image.

    This is where the sampler names came from: 125 distinct hashes over 693
    materials, swept against a candidate list, with the shape agreeing - a
    sampler carries a texture unit and no register at all.
    """
    sys.path.insert(0, str(Path(__file__).resolve().parent))
    from psarc import open_archives  # noqa: PLC0415

    seen: dict[int, tuple[int, set[int]]] = {}
    materials = 0
    for archive in ("DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06"):
        spec = f"{image}:PS3_GAME/USRDIR/{archive}.PSARC"
        try:
            opened = list(open_archives(spec))
        except SystemExit:
            continue
        for handle in opened:
            wanted = [
                index
                for path, _size, index in handle.files()
                if path.endswith(".rcsmaterial") and substring in path
            ]
            for index in wanted:
                materials += 1
                for _at, block in material_blocks(handle.read(index)):
                    for h, unit in block.samplers:
                        count, units = seen.get(h, (0, set()))
                        seen[h] = (count + 1, units | {unit})
    named = sum(1 for h in seen if h in NAMES)
    print(f"{materials} .rcsmaterial file(s), {len(seen)} distinct samplers, {named} named")
    for h, (count, units) in sorted(seen.items(), key=lambda kv: -kv[1][0]):
        print(f"  {h:#010x} {count:6}  units {sorted(units)}  {NAMES.get(h, '')}")


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    command = argv[1]
    if command == "material":
        show_material(argv[2], argv[3])
        return 0
    if command == "materials":
        show_materials(argv[2], argv[3] if len(argv) > 3 else "")
        return 0
    if command == "hash":
        for text in argv[2:]:
            print(f"{name_hash(text):#010x}  {text}")
        return 0
    if command == "names":
        for h, text in sorted(NAMES.items(), key=lambda kv: kv[1]):
            print(f"{h:#010x}  {text}")
        return 0
    path = Path(argv[-1]) if argv[-1].endswith(".elf") else DEFAULT_ELF
    if not path.exists():
        print(f"{path} is not present; see data/README.md", file=sys.stderr)
        return 1
    image = Image(path)
    if command == "census":
        census(image)
        return 0
    if command == "block":
        Block(image, int(argv[2], 0)).show()
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
