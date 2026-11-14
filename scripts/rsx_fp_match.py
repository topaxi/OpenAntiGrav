#!/usr/bin/env python3
"""Attribute each draw of a dumped RSX frame to the disc fragment program it runs.

    rsx_fp_match.py <dump-dir> <stem> <rcsmaterial-dir> [--min-instr N]

`rpcs3_draw_hook.py` dumps every draw's fragment program as `<stem>-<address>.bin`. A draw's live
program is disassembled with `ps3-microcode.py`, its constants stripped, and compared with every
fragment block of every `*.rcsmaterial` in `<rcsmaterial-dir>` (extract them with
`psarc.py cat <image>:<archive> <path>`). One output line per attributed draw: draw number,
render target, index count, blend, depth, unit 0's `SET_TEXTURE_ADDRESS` word (gamma nibble in bits
20-23), format and rect words, then `material#block` names. A block of fewer than `--min-instr`
instructions (default 4) is skipped: the one-tap texture copies are the same in every material and
in the bloom chain, so they name nothing.
"""
import contextlib
import glob
import importlib.util
import io
import os
import re
import subprocess
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
_spec = importlib.util.spec_from_file_location("ps3_microcode", HERE / "ps3-microcode.py")
mc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(mc)


def normalise(text):
    out = []
    for line in text.splitlines():
        m = re.match(r"@0x[0-9a-f]+\s+(.*)", line)
        if not m:
            continue
        s = re.sub(r"\[const@[^\]]*\]", "", m[1])
        s = re.sub(r"\{[^}]*\}(\.\w+)?", "K", s)
        s = s.replace("[writes cc]", "")
        out.append(re.sub(r"\s+", " ", s).strip())
    return "\n".join(out)


def disc_blocks(mats_dir, min_instr=4):
    table = {}
    for path in sorted(glob.glob(os.path.join(mats_dir, "*.rcsmaterial"))):
        raw = Path(path).read_bytes()
        for index, at in mc.blocks_in(raw, True):
            buf = io.StringIO()
            with contextlib.redirect_stdout(buf):
                mc.fp_disasm(raw, at)
            key = normalise(buf.getvalue())
            if key.count("\n") + 1 >= min_instr:
                table.setdefault(key, []).append("%s#%d" % (Path(path).stem, index))
    return table


def draws(directory, stem):
    out = subprocess.run([sys.executable, str(HERE / "rsx-draw-list.py"), directory, stem, "--samplers"],
                         capture_output=True, text=True).stdout.splitlines()
    rows, cur = [], None
    for line in out:
        m = re.match(r"\s*(\d+) tgt (\w+) idx(\d+)\s+fp (\w+) bl (\d) (\S+) eq (\S+) at (\S+) cull (\S+) dep (\S+)", line)
        if m:
            cur = dict(n=int(m[1]), tgt=m[2], idx=int(m[3]), fp=int(m[4], 16), bl=m[5], bf=m[6], dep=m[10])
            rows.append(cur)
            continue
        m = re.match(r"\s*unit\s+0 off (\w+) fmt (\w+) addr (\w+).*rect (\w+)", line)
        if m and cur is not None and "u0" not in cur:
            cur.update(u0=m[3], fmt=m[2], rect=m[4])
    return rows


def attribute(directory, stem, mats_dir, min_instr=4, table=None):
    """Rows of the dumped frame's draws, each with `prog`: the matching disc blocks, or `None`."""
    table = table if table is not None else disc_blocks(mats_dir, min_instr)
    rows = draws(directory, stem)
    for r in rows:
        path = "%s/%s-%08x.bin" % (directory, stem, 0xC0000000 | (r["fp"] & ~1))
        r["prog"] = None
        if not os.path.exists(path):
            continue
        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            try:
                mc.fp_code(Path(path).read_bytes(), 0, os.path.getsize(path), {})
            except Exception:
                continue
        r["prog"] = ",".join(table.get(normalise(buf.getvalue()), [])) or None
    return rows


def main():
    mins = 4
    args = sys.argv[1:]
    if "--min-instr" in args:
        i = args.index("--min-instr")
        mins = int(args[i + 1])
        del args[i:i + 2]
    directory, stem, mats = args
    rows = attribute(directory, stem, mats, mins)
    for r in rows:
        if r["prog"]:
            print(r["n"], r["tgt"], "idx", r["idx"], "bl", r["bl"], r["bf"], r["dep"], "u0", r.get("u0"),
                  r.get("fmt"), r.get("rect"), r["prog"])
    print("attributed %d of %d draws" % (sum(1 for r in rows if r["prog"]), len(rows)))


if __name__ == "__main__":
    main()
