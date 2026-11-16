#!/usr/bin/env python3
"""Does every 2048 fragment program that declares `alpha_emissive` and `glowtinrt` add the
picture's alpha times that colour?

    vita-alpha-emissive-sweep.py <psarc> [<psarc> ...]

For each fragment program of each `.rcsmaterial` entry that declares the sampler `alpha_emissive`
(resource 0 on all of them) it looks for the USSE instruction

    mad.f16 <dst>.xyzw, paN.wwww, saK.xyzw, <src>.xyzw

where `saK` is the register the `glowtinrt` uniform lives in (its `resource` field,
`docs/formats/gxp.md`), which is `texel.a * glowtinrt + lit`. Prints the counts, one line per
program that declares the sampler and lacks either the uniform or the instruction, and exits 1 if
a program that declares both has no such instruction.
"""
import importlib.util
import os
import re
import sys
from collections import Counter

HERE = os.path.dirname(os.path.abspath(__file__))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, file))
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


vg = load("vita_gxp", "vita-gxp.py")
vu = load("vita_usse", "vita-usse.py")

ADD = re.compile(r"mad\.f16 \S+, pa\d+\.wwww, sa(\d+)\.xyzw, ")


def main(paths):
    both = Counter()
    bad = 0
    declared_only = Counter()
    for path in paths:
        for label, prog, _at in vg.walk(path):
            if isinstance(prog, vg.Broken) or not prog.fragment:
                continue
            if not label.endswith(".rcsmaterial"):
                continue
            names = {q.name: q for q in prog.parameters}
            if "alpha_emissive" not in names:
                continue
            family = label.rsplit("/", 1)[-1][: -len(".rcsmaterial")]
            tint = names.get("glowtinrt")
            if tint is None:
                declared_only[family] += 1
                continue
            hit = False
            for word in prog.words("primary"):
                m = ADD.search(vu.decode(word).text or "")
                if m and int(m.group(1)) == tint.resource:
                    hit = True
                    break
            both[(family, hit)] += 1
            if not hit:
                bad += 1
                print(f"MISSING {label} #{prog.index}")
    print("declare alpha_emissive and glowtinrt, by family (add found):")
    for (family, hit), n in sorted(both.items()):
        print(f"  {n:4d}  {'yes' if hit else 'NO '}  {family}")
    print("declare alpha_emissive without glowtinrt:")
    for family, n in sorted(declared_only.items()):
        print(f"  {n:4d}  {family}")
    print(f"total with both: {sum(both.values())}, without the instruction: {bad}")
    return 1 if bad else 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
