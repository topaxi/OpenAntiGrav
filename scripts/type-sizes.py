#!/usr/bin/env python3
"""List every struct/enum a workspace crate defines with its size and alignment.

Uses nightly `-Zprint-type-sizes`, which reports each type rustc laid out while
compiling a crate (stable has no equivalent). Only types actually instantiated
are reported, so an unused generic is absent. Layout is for the host target.

usage: type-sizes.py [crate ...]    (default: every workspace library crate)
Writes data/type-sizes.tsv (crate, type, size, align), largest first, and
prints the top rows.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = ROOT / "data" / "type-sizes-target"
OUT = ROOT / "data" / "type-sizes.tsv"
LINE = re.compile(r"^print-type-size type: `(.+)`: (\d+) bytes, alignment: (\d+) bytes$")


def libs() -> tuple[dict[str, str], dict[str, set[str]]]:
    """Library crate name per package, and each package's dependency crate names."""
    meta = subprocess.run(
        ["cargo", "metadata", "--format-version", "1"],
        cwd=ROOT, capture_output=True, text=True, check=True,
    )
    data = json.loads(meta.stdout)
    members = set(data["workspace_members"])
    out, deps = {}, {}
    for pkg in data["packages"]:
        if pkg["id"] not in members:
            continue
        for t in pkg["targets"]:
            if "lib" in t["kind"]:
                out[pkg["name"]] = t["name"]
        deps[pkg["name"]] = {d["name"].replace("-", "_") for d in pkg["dependencies"]}
    return out, deps


FOREIGN_STD = {"std", "core", "alloc"}


def sizes(pkg: str, foreign: set[str]) -> list[tuple[str, int, int]]:
    """rustc prints a crate's own types without a crate prefix (`world::World`)
    and everyone else's with one, so drop names led by std or a dependency."""
    base = ["cargo", "+nightly"]
    subprocess.run([*base, "clean", "-p", pkg, "--target-dir", str(TARGET)],
                   cwd=ROOT, capture_output=True)
    run = subprocess.run(
        [*base, "rustc", "-p", pkg, "--lib", "--target-dir", str(TARGET),
         "--", "-Zprint-type-sizes"],
        cwd=ROOT, capture_output=True, text=True,
    )
    if run.returncode != 0:
        print(f"type-sizes: {pkg}: build failed\n{run.stderr[-2000:]}", file=sys.stderr)
        return []
    rows = []
    for line in (run.stdout + run.stderr).splitlines():
        m = LINE.match(line)
        if not m:
            continue
        name = m.group(1)
        head = name.split("::", 1)[0]
        if "::" not in name or not re.fullmatch(r"\w+", head) or head in foreign:
            continue
        rows.append((name, int(m.group(2)), int(m.group(3))))
    return rows


def main() -> int:
    all_libs, deps = libs()
    wanted = sys.argv[1:] or sorted(all_libs)
    rows = []
    for pkg in wanted:
        if pkg not in all_libs:
            print(f"type-sizes: no library crate {pkg!r}", file=sys.stderr)
            return 1
        print(f"type-sizes: {pkg}", file=sys.stderr)
        rows += [(pkg, *r) for r in sizes(
            pkg, FOREIGN_STD | set(all_libs.values()) | deps[pkg] - {all_libs[pkg]})]
    rows.sort(key=lambda r: (-r[2], r[0], r[1]))
    OUT.parent.mkdir(exist_ok=True)
    with OUT.open("w") as f:
        f.write("crate\ttype\tsize\talign\n")
        for r in rows:
            f.write("\t".join(map(str, r)) + "\n")
    for r in rows[:20]:
        print(f"{r[2]:>10}  {r[0]}  {r[1]}")
    print(f"type-sizes: {len(rows)} types -> {OUT.relative_to(ROOT)}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
