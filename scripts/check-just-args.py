#!/usr/bin/env python3
"""A `*ARGS` recipe must hand its arguments back exactly as typed.

Every WAD-internal path on these discs is authored with backslashes
(`Data\\Ships\\Feisar\\ship.vex`). `just` interpolates a bare `{{ARGS}}` into
the shell command line unquoted, and an unquoted backslash is an escape
character to `sh` - it deletes the separators silently, with no error. The
fix is `set positional-arguments` plus a recipe body that references `"$@"`
instead of `{{ARGS}}`, which is what every `*ARGS`-only recipe in `justfile`
now does - see that file's own comment above `set positional-arguments` for
which recipes were deliberately left out (the ones with a named parameter
before `*ARGS`, none of which take a WAD-internal path).

This script is the regression test: it calls `justfile`'s `_just-args-echo`
fixture with a handful of arguments a WAD path or a quoted shell word can
actually contain, and asserts each one survives unchanged. Needs no disc
data, so it runs in the plain `just check` gate, not only `test-data`.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# Each case is the argv `just` should hand to the fixture recipe, verbatim.
CASES: list[list[str]] = [
    ["Data\\Ships\\Feisar\\ship.vex", "--mesh"],
    ["data/images/pulse-psp-eu.chd:Data\\Environments\\01_Vineta_K\\track.vex"],
    ["a b", "c\\d", "--flag"],
]


def run_case(args: list[str]) -> list[str]:
    result = subprocess.run(
        ["just", "_just-args-echo", *args],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        print(f"FAIL: `just _just-args-echo {args!r}` exited {result.returncode}")
        print(result.stderr, end="")
        return []
    return result.stdout.splitlines()


def main() -> int:
    ok = True
    for args in CASES:
        got = run_case(args)
        if got != args:
            ok = False
            print(f"FAIL: passed {args!r}, got back {got!r} - a separator was eaten")

    if not ok:
        return 1

    print(f"OK: {len(CASES)} case(s) survived just's own quoting unmangled")
    return 0


if __name__ == "__main__":
    sys.exit(main())
