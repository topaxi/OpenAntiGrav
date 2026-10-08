#!/usr/bin/env python3
"""Name the fragment programs of a dumped RSX frame by matching them to the executable's blocks.

    python3 scripts/rsx-fp-names.py <dir> <stem>

Run on a `scripts/rpcs3_draw_hook.py` dump (`<dir>/<stem>-4*.bin` plus the `-c0...` program spans).
Each live program is disassembled (`scripts/ps3-fp-live.py`), reduced to its opcode stream (constants
and patch comments dropped, because the engine patches constants into the program in place between
draws and only the last patch survives in a dump), and compared with the same reduction of every
fragment block of the HD `EBOOT.elf`. A line is `address xN block,block,...`; several blocks on one
line means their opcode streams are the same and the draw cannot tell them apart by code alone.
`scripts/ps3-registry.py` maps 62 of the blocks to their registered names.

Reading a frame is only sound when the frame is complete; see `scripts/rsx-frame-census.py`.
"""
import importlib.util
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


def norm(text):
    out = []
    for line in text.split("\n"):
        if not line.startswith("@"):
            continue
        line = re.sub(r"\{[^}]*\}(\.\w+)?", "K", line)
        line = re.sub(r"\[const.*?\]", "", line)
        out.append(re.sub(r"^@0x[0-9a-f]+\s+", "", line).strip())
    return "\n".join(out)


def run(script, *args):
    return subprocess.run([sys.executable, "-I", os.path.join(HERE, script), *args],
                          capture_output=True, text=True).stdout


def main():
    directory, stem = sys.argv[1:3]
    mc = load("ps3_microcode", "ps3-microcode.py")
    rdl = load("rsx_draw_list", "rsx-draw-list.py")
    draws, _ = rdl.draws(directory, stem)
    elf = mc.Elf(mc.DEFAULT_ELF)
    blocks = {}
    for va in range(0x927800, 0x936D80, 0x10):
        if elf.raw[elf.offset(va):elf.offset(va) + 4] == b"SHO\x08":
            text = norm(run("ps3-microcode.py", "fp", "0x%08x" % va))
            if text:
                blocks[va] = text
    programs = sorted({d.get(0x8E4, 0) & ~0xF for d in draws})
    print("%d fragment blocks, %d live programs" % (len(blocks), len(programs)))
    for fp in programs:
        path = "%s/%s-%08x.bin" % (directory, stem, 0xC0000000 + fp)
        if not os.path.exists(path):
            print("%08x: not dumped" % fp)
            continue
        live = norm(run("ps3-fp-live.py", path))
        hits = [hex(va) for va, text in blocks.items() if text == live]
        count = sum(1 for d in draws if d.get(0x8E4, 0) & ~0xF == fp)
        print("%08x x%-3d %s" % (fp, count, ",".join(hits) if hits else "-"))


if __name__ == "__main__":
    main()
