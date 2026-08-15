#!/usr/bin/env python3
"""Keep the platform's libm out of the simulation crates.

`docs/architecture/determinism.md` forbids `sin`/`cos`/`acos`/`exp`/... in
simulation code: IEEE-754 requires `sqrt` to be correctly rounded and does not
require it of the transcendentals, so they resolve to whatever the host's libm
does and two targets may legitimately differ in the last bit. A bit in an angle
that feeds a speed target is a bit in the world hash.

Until 2026-08-15 that rule was enforced by review alone, and it had already been
broken: `oag_ai::Line::curvature` called `f32::acos` from the day it was
written. Nothing failed, because no cross-platform gate ran a driver - the fix
was `oag_core::math::acos` (our own libm) plus a determinism gate over
`oag-ai`, and this script is the third leg, so the *next* one cannot land the
same way. It fails like clippy or fmt rather than like a hash mismatch on
somebody else's macOS runner three weeks later.

What it does not do: parse Rust. It strips comments and `#[cfg(test)]` blocks
and then matches method calls textually, which is enough for a rule about a
fixed list of method names and cheap enough to run on every `just`.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The crates whose arithmetic reaches a committed state hash. `oag-render`,
# `oag-game` and the tools are deliberately absent: a renderer may call
# whatever it likes, because a pixel is not compared across machines.
SIMULATION_CRATES = ("core", "physics", "gameplay", "ai", "race")

# Not required by IEEE-754 to be correctly rounded, so not portable. `sqrt` is
# absent on purpose - it *is* required, and glam's `length`/`normalize` are
# built on it. `to_radians`/`to_degrees` are absent too: they are a multiply by
# a constant.
FORBIDDEN = (
    "sin",
    "cos",
    "tan",
    "asin",
    "acos",
    "atan",
    "atan2",
    "sin_cos",
    "exp",
    "exp2",
    "exp_m1",
    "ln",
    "ln_1p",
    "log",
    "log2",
    "log10",
    "powf",
    "sinh",
    "cosh",
    "tanh",
    "asinh",
    "acosh",
    "atanh",
    "cbrt",
    "hypot",
)

# Files allowed to call them, each for a stated reason. **A new entry here is a
# decision about determinism**, not a formality - it says this call cannot reach
# a state hash, or that it is the wrapper everything else goes through.
ALLOWED = {
    "crates/core/src/math.rs": (
        "the wrappers themselves - `acos` here is our own libm, which is what "
        "every simulation caller is required to use instead of the platform's"
    ),
    "crates/core/src/probe.rs": (
        "the determinism probe calls `sin` deliberately, so a platform whose "
        "libm differs shows up as a failing gate rather than as a mystery "
        "desync - see determinism.md"
    ),
}

METHOD_CALL = re.compile(r"\.\s*(" + "|".join(FORBIDDEN) + r")\s*\(")
PATH_CALL = re.compile(r"\bf(?:32|64)::\s*(" + "|".join(FORBIDDEN) + r")\s*\(")


def blank(text: str, start: int, end: int) -> str:
    """Replace `text[start:end]` with spaces, keeping every newline.

    Line numbers have to survive, because the whole output of this script is
    `file:line`.
    """
    cut = "".join("\n" if char == "\n" else " " for char in text[start:end])
    return text[:start] + cut + text[end:]


def strip_comments(source: str) -> str:
    """Blank out line and block comments, and string literals.

    A doc comment saying "the obvious way to draw a circuit is arcs off
    `angle.cos()`" is a real line in `oag_ai::probe`, and reporting it would
    train the reader to ignore this script.
    """
    out = source
    index = 0
    while index < len(out):
        if out.startswith("//", index):
            end = out.find("\n", index)
            end = len(out) if end == -1 else end
            out = blank(out, index, end)
            index = end
        elif out.startswith("/*", index):
            end = out.find("*/", index + 2)
            end = len(out) if end == -1 else end + 2
            out = blank(out, index, end)
            index = end
        elif out[index] == '"':
            end = index + 1
            while end < len(out):
                if out[end] == "\\":
                    end += 2
                    continue
                if out[end] == '"':
                    end += 1
                    break
                end += 1
            out = blank(out, index, end)
            index = end
        else:
            index += 1
    return out


def strip_test_modules(source: str) -> str:
    """Blank out every `#[cfg(test)]` item, braces balanced.

    Test code may call anything: a test asserting an angle in degrees is not
    simulation state, and `closed_loop.rs`'s fixtures draw their ovals with
    `cos` on purpose.
    """
    out = source
    for match in list(re.finditer(r"#\[cfg\(test\)\]", out)):
        opening = out.find("{", match.end())
        if opening == -1:
            continue
        depth = 0
        end = opening
        while end < len(out):
            if out[end] == "{":
                depth += 1
            elif out[end] == "}":
                depth -= 1
                if depth == 0:
                    end += 1
                    break
            end += 1
        out = blank(out, match.start(), end)
    return out


def main() -> int:
    findings: list[str] = []
    scanned = 0

    for crate in SIMULATION_CRATES:
        root = ROOT / "crates" / crate / "src"
        if not root.is_dir():
            print(f"no such crate directory: {root.relative_to(ROOT)}")
            return 1
        for path in sorted(root.rglob("*.rs")):
            relative = path.relative_to(ROOT).as_posix()
            if relative in ALLOWED:
                continue
            scanned += 1
            source = strip_test_modules(strip_comments(path.read_text()))
            for number, line in enumerate(source.splitlines(), start=1):
                for pattern in (METHOD_CALL, PATH_CALL):
                    found = pattern.search(line)
                    if found:
                        findings.append(f"{relative}:{number}: {found.group(1)}")

    if findings:
        print("platform transcendentals in simulation code:\n")
        for finding in findings:
            print(f"  {finding}")
        print(
            "\nIEEE-754 does not require these to be correctly rounded, so they "
            "differ between\ntargets and the difference reaches the world hash. "
            "Use `oag_core::math`, adding a\nwrapper over our own libm if the one "
            "you need is not there yet - see\ndocs/architecture/determinism.md. "
            "If the call genuinely cannot reach simulation\nstate, add the file "
            "to ALLOWED in this script with the reason."
        )
        return 1

    print(
        f"OK: no platform transcendentals in {scanned} simulation source file(s), "
        f"{len(ALLOWED)} allowed by name"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
