#!/usr/bin/env python3
"""Decoder and lister for the Vita's USSE instruction stream (PowerVR SGX543).

Reads the `GXP\\0` programs that `vita-gxp.py` locates and turns each 64-bit
instruction word into a class plus, for the classes whose operands are
understood, a listing line. Everything else is counted by class and by raw
encoding, never guessed.

    python3 scripts/vita-usse.py disasm <elf|psarc|psarc:entry> <program index> [--blocks]
    python3 scripts/vita-usse.py census <elf|psarc> ...

The instruction layouts and operand rules come from a reading of Vita3K's
USSE translator, re-expressed here in our own tables, names and listing
syntax (`docs/formats/gxp.md`, "The instruction stream"), and were checked
against the disassembly logs that emulator writes and against the corpus.
"""
from __future__ import annotations

import importlib.util
import os
import sys
from collections import Counter

"""Instruction classes, in match order: `(name, mask, value, fields)`.

A word belongs to the first class whose `mask` bits equal `value`: the
short-opcode classes are prefixes of the long ones, so the order is part of
the layout. `fields` lists the fields the listing reads as `(name, lowest bit,
width)`; bits no decoder reads are left out. Field names are ours.
"""
LAYOUTS: list[tuple[str, int, int, tuple]] = [
    ("mad2", 0xf800000000000000, 0x0000000000000000,
     (("half", 58, 1), ("pred", 56, 2), ("s0_swz_hi", 53, 1), ("s0_abs", 50, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("s2_swz", 45, 3), ("s1_swz_hi", 44, 1), ("mask", 39, 4), ("s1_mod", 37, 2), ("s2_mod", 35, 2), ("s0_bank", 34, 1), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 22, 6), ("s1_swz_lo", 20, 2), ("s0_swz_lo", 18, 2), ("s0_num", 12, 6), ("s1_num", 6, 6), ("s2_num", 0, 6))),
    ("nmad32", 0xf800000000000000, 0x0800000000000000,
     (("pred", 56, 3), ("s1_swz_d", 53, 2), ("dst_ext", 51, 1), ("s1_swz_c", 50, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("s2_swz", 44, 4), ("mask", 39, 4), ("s1_mod", 37, 2), ("s2_mod", 36, 1), ("s1_swz_b", 34, 2), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 22, 6), ("s1_swz_a", 15, 7), ("fn", 12, 3), ("s1_num", 6, 6), ("s2_num", 0, 6))),
    ("nmad16", 0xf800000000000000, 0x1000000000000000,
     (("pred", 56, 3), ("s1_swz_d", 53, 2), ("dst_ext", 51, 1), ("s1_swz_c", 50, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("s2_swz", 44, 4), ("mask", 39, 4), ("s1_mod", 37, 2), ("s2_mod", 36, 1), ("s1_swz_b", 34, 2), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 22, 6), ("s1_swz_a", 15, 7), ("fn", 12, 3), ("s1_num", 6, 6), ("s2_num", 0, 6))),
    ("mad", 0xf820000000000000, 0x1820000000000000,
     (("pred", 56, 3), ("g1_swz_ext", 54, 1), ("wide", 52, 1), ("dst_ext", 51, 1), ("s1_ext", 49, 1), ("g0_abs", 46, 1), ("repeat", 44, 2), ("mask", 39, 4), ("s1_neg", 38, 1), ("s1_abs", 37, 1), ("g1_neg", 36, 1), ("g1_abs", 35, 1), ("g0_swz_ext", 34, 1), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("g0_num", 28, 2), ("dst_num", 22, 6), ("g0_swz", 18, 4), ("g1_swz", 14, 4), ("g1_num", 12, 2), ("g0_neg", 11, 1), ("s1_swz_ext", 10, 1), ("s1_swz", 6, 4), ("s1_num", 0, 6))),
    ("dp", 0xf820000000000000, 0x1800000000000000,
     (("pred", 56, 3), ("clip", 54, 1), ("wide", 52, 1), ("dst_ext", 51, 1), ("s1_ext", 49, 1), ("g0_abs", 46, 1), ("repeat", 44, 2), ("mask", 39, 4), ("s1_neg", 38, 1), ("s1_abs", 37, 1), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("g0_num", 28, 2), ("dst_num", 22, 6), ("g0_swz", 18, 4), ("s1_w", 15, 3), ("s1_z", 12, 3), ("s1_y", 9, 3), ("s1_x", 6, 3), ("s1_num", 0, 6))),
    ("dual", 0xf000000000000000, 0x2000000000000000,
     ()),
    ("comp", 0xf800000000000000, 0x3000000000000000,
     (("pred", 56, 3), ("dst_type", 53, 2), ("dst_ext", 51, 1), ("s1_ext", 49, 1), ("repeat", 44, 4), ("fn", 41, 2), ("src_type", 39, 2), ("s1_mod", 37, 2), ("s1_chan", 35, 2), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("dst_num", 21, 7), ("s1_num", 7, 7), ("mask", 0, 4))),
    ("mov", 0xf800000000000000, 0x3800000000000000,
     (("pred", 56, 3), ("cmp_hi", 54, 1), ("s0_follows", 53, 1), ("dst_ext", 51, 1), ("s0_ext", 50, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("kind", 46, 2), ("repeat", 44, 2), ("type", 40, 3), ("cmp_lo", 39, 1), ("swz", 35, 4), ("s0_bank", 34, 1), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("mask", 24, 4), ("dst_num", 18, 6), ("s0_num", 12, 6), ("s1_num", 6, 6), ("s2_num", 0, 6))),
    ("pck", 0xf800000000000000, 0x4000000000000000,
     (("pred", 56, 3), ("dst_ext", 51, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("repeat", 44, 4), ("src_fmt", 41, 3), ("dst_fmt", 38, 3), ("mask", 34, 4), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 21, 7), ("sel3", 19, 2), ("scale", 18, 1), ("sel1", 16, 2), ("sel2", 14, 2), ("s1_num", 8, 6), ("sel0_hi", 7, 1), ("s2_num", 1, 6), ("sel0_lo", 0, 1))),
    ("tst", 0xf800000000000000, 0x4800000000000000,
     (("pred", 56, 3), ("s1_neg", 50, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("prec", 47, 1), ("s2_splat", 46, 1), ("repeat", 44, 2), ("sign", 42, 2), ("zero", 40, 2), ("comb", 39, 1), ("chan", 36, 3), ("pdst", 34, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("family", 18, 2), ("fn", 14, 4), ("s1_num", 7, 7), ("s2_num", 0, 7))),
    ("tstmsk", 0xf800000000000000, 0x7800000000000000,
     ()),
    ("bw", 0xc000000000000000, 0x4000000000000000,
     (("fn", 59, 3), ("pred", 56, 3), ("dst_ext", 51, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("repeat", 44, 4), ("invert", 43, 1), ("rot", 38, 5), ("imm_hi", 36, 2), ("alt", 35, 1), ("half", 34, 1), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 21, 7), ("imm_mid", 14, 7), ("s1_num", 7, 7), ("s2_num", 0, 7))),
    ("sop2", 0xf800000000000000, 0x8000000000000000,
     ()),
    ("sop2m", 0xf800000000000000, 0x9000000000000000,
     ()),
    ("sop3", 0xf800000000000000, 0x8800000000000000,
     ()),
    ("imad8", 0xf800000000000000, 0x9800000000000000,
     ()),
    ("imad16", 0xf800000000000000, 0xa000000000000000,
     (("pred", 57, 2), ("s2_neg", 53, 1), ("s1_hi", 52, 1), ("dst_ext", 51, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("repeat", 44, 3), ("mode", 42, 2), ("s2_fmt", 40, 2), ("s1_fmt", 38, 2), ("s2_hi", 37, 1), ("or_shift", 35, 2), ("s0_bank", 34, 1), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 21, 7), ("s0_num", 14, 7), ("s1_num", 7, 7), ("s2_num", 0, 7))),
    ("imad32", 0xf800833800000000, 0xa800000000000000,
     (("pred", 57, 2), ("s0_hi", 56, 1), ("s1_hi", 53, 1), ("s2_hi", 52, 1), ("dst_ext", 51, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("repeat", 44, 3), ("signed", 43, 1), ("sat", 42, 1), ("s2_type", 38, 2), ("s0_bank", 34, 1), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 21, 7), ("s0_num", 14, 7), ("s1_num", 7, 7), ("s2_num", 0, 7))),
    ("illegal22", 0xf800000000000000, 0xb000000000000000,
     ()),
    ("illegal23", 0xf800000000000000, 0xb800000000000000,
     ()),
    ("illegal24", 0xf800000000000000, 0xc000000000000000,
     ()),
    ("imad8x2", 0xf800000000000000, 0xc800000000000000,
     ()),
    ("imad32x2", 0xf8000c7800000000, 0xd000000000000000,
     (("pred", 56, 3), ("sn", 52, 2), ("dst_ext", 51, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("s0_ext", 47, 1), ("count", 44, 3), ("signed", 41, 1), ("s1_neg", 40, 1), ("s2_neg", 39, 1), ("s0_bank", 34, 1), ("dst_bank", 32, 2), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 21, 7), ("s0_num", 14, 7), ("s1_num", 7, 7), ("s2_num", 0, 7))),
    ("illegal27", 0xf800000000000000, 0xd800000000000000,
     ()),
    ("smp", 0xf800000000000000, 0xe000000000000000,
     (("pred", 56, 3), ("s0_ext", 50, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("fconv", 46, 2), ("dim", 42, 2), ("lod", 40, 2), ("dst_pa", 39, 1), ("mode", 37, 2), ("coord_type", 35, 2), ("s0_bank", 34, 1), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("dst_num", 21, 7), ("s0_num", 14, 7), ("s1_num", 7, 7), ("s2_num", 0, 7))),
    ("phase", 0xff70000000000000, 0xfa40000000000000,
     ()),
    ("nop", 0xf87001c000000000, 0xf800014000000000,
     ()),
    ("branch", 0xf870018000000000, 0xf800000000000000,
     (("pred", 56, 3), ("offset", 0, 20))),
    ("slsi", 0xff30000000000000, 0xfa10000000000000,
     (("lim_t", 44, 4), ("lim_p", 40, 4), ("lim_s", 36, 4), ("dst_mode", 35, 1), ("s0_mode", 34, 1), ("s1_mode", 33, 1), ("s2_mode", 32, 1), ("dst_val", 24, 8), ("s0_val", 16, 8), ("s1_val", 8, 8), ("s2_val", 0, 8))),
    ("sbo", 0xff30000000000000, 0xfb10000000000000,
     ()),
    ("kill", 0xff3ff9fff0000000, 0xf9300006f0000000,
     (("pred", 41, 2),)),
    ("limm", 0xff30000000000000, 0xfc20000000000000,
     ()),
    ("depthf", 0xff30000000000000, 0xfb30000000000000,
     (("s0_ext", 51, 1), ("s1_ext", 49, 1), ("s2_ext", 48, 1), ("pred", 41, 2), ("two_sided", 37, 1), ("s0_bank", 34, 1), ("s1_bank", 30, 2), ("s2_bank", 28, 2), ("s0_num", 14, 7), ("s1_num", 7, 7), ("s2_num", 0, 7))),
    ("spec", 0xf800000000000000, 0xf800000000000000,
     (("special", 54, 1), ("category", 52, 2))),
    ("ldst", 0xe000000000000000, 0xe000000000000000,
     ()),
]

CLASS_NAMES = [row[0] for row in LAYOUTS]


def classify(word: int) -> str | None:
    """The first class whose fixed bits the word carries, or None."""
    for name, mask, value, _ in LAYOUTS:
        if word & mask == value:
            return name
    return None


def fields_of(word: int, cls: str) -> dict[str, int]:
    for name, _mask, _value, fields in LAYOUTS:
        if name == cls:
            return {fname: (word >> lo) & ((1 << n) - 1) for fname, lo, n in fields}
    raise KeyError(cls)


PRED_EXT = ["", "p0 ", "p1 ", "p2 ", "p3 ", "!p0 ", "!p1 ", "pN "]
PRED_VEC = ["", "p0 ", "p1 ", "p2 ", "!p0 ", "!p1 ", "!p2 ", "pN "]
PRED_SHORT = ["", "p0 ", "p1 ", "!p0 "]
CH = "xyzw012H"

TYPES = ["i8", "i16", "i32", "c10", "f16", "f32", "u8", "u16"]


class Reg:
    __slots__ = ("bank", "num", "neg", "abs")

    def __init__(self, bank: str, num: int):
        self.bank, self.num, self.neg, self.abs = bank, num, False, False


def _dest_bank(sel: int, ext: int) -> str:
    return (["sa", "special", "idx", "indexed2"] if ext else ["r", "o", "pa", "indexed1"])[sel]


def _src0_bank(sel: int, ext: int) -> str:
    return (["o", "sa"] if ext else ["r", "pa"])[sel]


def _src12_bank(sel: int, ext: int) -> str:
    return (["indexed1", "special", "imm", "indexed2"] if ext else ["r", "o", "pa", "sa"])[sel]


def _finish(reg: Reg, double: bool, bits: int, secondary: bool) -> Reg:
    """Registers 4 (8 doubled) from the top of the temp file are the
    internal ones; special-bank numbers name constants or globals; in a
    secondary program the per-draw banks are all `sa`."""
    if double and reg.bank not in ("special", "imm"):
        reg.num = (reg.num << 1) & 0xFF
    n_temps = 8 if double else 4
    limit = (1 << bits) - n_temps
    if reg.bank == "r" and reg.num >= limit:
        reg.num -= limit
        if double:
            reg.num >>= 1
        reg.bank = "i"
    if reg.bank == "special":
        if reg.num & 0x40:
            reg.num &= ~0x40
            reg.bank = "g"
        else:
            reg.bank = "c"
    if secondary and reg.bank not in ("i", "c", "imm"):
        reg.bank = "sa"
    return reg


def dest_reg(n, sel, ext, double, bits, secondary):
    return _finish(Reg(_dest_bank(sel, ext), n), double, bits, secondary)


def src0_reg(n, sel, ext, double, bits, secondary):
    return _finish(Reg(_src0_bank(sel, ext), n), double, bits, secondary)


def src12_reg(n, sel, ext, double, bits, secondary):
    return _finish(Reg(_src12_bank(sel, ext), n), double, bits, secondary)


def reg_text(r: Reg, swiz: str | None, offset: int = 0) -> str:
    if r.bank == "imm":
        s = str(r.num)
        return s
    if r.bank.startswith("indexed"):
        which = ("r", "o", "pa", "sa")[(r.num >> 5) & 3]
        s = f"{which}[idx{r.bank[-1]}*2+{r.num & 31}]"
    else:
        s = f"{r.bank}{r.num + offset}"
    if r.neg:
        s = "-" + s
    if r.abs:
        s = f"abs({s})"
    if swiz:
        s += "." + swiz
    return s


def swiz_text(chans: list[int | str], mask: int) -> str:
    out = ""
    for i in range(4):
        if mask & (1 << i):
            c = chans[i]
            out += c if isinstance(c, str) else CH[c]
        else:
            out += "-"
    return out.rstrip("-")


def chans(s: str) -> list[str]:
    return list(s)


SW4_STD = ["xxxx", "yyyy", "zzzz", "wwww", "xyzw", "yzww", "xyzz", "xxyz",
           "xyxy", "xywz", "zxyw", "zwzw", "yzxz", "xxyy", "xzww", "xyz1"]
SW4_EXT = ["yzxw", "zwxy", "xzwy", "yyww", "wyzw", "wzwz", "xyzx", "zzww",
           "xwzx", "yyyx", "yyyz", "xzyw", "xxxy", "zyxw", "yyzz", "zzzy"]
SW3_STD = ["xxx", "yyy", "zzz", "www", "xyz", "yzw", "xxy", "xyx",
           "yyx", "yyz", "zxy", "xzy", "yzx", "zyx", "zzy", "xy1"]
SW3_EXT = ["xyy", "yxy", "xxz", "yxx", "xy0", "x10", "000", "111", "HHH", "222", "x00"]
SW_SCALAR = ["xxxx", "yyyy", "zzzz", "wwww", "0000", "1111", "2222", "HHHH"]

MAD2_S0 = ["xxxx", "yyyy", "zzzz", "wwww", "xyzw", "yzxw", "xyww", "zwxy"]
MAD2_S1 = ["xxxx", "yyyy", "zzzz", "wwww", "xyzw", "xyyz", "yyww", "wyzw"]
MAD2_S2 = ["xxxx", "yyyy", "zzzz", "wwww", "xyzw", "xzww", "xxyz", "xyzz"]


def vec_swizzle(idx: int, ext: int, kind: int) -> str | None:
    """The shared 4-bit swizzle tables: kind 0 scalar, 1 vec3, 2 vec4."""
    if kind == 0:
        return None if ext or idx >= len(SW_SCALAR) else SW_SCALAR[idx]
    if kind == 1:
        t = SW3_EXT if ext else SW3_STD
        return t[idx] + "x" if idx < len(t) else None
    t = SW4_EXT if ext else SW4_STD
    return t[idx]


REGISTER_BANKS = ("pa", "sa", "o", "i", "r")


def write_mask(bank: str, mask: int, f16: bool) -> int:
    """The mask as the register file sees it. Internal registers and f16
    moves to non-register banks keep the raw mask; f16 to a register bank
    works in halves (bit 0 selects the pair x y, bit 2 the pair z w); f32
    has only two lanes."""
    if bank == "i" or (f16 and bank not in REGISTER_BANKS):
        return mask
    if f16:
        return (0b0011 if mask & 0b0001 else 0) | (0b1100 if mask & 0b0100 else 0)
    return mask & 0b0011


def mod_flags(r: Reg, m: int) -> Reg:
    if m in (1, 3):
        r.neg = True
    if m in (2, 3):
        r.abs = True
    return r


class Instr:
    """One decoded word: `cls`, a listing `text` (None when only the class is
    understood), and `parts` for tests (op, type, operand strings)."""

    def __init__(self, cls, text=None, op=None, ty=None, ops=(), pred="", rpt=0, note=""):
        self.cls, self.text, self.op, self.ty, self.ops = cls, text, op, ty, list(ops)
        self.pred, self.rpt, self.note = pred, rpt, note


def _line(pred, op, ty, ops, rpt=0, note=""):
    head = f"{pred}{op}" + (f".{ty}" if ty else "")
    if rpt:
        head += f" rpt{rpt}"
    s = head + (" " + ", ".join(ops) if ops else "")
    return s + (f"  ; {note}" if note else "")


def _mk(cls, pred, op, ty, ops, rpt=0, note=""):
    return Instr(cls, _line(pred, op, ty, ops, rpt, note), op, ty, ops, pred, rpt, note)


def _vec_op_name(op2: int, is32: bool) -> str:
    return ["mul", "add", "frc", "ddx", "ddy", "min", "max", "dot"][op2]


def decode(word: int, secondary: bool = False) -> Instr:
    cls = classify(word)
    if cls is None:
        return Instr("unknown", f".word {word:016x}")
    f = fields_of(word, cls)
    fn = _DECODERS.get(cls)
    if fn is None:
        return Instr(cls, None)
    try:
        r = fn(f, word, secondary)
    except (IndexError, KeyError):
        return Instr(cls, None, note="field out of table")
    return r if r is not None else Instr(cls, None)


def _d_mad2(f, w, sec):
    f16 = bool(f["half"])
    dest = dest_reg(f["dst_num"], f["dst_bank"], 0, True, 7, sec)
    s0 = src0_reg(f["s0_num"], f["s0_bank"], 0, True, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], True, 7, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], True, 7, sec)
    mask = write_mask(dest.bank, f["mask"], f16)
    s0.abs = bool(f["s0_abs"])
    mod_flags(s1, f["s1_mod"])
    mod_flags(s2, f["s2_mod"])
    sw0 = MAD2_S0[f["s0_swz_lo"] | (f["s0_swz_hi"] << 2)]
    sw1 = MAD2_S1[f["s1_swz_lo"] | (f["s1_swz_hi"] << 2)]
    sw2 = MAD2_S2[f["s2_swz"]]
    ops = [reg_text(dest, swiz_text(chans("xyzw"), mask)),
           reg_text(s0, swiz_text(chans(sw0), mask)),
           reg_text(s1, swiz_text(chans(sw1), mask)),
           reg_text(s2, swiz_text(chans(sw2), mask))]
    return _mk("mad2", PRED_SHORT[f["pred"]], "mad", "f16" if f16 else "f32", ops)


def _d_nmad(f, w, sec):
    is32 = bool((w >> 59) & 1)
    op = _vec_op_name(f["fn"], is32)
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], True, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], True, 7, sec)
    mod_flags(s1, f["s1_mod"])
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], True, 7, sec)
    if f["s2_mod"] == 1:
        s2.abs = True
    ty = "f32" if is32 else "f16"
    mask = f["mask"]
    enc = f["s1_swz_a"] | (f["s1_swz_b"] << 7) | (f["s1_swz_c"] << 9) | (f["s1_swz_d"] << 10)
    sw1 = [(enc >> (3 * k)) & 7 for k in range(4)]
    sw2 = SW4_STD[f["s2_swz"]]
    smask = 15 if op == "dot" else mask
    ops = [reg_text(dest, swiz_text(chans("xyzw"), mask)),
           reg_text(s1, swiz_text(sw1, smask)),
           reg_text(s2, swiz_text(chans(sw2), smask))]
    return _mk("nmad", PRED_VEC[f["pred"]], op, ty, ops)


def _gpi(n: int, neg: int, ab: int, swiz: str | None) -> str:
    r = Reg("i", n)
    r.neg, r.abs = bool(neg), bool(ab)
    return reg_text(r, swiz)


def _d_mad(f, w, sec):
    kind = 1 if f["wide"] == 0 else 2
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], True, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], True, 7, sec)
    s1.neg, s1.abs = bool(f["s1_neg"]), bool(f["s1_abs"])
    mask = f["mask"]
    sw1 = vec_swizzle(f["s1_swz"], f["s1_swz_ext"], kind)
    sw0 = vec_swizzle(f["g0_swz"], f["g0_swz_ext"], kind)
    swg1 = vec_swizzle(f["g1_swz"], f["g1_swz_ext"], kind)
    if None in (sw1, sw0, swg1):
        return None
    dsw = "xyz" if kind == 1 else "xyzw"
    ops = [reg_text(dest, swiz_text(chans(dsw + ("x" if kind == 1 else "")), mask)),
           _gpi(f["g0_num"], f["g0_neg"], f["g0_abs"], swiz_text(chans(sw0), mask)),
           reg_text(s1, swiz_text(chans(sw1), mask)),
           _gpi(f["g1_num"], f["g1_neg"], f["g1_abs"], swiz_text(chans(swg1), mask))]
    return _mk("mad", PRED_VEC[f["pred"]], "mad", "f32", ops, rpt=f["repeat"], note="vec3" if kind == 1 else "")


def _d_mov(f, w, sec):
    ty = TYPES[f["type"]]
    dbl = ty in ("c10", "f16", "f32")
    bits = 7 if dbl else 6
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], dbl, bits, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], dbl, bits, sec)
    mask = f["mask"]
    if ty in ("f16", "f32"):
        mask = write_mask(dest.bank, mask, ty == "f16")
    elif not dbl:
        mask = 1
    sw = SW4_STD[f["swz"]] if dbl else "xyzw"
    mt = f["kind"]
    if mt == 3:
        return None
    ops_d = reg_text(dest, swiz_text(chans("xyzw"), mask))
    ops_s1 = reg_text(s1, swiz_text(chans(sw), mask))
    if mt == 0:
        return _mk("mov", PRED_EXT[f["pred"]], "mov", ty, [ops_d, ops_s1], rpt=f["repeat"])
    s0 = src0_reg(f["s0_num"], f["s0_bank"], f["s0_ext"], dbl, bits, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], dbl, bits, sec)
    cmp = ["==", "!=", "<", "<="][(f["cmp_hi"] << 1) | f["cmp_lo"]]
    sw0 = sw if f["s0_follows"] else "xyzw"
    ops = [ops_d, reg_text(s0, swiz_text(chans(sw0), mask)), cmp + "0",
           ops_s1, reg_text(s2, swiz_text(chans(sw), mask))]
    return _mk("mov", PRED_EXT[f["pred"]], "movc" if mt == 1 else "movcu8", ty, ops, rpt=f["repeat"])


PCK_TYPES = ["u8", "i8", "o8", "u16", "i16", "f16", "f32", "c10"]


def _d_pck(f, w, sec):
    sf, df = PCK_TYPES[f["src_fmt"]], PCK_TYPES[f["dst_fmt"]]
    valid = {"u8": 7, "i8": 7, "o8": 7, "u16": 7, "i16": 7, "f16": 15, "f32": 15, "c10": 0}
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], False, 7, sec)
    isfloat = sf in ("f16", "f32")
    s1n = f["s1_num"] if isfloat else (f["s1_num"] << 1) | f["sel0_hi"]
    s1 = src12_reg(s1n, f["s1_bank"], f["s1_ext"], isfloat, 7, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], True, 7, sec)
    c0 = f["sel0_lo"]
    c0 |= (f["sel0_hi"] & 1) << 1 if sf == "f32" else (f["s2_num"] & 1) << 1
    sel = [c0, f["sel1"], f["sel2"], f["sel3"]]
    mask = f["mask"]
    dfloat = df in ("f16", "f32")
    keep_selectors = (isfloat and dfloat) or (f["scale"] and (sf == "u8" or df == "u8"))
    out = list(sel)
    if not keep_selectors:
        # The selectors address the source in order, one per enabled
        # destination lane; 8-bit packs reuse the first two across four lanes.
        enabled = [i for i in range(4) if mask >> i & 1]
        two_lane = (sf == "u8" or df == "u8") and mask == 0b1111
        out = [0, 0, 0, 0]
        for rank, lane in enumerate(enabled):
            out[lane] = sel[lane % 2] if two_lane else sel[rank]
    ops = [reg_text(dest, swiz_text(chans("xyzw"), mask)), reg_text(s1, swiz_text(out, mask))]
    if sf == "f32" and s2.bank != "imm":
        same = s1.num == s2.num and s1.bank == s2.bank
        contig = mask in (1, 3, 7, 15)
        if not (out == [0, 1, 2, 3] and same and contig):
            ops.append(reg_text(s2, "xyzw"))
    note = "scale" if f["scale"] else ""
    return _mk("pck", PRED_EXT[f["pred"]], f"pck.{df}.{sf}", None, ops, rpt=f["repeat"], note=note)


def rotate_left(value: int, amount: int, width: int) -> int:
    value &= (1 << width) - 1
    return ((value << amount) | (value >> (width - amount))) & ((1 << width) - 1) if amount else value


def _d_bw(f, w, sec):
    names = {2: ("and", "or"), 3: ("xor", "xor"), 4: ("shl", "rol"), 5: ("shr", "asr")}
    if f["fn"] not in names:
        return None
    op = names[f["fn"]][f["alt"]]
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], False, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], False, 7, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], False, 7, sec)
    ty = "u16" if f["half"] else "u32"
    if s2.bank == "imm":
        value = f["s2_num"] | (f["imm_mid"] << 7) | (f["imm_hi"] << 14)
        width = 16 if ty == "u16" else 32
        value = rotate_left(value, f["rot"] % width, width)
        if f["invert"]:
            value = ~value & ((1 << width) - 1)
        s2txt = f"#{value:#x}"
    else:
        s2txt = reg_text(s2, None)
        if f["rot"] or f["invert"]:
            s2txt += f" rot{f['rot']}" + (" inv" if f["invert"] else "")
    return _mk("bw", PRED_EXT[f["pred"]], op, ty, [reg_text(dest, None), reg_text(s1, None), s2txt], rpt=f["repeat"])


def _d_imad16(f, w, sec):
    signed = f["mode"] >= 2
    ty = "i16" if signed else "u16"
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], False, 7, sec)
    s0 = src0_reg(f["s0_num"], f["s0_bank"], 0, False, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], False, 7, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], False, 7, sec)
    ops = [reg_text(dest, "x"), reg_text(s0, "x")]
    for r, fmt, hi in ((s1, f["s1_fmt"], f["s1_hi"]), (s2, f["s2_fmt"], f["s2_hi"])):
        t = reg_text(r, "y" if fmt and hi else "x")
        if fmt:
            t += " u8" if not signed else " i8"
        ops.append(t)
    note = []
    if f["mode"] in (1, 3):
        note.append("sat")
    if f["s2_neg"]:
        note.append("src2 neg")
    if f["or_shift"]:
        note.append(f"or_shift {f['or_shift']}")
    return _mk("imad16", PRED_SHORT[f["pred"]], "imad", ty, ops, rpt=f["repeat"], note=" ".join(note))


def _d_smp(f, w, sec):
    dim = f["dim"] + 1
    cty = ["f32", "f16", "c10", "?"][f["coord_type"]]
    fconv = ["packed", "?", "f16", "f32"][f["fconv"]]
    pa = "sa" if sec else "pa"
    dest = Reg(pa if f["dst_pa"] else "r", f["dst_num"])
    s0 = src0_reg(f["s0_num"], f["s0_bank"], f["s0_ext"], True, 8, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], True, 8, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], True, 8, sec)
    cmask = {1: 1, 3: 7}.get(dim, 3)
    sb = ["", ".gather4", ".info", ".gather4.uv"][f["mode"]]
    ops = [reg_text(dest, "x"), reg_text(s0, swiz_text(chans("xyzw"), cmask)), reg_text(s1, None)]
    if f["lod"] != 0:
        ops.append(reg_text(s2, "x"))
    return _mk("smp", PRED_EXT[f["pred"]], f"smp{dim}d{sb}", f"{fconv}.{cty}", ops,
               note=f"lod{f['lod']}")


def _d_dp(f, w, sec):
    kind = 1 if f["wide"] == 0 else 2
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], True, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], True, 7, sec)
    s1.neg, s1.abs = bool(f["s1_neg"]), bool(f["s1_abs"])
    smask = 7 if kind == 1 else 15
    sw1 = [f["s1_x"], f["s1_y"], f["s1_z"], f["s1_w"]]
    swg = vec_swizzle(f["g0_swz"], 0, kind)
    if swg is None:
        return None
    ops = [reg_text(dest, swiz_text(chans("xyzw"), f["mask"])),
           reg_text(s1, swiz_text(sw1, smask)),
           _gpi(f["g0_num"], 0, f["g0_abs"], swiz_text(chans(swg), smask))]
    return _mk("dp", PRED_VEC[f["pred"]], "dot", "f32", ops, rpt=f["repeat"],
               note="clip" if f["clip"] else "")


def _d_comp(f, w, sec):
    op = ["rcp", "rsq", "log2", "exp2"][f["fn"]]
    types = ["f32", "f16", "c10", "?"]
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], True, 8, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], True, 8, sec)
    mod_flags(s1, f["s1_mod"])
    ops = [reg_text(dest, swiz_text(chans("xyzw"), f["mask"])),
           reg_text(s1, swiz_text(chans("xyzw"), 1 << f["s1_chan"]))]
    return _mk("comp", PRED_EXT[f["pred"]], op, None, ops, rpt=f["repeat"],
               note=f"dst {types[f['dst_type']]} src {types[f['src_type']]}")


def _d_smlsi(f, w, sec):
    parts = []
    for tag, mode, val in (("dest", f["dst_mode"], f["dst_val"]), ("src0", f["s0_mode"], f["s0_val"]),
                           ("src1", f["s1_mode"], f["s1_val"]), ("src2", f["s2_mode"], f["s2_val"])):
        parts.append(f"{tag} " + (f"swz {val:#04x}" if mode else f"{val - 256 if val > 127 else val:+d}"))
    return _mk("slsi", "", "slsi", None, parts,
               note=f"limits t{f['lim_t']} p{f['lim_p']} s{f['lim_s']}")


TST_FAMILY = {
    0: ["fadd", None, None, "fsubflr", "frcp", "frsq", "flog", "fexp",
        "fdp", "fmin", "fmax", "fdsx", "fdsy", "fmul", "fsub", None],
}
TST_VEC = [None, None, "add", "frc", "rcp", "rsq", "log2", "exp2",
           "dot", "min", "max", "ddx", "ddy", "mul", "sub", None]


TST_INT = {
    1: [None] * 6 + ["iadd16", "isub16", "imul16", "iaddu16", "isubu16", "imulu16",
                     "iadd32", "iaddu32", "isub32", "isubu32"],
    2: ["iadd8", "isub8", "iaddu8", "isubu8", "imul8", "fpmul8", "imulu8", "fpadd8",
        "fpsub8"] + [None] * 7,
    3: ["and", "or", "xor", "shl", "shr", "rol", None, "asr"] + [None] * 8,
}


def _d_tst(f, w, sec):
    """A test: the comparison fields are kept raw (sign, zero, combine,
    channel); only the ALU op that produces the tested value is named."""
    sel = f["family"]
    if sel == 0:
        op = TST_VEC[f["fn"]]
        ty = "f32" if f["prec"] else "f16"
    else:
        op = TST_INT[sel][f["fn"]]
        ty = None
    if op is None:
        return None
    dest = f"p{f['pdst']}"
    dbl = sel == 0
    bits = 8 if dbl else 7
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], dbl, bits, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], dbl, bits, sec)
    s1.neg = bool(f["s1_neg"])
    if dbl:
        cmask = [1, 2, 4, 8, 15, 15, 15, 15][f["chan"]]
        swz2 = "xxxx" if f["s2_splat"] else "xyzw"
        ops = [dest, reg_text(s1, swiz_text(chans("xyzw"), cmask)),
               reg_text(s2, swiz_text(chans(swz2), cmask))]
    else:
        ops = [dest, reg_text(s1, None), reg_text(s2, None)]
    note = f"sign {f['sign']} zero {f['zero']} comb {f['comb']} chan {f['chan']}"
    return _mk("tst", PRED_EXT[f["pred"]], f"tst.{op}", ty, ops, rpt=f["repeat"], note=note)


def _d_imad32(f, w, sec):
    signed = bool(f["signed"])
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], False, 7, sec)
    s0 = src0_reg(f["s0_num"], f["s0_bank"], 0, False, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], False, 7, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], False, 7, sec)
    wide2 = f["s2_type"] == 2
    ops = [reg_text(dest, "x"),
           reg_text(s0, "y" if f["s0_hi"] else "x") + " 16",
           reg_text(s1, "y" if f["s1_hi"] else "x") + " 16",
           reg_text(s2, "y" if f["s2_hi"] and not wide2 else "x") + (" 32" if wide2 else " 16")]
    note = " ".join(x for x in ("sat" if f["sat"] else "", ) if x)
    return _mk("imad32", PRED_SHORT[f["pred"]], "imad", "i32" if signed else "u32", ops,
               rpt=f["repeat"], note=note)


def _d_imad32x2(f, w, sec):
    signed = bool(f["signed"])
    dest = dest_reg(f["dst_num"], f["dst_bank"], f["dst_ext"], False, 7, sec)
    s0 = src0_reg(f["s0_num"], f["s0_bank"], f["s0_ext"], False, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], False, 7, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], False, 7, sec)
    s1.neg, s2.neg = bool(f["s1_neg"]), bool(f["s2_neg"])
    ops = [reg_text(dest, None), reg_text(s0, None), reg_text(s1, None), reg_text(s2, None)]
    return _mk("imad32x2", PRED_EXT[f["pred"]], "imad", "i32" if signed else "u32", ops,
               rpt=f["count"], note=f"sn {f['sn']}")


def _d_kill(f, w, sec):
    return _mk("kill", PRED_SHORT[f["pred"]], "kill", None, [])


def _d_depthf(f, w, sec):
    s0 = src0_reg(f["s0_num"], f["s0_bank"], f["s0_ext"], True, 7, sec)
    s1 = src12_reg(f["s1_num"], f["s1_bank"], f["s1_ext"], False, 7, sec)
    s2 = src12_reg(f["s2_num"], f["s2_bank"], f["s2_ext"], False, 7, sec)
    return _mk("depthf", PRED_SHORT[f["pred"]], "depth", None,
               [reg_text(s0, "x"), reg_text(s1, "x"), reg_text(s2, "x")],
               note="two-sided" if f["two_sided"] else "")


def _d_branch(f, w, sec):
    off = f["offset"]
    return _mk("branch", PRED_EXT[f["pred"]], "br", None, [f"{off:#x}"],
               note="raw offset, units unread")


def _d_nop(f, w, sec):
    return _mk("nop", "", "nop", None, [])


def _d_phase(f, w, sec):
    return _mk("phase", "", "phase", None, [])


def _d_spec(f, w, sec):
    return _mk("spec", "", "spec", None, [], note=f"category {f['category']}, special {f['special']}")


_DECODERS = {
    "mad2": _d_mad2,
    "nmad32": _d_nmad,
    "nmad16": _d_nmad,
    "mad": _d_mad,
    "mov": _d_mov,
    "pck": _d_pck,
    "bw": _d_bw,
    "imad16": _d_imad16,
    "smp": _d_smp,
    "dp": _d_dp,
    "comp": _d_comp,
    "slsi": _d_smlsi,
    "tst": _d_tst,
    "imad32": _d_imad32,
    "imad32x2": _d_imad32x2,
    "kill": _d_kill,
    "depthf": _d_depthf,
    "branch": _d_branch,
    "nop": _d_nop,
    "phase": _d_phase,
    "spec": _d_spec,
}


def _load_gxp():
    here = os.path.dirname(os.path.abspath(__file__))
    spec = importlib.util.spec_from_file_location("vita_gxp", os.path.join(here, "vita-gxp.py"))
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def cmd_disasm(spec: str, index: int) -> int:
    vg = _load_gxp()
    n = 0
    for _label, p, _at in vg.walk(spec):
        if isinstance(p, vg.Broken):
            continue
        if n == index:
            print(f"# {p.kind()} program, {p.primary_count} primary, {p.secondary_count} secondary")
            for blk in ("secondary", "primary"):
                print(f"{blk}:")
                for i, w in enumerate(p.words(blk)):
                    ins = decode(w, blk == "secondary")
                    print(f"  {i:3d}  {w:016x}  [{ins.cls}] {ins.text or ''}")
            return 0
        n += 1
    print("no such program")
    return 1


def cmd_census(specs: list[str]) -> int:
    vg = _load_gxp()
    cls_count: Counter = Counter()
    listed: Counter = Counter()
    unknown: Counter = Counter()
    programs = full = fully_listed = 0
    refused: Counter = Counter()
    for spec in specs:
        for _label, p, _at in vg.walk(spec):
            if isinstance(p, vg.Broken):
                continue
            programs += 1
            ok = True
            listed_all = True
            for blk in ("primary", "secondary"):
                for w in p.words(blk):
                    ins = decode(w, blk == "secondary")
                    cls_count[ins.cls] += 1
                    if ins.cls == "unknown":
                        unknown[w >> 59] += 1
                        ok = False
                    elif ins.text is not None:
                        listed[ins.cls] += 1
                    else:
                        listed_all = False
                        refused[(ins.cls, w >> 56)] += 1
            full += ok
            fully_listed += listed_all
    print(f"{programs} programs, {full} with every instruction classified, "
          f"{fully_listed} with every instruction listed")
    for c, n in cls_count.most_common():
        print(f"  {c:10s} {n:9d}  listed {listed[c]:9d}")
    print("refused by name (class, top byte): count")
    for (c, top), n in refused.most_common():
        print(f"  {c:10s} {top:#04x} {n:8d}")
    return 0


def main(argv: list[str]) -> int:
    if len(argv) >= 4 and argv[1] == "disasm":
        return cmd_disasm(argv[2], int(argv[3]))
    if len(argv) >= 3 and argv[1] == "census":
        return cmd_census(argv[2:])
    print(__doc__)
    return 2


if __name__ == "__main__":
    raise SystemExit(main(sys.argv))
