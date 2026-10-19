#!/usr/bin/env python3
"""Match a live RSX fragment program to a block of a `.rcsmaterial`.

    python3 scripts/rsx-fp-match-material.py <dump-dir> <stem> <material.rcsmaterial>... -- <fp-offset>...

`<dump-dir>/<stem>-c0<fp>.bin` are the fragment programs `rpcs3_draw_hook.py` dumps (`<fp-offset>` is a draw's
`fp` column from `rsx-draw-list.py`, low nibble cleared, as hex). Each material's fragment blocks are decoded with
`ps3-microcode.py fp-file`, reduced to their opcode streams (constants and patch comments dropped, because the
engine patches constants into a program in place) and compared with the live program. A line is `MATCH` for an equal
stream and `prefix12` when only the first 12 instructions agree (a live dump can be cut short). The block number
and file offset it prints are the ones `crates/render/examples/hd_material_variants.rs` names with a feature set,
which is how a live draw's permutation is read (`docs/rendering/hd-ship-materials.md`, "Finding 4").
"""
import os
import re
import subprocess
import sys

HERE = os.path.dirname(os.path.abspath(__file__))


def run(script, *args):
    return subprocess.run([sys.executable, "-I", os.path.join(HERE, script), *args],
                          capture_output=True, text=True).stdout


def norm(text):
    out = []
    for line in text.split("\n"):
        if not line.startswith("@"):
            continue
        line = re.sub(r"\{[^}]*\}(\.\w+)?", "K", line)
        line = re.sub(r"\[const.*?\]", "", line)
        out.append(re.sub(r"^@0x[0-9a-f]+\s+", "", line).strip())
    return "\n".join(out)


def main():
    args = sys.argv[1:]
    if "--" not in args or len(args) < 5:
        sys.exit(__doc__)
    split = args.index("--")
    directory, stem, materials, fps = args[0], args[1], args[2:split], args[split + 1:]
    live = {fp: norm(run("ps3-fp-live.py", "%s/%s-%08x.bin" % (directory, stem, 0xC0000000 + int(fp, 16))))
            for fp in fps}
    for material in materials:
        text = run("ps3-microcode.py", "fp-file", material)
        for block in re.split(r"(?m)^(?=fragment block #)", text)[1:]:
            header = block.split("\n")[0]
            stream = norm(block).split("\n")
            for fp, ref in live.items():
                ref = ref.split("\n")
                if stream == ref:
                    print("MATCH   %s %s: %s" % (fp, os.path.basename(material), header))
                elif stream[:12] == ref[:12]:
                    print("prefix12 %s %s: %s" % (fp, os.path.basename(material), header))


if __name__ == "__main__":
    main()
