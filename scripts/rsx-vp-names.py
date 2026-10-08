#!/usr/bin/env python3
"""Name each draw's vertex program in a dumped RSX frame against the executable's vertex blocks.

    python3 scripts/rsx-vp-names.py <dir> <stem> [draw numbers ...]

Replays the stream's vertex program uploads (`TRANSFORM_PROGRAM` `0x0b80`, load slot `0x1e9c`,
start slot `0x1ea0`) to rebuild the program each draw ran, renders it with `scripts/ps3-microcode.py`,
drops the constant register numbers (the engine relocates them), and compares the instruction
text with the same reduction of every vertex block of the HD `EBOOT.elf`. With draw numbers it
prints those draws; without, one line per distinct program: the matching blocks (`-` when none, which
is a program the executable does not carry, for example one built from a `.rcsmaterial`), the draw
count and the first four draw numbers. Draw numbers are `scripts/rsx-draw-list.py`'s.
"""
import contextlib
import glob
import importlib.util
import io
import os
import re
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


def norm(text):
    text = re.sub(r"c\[\d+\]", "c", text.strip())
    return re.sub(r"^\s*\d+\s+", "", text)


def main():
    directory, stem = sys.argv[1:3]
    wanted = {int(x) for x in sys.argv[3:]}
    mc = load("ps3_microcode", "ps3-microcode.py")
    fifo = load("rsx_fifo", "rsx_fifo.py")
    elf = mc.Elf(mc.DEFAULT_ELF)
    blocks = {}
    for va in range(0x927800, 0x936D80, 0x10):
        offset = elf.offset(va)
        if elf.raw[offset:offset + 4] != b"SHO\x08":
            continue
        buffer = io.StringIO()
        try:
            with contextlib.redirect_stdout(buffer):
                mc.vp_disasm(elf.raw, offset)
        except Exception:
            continue
        lines = tuple(norm(line) for line in buffer.getvalue().split("\n") if re.match(r"\s+\d+\s+[A-Z]", line))
        if lines:
            blocks.setdefault(lines, []).append(va)

    def render(words):
        out = []
        for i in range(0, len(words) - 3, 4):
            out.append(norm(mc.vp_render(*words[i:i + 4])))
            if words[i + 3] & 1:
                break
        return tuple(out)

    memory = fifo.Mem.load(sorted(glob.glob("%s/%s-4*.bin" % (directory, stem))))
    program, load_slot, start, number, seen = {}, 0, 0, 0, {}
    for position, method, _noinc, args in fifo.Walk(memory).run(0x1000):
        if (memory.word(position) >> 13) & 7:
            continue
        if method == 0x1E9C:
            load_slot = args[0]
        elif method == 0xB80:
            for i in range(0, len(args) - 3, 4):
                program[load_slot] = tuple(args[i:i + 4])
                load_slot += 1
        elif method == 0x1EA0:
            start = args[0] & 0xFFF
        elif method in (0x1814, 0x1824):
            words, slot = [], start
            while slot in program and len(words) < 2048:
                words.extend(program[slot])
                slot += 1
                if program[slot - 1][3] & 1:
                    break
            hit = blocks.get(render(tuple(words)))
            if number in wanted:
                print("draw %d vertex program start %d -> %s" % (number, start, [hex(x) for x in hit] if hit else None))
            seen.setdefault(tuple(hit) if hit else None, []).append(number)
            number += 1
    if not wanted:
        for key, draws in seen.items():
            print("%-30s x%-4d first %s" % ([hex(x) for x in key] if key else "-", len(draws), draws[:4]))


if __name__ == "__main__":
    main()
