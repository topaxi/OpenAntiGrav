#!/usr/bin/env python3
"""Disassembles the RSX shader microcode inside Wipeout HD's `SHO` blocks.

    ps3-microcode.py vp 0x0092fe80              # a vertex block in EBOOT.elf
    ps3-microcode.py vp-file mat.rcsmaterial 3  # vertex block 3 of a material
    ps3-microcode.py fp-file mat.rcsmaterial 3  # fragment block 3, likewise
    ps3-microcode.py fp-file mat.rcsmaterial    # every fragment block

Materials are read from a plain file; extract one first with
`just psarc cat <image>:PS3_GAME/USRDIR/DATA00.PSARC <path> > mat.rcsmaterial`.
`scripts/ps3-sho.py` frames the same blocks without decoding their code; this
is the other half, and it is what turned `docs/ghidra/functions/
ps3-hdfury-eu/renderer.md`'s "the curve needs the vertex microcode" into a
read formula. The fog finding this tool exists for:

    every fogged fragment variant of a circuit material computes
    exp(-(fogColour.w * view_depth)^2)  - a MUL by log2(e) into EX2_SAT,
    the product squared and negated - and lerps fogColour.rgb in by it,
    with view_depth the clip-space w its paired vertex program writes.

The instruction encodings are NV40's, taken from Mesa's nouveau driver
headers (`nv40_vertprog.h`, `nvfx_shader.h`, MIT-licensed; the RSX is an
NV4x). Three facts about the *container* were established empirically here
rather than taken from those headers, each by an exhaustive check:

- **Vertex instructions are stored as four big-endian dwords in the spec's
  own order.** All 24 dword permutations were decoded and scored on opcode
  validity, source-type validity and END-bit placement; the identity ordering
  scores 1.00 with exactly one END bit, on the last instruction, and no other
  ordering exceeds 0.67 (`nv40vp.py orderings`, 2026-08-18, block
  0x0092fe80).
- **Fragment instructions store each dword's 16-bit halves swapped.** Read
  plainly, the first opcode of every fragment program is garbage; halfword-
  swapped, block after block opens with TEX/MOV chains and ends where its own
  declared code length says. This is the RSX's documented fragment-microcode
  storage quirk, confirmed here on this data.
- **A vertex program's `c[N]` is the parameter table's register `N + 256`.**
  The table binds `viewProj` to c256 and rows c[0]..c[3] multiply the
  position; `positionScale`/`positionBias` are declared c466/c467 and the
  code reads c[210]/c[211].

Fragment-program constants are inline: a source of register-type CONST takes
its four floats from the 16 bytes after the instruction, which is why
material parameters carry patch offsets instead of registers - the engine
writes values straight into the code. The patch chain is
`param.fslot -> u16 index at block+fslot -> offset list entry ->
(count, [16-byte code slots])`, and following it for `clouds.rcsmaterial`
block 3 is what tied one patched float4 to both the fog coefficient and the
fog colour; `~crc32("fogColour")` is that parameter's name hash, a preimage.
"""

from __future__ import annotations

import struct
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DEFAULT_ELF = ROOT / "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR/EBOOT.elf"

VEC_OPS = {
    0x00: "VNOP", 0x01: "MOV", 0x02: "MUL", 0x03: "ADD", 0x04: "MAD",
    0x05: "DP3", 0x06: "DPH", 0x07: "DP4", 0x08: "DST", 0x09: "MIN",
    0x0A: "MAX", 0x0B: "SLT", 0x0C: "SGE", 0x0D: "ARL", 0x0E: "FRC",
    0x0F: "FLR", 0x10: "SEQ", 0x11: "SFL", 0x12: "SGT", 0x13: "SLE",
    0x14: "SNE", 0x15: "STR", 0x16: "SSG", 0x17: "ARR", 0x18: "ARA",
    0x19: "TXL",
}
SCA_OPS = {
    0x00: "SNOP", 0x01: "SMOV", 0x02: "RCP", 0x03: "RCC", 0x04: "RSQ",
    0x05: "EXP", 0x06: "LOG", 0x07: "LIT", 0x09: "BRA", 0x0B: "CAL",
    0x0C: "RET", 0x0D: "LG2", 0x0E: "EX2", 0x0F: "SIN", 0x10: "COS",
    0x13: "PUSHA", 0x14: "POPA",
}
VP_DESTS = {0: "o[POS]", 1: "o[COL0]", 2: "o[COL1]", 3: "o[BFC0]", 4: "o[BFC1]",
            5: "o[FOGC]", 6: "o[PSZ]"} | {7 + n: f"o[TC{n}]" for n in range(8)}
# Which of the three source slots each vector op reads, as nouveau's own
# emitter fills them: ADD is src0 + src2, MAD all three, one-operand ops src0.
VP_SRC_SLOTS = {
    "MOV": [0], "FRC": [0], "FLR": [0], "ARL": [0], "ARR": [0], "SSG": [0],
    "ADD": [0, 2], "MAD": [0, 1, 2],
}

FP_OPS = {
    0x00: "NOP", 0x01: "MOV", 0x02: "MUL", 0x03: "ADD", 0x04: "MAD",
    0x05: "DP3", 0x06: "DP4", 0x07: "DST", 0x08: "MIN", 0x09: "MAX",
    0x0A: "SLT", 0x0B: "SGE", 0x0C: "SLE", 0x0D: "SGT", 0x0E: "SNE",
    0x0F: "SEQ", 0x10: "FRC", 0x11: "FLR", 0x12: "KIL", 0x13: "PK4B",
    0x14: "UP4B", 0x15: "DDX", 0x16: "DDY", 0x17: "TEX", 0x18: "TXP",
    0x19: "TXD", 0x1A: "RCP", 0x1B: "RSQ", 0x1C: "EX2", 0x1D: "LG2",
    0x1E: "LIT", 0x1F: "LRP", 0x20: "STR", 0x21: "SFL", 0x22: "COS",
    0x23: "SIN", 0x24: "PK2H", 0x25: "UP2H", 0x26: "POW", 0x27: "PK4UB",
    0x28: "UP4UB", 0x29: "PK2US", 0x2A: "UP2US", 0x2E: "DP2A", 0x2F: "TXL",
    0x31: "TXB", 0x36: "RFL", 0x3A: "DIV", 0x3C: "LITEX2", 0x3B: "op3B",
    0x3D: "op3D",
}
FP_INPUTS = {0: "f[POS]", 1: "f[COL0]", 2: "f[COL1]", 3: "f[FOGC]"} | {
    4 + n: f"f[TC{n}]" for n in range(10)
} | {0xE: "f[FACING]"}
FP_SRC_ARITY = {
    "MOV": 1, "FRC": 1, "FLR": 1, "RCP": 1, "RSQ": 1, "EX2": 1, "LG2": 1,
    "COS": 1, "SIN": 1, "DDX": 1, "DDY": 1, "KIL": 0, "TEX": 1, "TXP": 1,
    "TXB": 1, "TXL": 1, "UP4B": 1, "UP2H": 1, "UP4UB": 1, "UP2US": 1,
    "PK4B": 1, "PK2H": 1, "PK4UB": 1, "PK2US": 1, "MAD": 3, "DP2A": 3,
    "TXD": 3, "LRP": 3,
}


def swz(bits: int, shifts: tuple[int, int, int, int]) -> str:
    lanes = "xyzw"
    text = "".join(lanes[(bits >> s) & 3] for s in shifts)
    return "" if text == "xyzw" else "." + text


def mask(bits: int, letters: tuple[tuple[str, int], ...]) -> str:
    if bits == 0xF:
        return ""
    return "." + "".join(c for c, b in letters if bits & b)


class Elf:
    """A big-endian ELF64, addressed by virtual address."""

    def __init__(self, path: Path) -> None:
        self.raw = path.read_bytes()
        count = struct.unpack_from(">H", self.raw, 0x38)[0]
        off = struct.unpack_from(">Q", self.raw, 0x20)[0]
        self.segments = []
        for i in range(count):
            p_type, _, p_off, p_va = struct.unpack_from(">IIQQ", self.raw, off + 56 * i)
            p_filesz = struct.unpack_from(">Q", self.raw, off + 56 * i + 32)[0]
            if p_type == 1:
                self.segments.append((p_va, p_off, p_filesz))

    def offset(self, va: int) -> int:
        for seg_va, seg_off, size in self.segments:
            if seg_va <= va < seg_va + size:
                return seg_off + (va - seg_va)
        raise SystemExit(f"{va:#x} is in no loadable segment")


def block_header(raw: bytes, at: int) -> tuple[tuple[int, int, int], tuple[int, int, int], int]:
    """`(counts, offsets, program_at)` of the SHO block at `at` - the framing
    `scripts/ps3-sho.py` documents and validates."""
    fields = struct.unpack_from(">8H", raw, at + 8)
    return (fields[1], fields[2], fields[3]), (fields[4], fields[5], fields[6]), fields[7]


def show_tables(raw: bytes, at: int) -> None:
    counts, offsets, _ = block_header(raw, at)
    for i in range(counts[0]):
        h, slot = struct.unpack_from(">II", raw, at + offsets[0] + 8 * i)
        print(f"  attribute {h:#010x}  slot {slot}")
    for i in range(counts[1]):
        h, ty, count, vreg, fslot = struct.unpack_from(">IHHHH", raw, at + offsets[1] + 12 * i)
        where = f"c{vreg}" if vreg != 0xFFFF else f"patch fslot {fslot:#x}"
        print(f"  parameter {h:#010x}  float{ty & 0xFF} x{count}  {where}")
    for i in range(counts[2]):
        h, unit = struct.unpack_from(">II", raw, at + offsets[2] + 8 * i)
        print(f"  sampler   {h:#010x}  unit {unit}")


def vp_src(bits17: int, const: int, inp: int) -> str:
    neg = "-" if bits17 & (1 << 16) else ""
    swizzle = swz(bits17 >> 8, (6, 4, 2, 0))
    temp = (bits17 >> 2) & 0x1F
    ty = bits17 & 3
    if ty == 1:
        return f"{neg}R{temp}{swizzle}"
    if ty == 2:
        return f"{neg}v[{inp}]{swizzle}"
    if ty == 3:
        # The parameter table's registers, which are this number + 256.
        return f"{neg}c[{const}]{swizzle}"
    return f"{neg}?{bits17:05x}{swizzle}"


def vp_render(d0: int, d1: int, d2: int, d3: int) -> str:
    vec_op = (d1 >> 22) & 0x1F
    sca_op = (d1 >> 27) & 0x1F
    const = (d1 >> 12) & 0xFF
    inp = (d1 >> 8) & 0x0F
    srcs = [
        ((d1 & 0xFF) << 9) | ((d2 >> 23) & 0x1FF),
        (d2 >> 6) & 0x1FFFF,
        ((d2 & 0x3F) << 11) | ((d3 >> 21) & 0x7FF),
    ]
    dest = (d3 >> 2) & 0x1F
    parts = []
    if vec_op:
        name = VEC_OPS.get(vec_op, hex(vec_op))
        dst = (
            VP_DESTS.get(dest, f"o[{dest}]")
            if d0 & (1 << 30)
            else f"R{(d0 >> 15) & 0x3F}"
        )
        wm = mask((d3 >> 13) & 0xF, (("x", 8), ("y", 4), ("z", 2), ("w", 1)))
        slots = VP_SRC_SLOTS.get(name, [0, 1])
        ops = ", ".join(vp_src(srcs[n], const, inp) for n in slots)
        parts.append(f"{name} {dst}{wm}, {ops}")
    if sca_op:
        name = SCA_OPS.get(sca_op, hex(sca_op))
        dst = (
            VP_DESTS.get(dest, f"o[{dest}]")
            if d3 & (1 << 12)
            else f"R{(d3 >> 7) & 0x1F}"
        )
        wm = mask((d3 >> 17) & 0xF, (("x", 8), ("y", 4), ("z", 2), ("w", 1)))
        parts.append(f"{name} {dst}{wm}, {vp_src(srcs[2], const, inp)}")
    if not parts:
        parts.append("NOP")
    if d3 & 1:
        parts.append("END")
    return " | ".join(parts)


def vp_disasm(raw: bytes, at: int) -> None:
    """One vertex block: tables, then code.

    The program sub-header at `program_at` begins with a u16 instruction
    count; the code itself starts after a defaults section whose size varies,
    so it is found by probing 16-byte alignments until the whole stream
    decodes with valid opcodes - the same validity score that pinned the
    dword order.
    """
    show_tables(raw, at)
    _, _, program_at = block_header(raw, at)
    n_insn = struct.unpack_from(">H", raw, at + program_at)[0]
    base = at + program_at

    def plausible(probe: int) -> bool:
        for k in range(min(n_insn, 8)):
            d0, d1, d2, d3 = struct.unpack_from(">4I", raw, base + probe + 16 * k)
            vec_op = (d1 >> 22) & 0x1F
            sca_op = (d1 >> 27) & 0x1F
            if vec_op not in VEC_OPS or sca_op not in SCA_OPS:
                return False
            if vec_op == 0 and sca_op == 0:
                return False
        return True

    for probe in range(0x10, 0x200, 0x10):
        if plausible(probe):
            print(f"  {n_insn} instruction(s) at +{probe:#x}")
            for k in range(n_insn):
                words = struct.unpack_from(">4I", raw, base + probe + 16 * k)
                print(f"{k:4}  {vp_render(*words)}")
            return
    print("  no clean instruction stream found")


def fp_word(raw: bytes, off: int) -> int:
    """One fragment-program dword: 16-bit halves stored swapped."""
    return (struct.unpack_from(">H", raw, off + 2)[0] << 16) | struct.unpack_from(
        ">H", raw, off
    )[0]


def fp_src(bits: int, const: list[float] | None, input_src: int) -> str:
    ty = bits & 3
    reg = (bits >> 2) & 0x3F
    half = "H" if bits & (1 << 8) else "R"
    swizzle = swz(bits >> 9, (0, 2, 4, 6))
    neg = "-" if bits & (1 << 17) else ""
    if ty == 0:
        return f"{neg}{half}{reg}{swizzle}"
    if ty == 1:
        return f"{neg}{FP_INPUTS.get(input_src, f'f[{input_src}]')}{swizzle}"
    if ty == 2:
        vals = "{" + ", ".join(f"{v:g}" for v in const) + "}" if const else "{?}"
        return f"{neg}{vals}{swizzle}"
    return f"{neg}?{bits:08x}"


def fp_disasm(raw: bytes, at: int) -> None:
    """One fragment block: tables, then code.

    The program sub-header carries the code's byte length at +0x00 and its
    offset at +0x10. An instruction whose source selects an inline constant
    is followed by that constant's 16 bytes, so the stream advances 32 rather
    than 16 there; `@slot` is the 16-byte index a parameter's patch list
    names.
    """
    show_tables(raw, at)
    _, _, program_at = block_header(raw, at)
    code_len = struct.unpack_from(">I", raw, at + program_at)[0]
    code_off = struct.unpack_from(">I", raw, at + program_at + 0x10)[0]
    print(f"  {code_len:#x} byte(s) of code at +{code_off:#x}")
    start = at + program_at + code_off
    pos = start
    while pos < start + code_len:
        d0, d1, d2, d3 = (fp_word(raw, pos + 4 * k) for k in range(4))
        opcode = (d0 >> 24) & 0x3F
        name = FP_OPS.get(opcode, f"op{opcode:#04x}")
        srcs = [d1, d2, d3]
        const = None
        if any((b & 3) == 2 for b in srcs):
            const = [
                struct.unpack(">f", struct.pack(">I", fp_word(raw, pos + 16 + 4 * k)))[0]
                for k in range(4)
            ]
        half = "H" if d0 & (1 << 7) else "R"
        out_reg = (d0 >> 1) & 0x3F
        wm = mask((d0 >> 9) & 0xF, (("x", 1), ("y", 2), ("z", 4), ("w", 8)))
        sat = "_SAT" if d0 & (1 << 31) else ""
        input_src = (d0 >> 13) & 0xF
        arity = FP_SRC_ARITY.get(name, 2)
        ops = ", ".join(fp_src(b, const, input_src) for b in srcs[:arity])
        unit = f" unit{(d0 >> 17) & 0xF}" if name in ("TEX", "TXP", "TXB", "TXL", "TXD") else ""
        slot = (pos - start) // 16
        tail = f" [const@slot {slot + 1:#x}]" if const is not None else ""
        end = " END" if d0 & 1 else ""
        print(f"@{slot:#04x}  {name}{sat} {half}{out_reg}{wm}, {ops}{unit}{tail}{end}")
        pos += 32 if const is not None else 16
        if d0 & 1:
            break


def blocks_in(raw: bytes, fragment: bool):
    at = 0
    index = 0
    while True:
        at = raw.find(b"SHO\x08", at)
        if at < 0:
            return
        if (struct.unpack_from(">I", raw, at + 4)[0] == 1) == fragment:
            yield index, at
            index += 1
        at += 4


def main(argv: list[str]) -> int:
    if len(argv) < 2:
        print(__doc__)
        return 2
    command = argv[1]
    if command == "vp":
        elf = Elf(DEFAULT_ELF if len(argv) < 4 else Path(argv[3]))
        at = elf.offset(int(argv[2], 16))
        vp_disasm(elf.raw, at)
        return 0
    if command == "fp":
        elf = Elf(DEFAULT_ELF if len(argv) < 4 else Path(argv[3]))
        at = elf.offset(int(argv[2], 16))
        fp_disasm(elf.raw, at)
        return 0
    if command in ("vp-file", "fp-file"):
        raw = Path(argv[2]).read_bytes()
        pick = int(argv[3]) if len(argv) > 3 else None
        fragment = command == "fp-file"
        for index, at in blocks_in(raw, fragment):
            if pick is not None and index != pick:
                continue
            kind = "fragment" if fragment else "vertex"
            print(f"{kind} block #{index} at {at:#x}")
            (fp_disasm if fragment else vp_disasm)(raw, at)
        return 0
    print(__doc__)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
