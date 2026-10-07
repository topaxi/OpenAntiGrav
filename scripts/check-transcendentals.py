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

Two things widened on 2026-08-18, both from the review that day. The crate set
gained `oag-formats` (finding I2), because a transcendental applied to a
handling stat or a spline at *load* time reaches the world hash as surely as
one applied at tick time. And the patterns gained the rotation constructors
(finding D1): matching method names alone could never see
`Quat::from_axis_angle`, which is a `sin_cos` wearing a type name and which had
been feeding the rocket fan's spread into hashed state in plain sight.

One interaction worth knowing about, because it is a way this gate could have
gone wrong quietly. `scripts/check-file-size.py` requires a `#[cfg(test)]`
module over 200 lines to move into a file of its own, and what lands there is
the module's *body* - the `#[cfg(test)]` this script strips on stays behind on
the `mod tests;` declaration. So a `tests.rs` under a simulation crate's `src/`
is test code with no marker in it, and is skipped because **something declared
it** `#[cfg(test)]` - not because of what it is called. Trusting the name would
quietly exempt the first `src/lap_tests.rs` somebody writes as real code.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# The crates whose arithmetic reaches a committed state hash, plus the crate
# that *parses the numbers it starts from*. `oag-render`, `oag-game` and the
# tools are deliberately absent: a renderer may call whatever it likes, because
# a pixel is not compared across machines.
#
# `formats` joined 2026-08-18 (finding I2). It is not simulation code and never
# will be, but handling stats, splines and track data are read there and handed
# straight to the simulation, so a transcendental applied to a parsed value at
# load time reaches the world hash exactly as surely as one applied at tick
# time - and CLAUDE.md's claim ("no platform transcendental reaches simulation
# code") was already the broader of the two. It carries one live call, which
# ALLOWED names.
# `video` is here because `formats` was, and the split under ADR-0050 must not
# quietly narrow what this gate covers: every crate carved out of `oag-formats`
# joins this tuple on the way out, whether or not its arithmetic looks like it
# could reach a hash. Narrowing the list is a separate, argued change.
SCANNED_CRATES = (
    "core", "physics", "weapons", "gameplay", "ai", "race", "formats", "video", "tables",
    "texture", "vex", "pob", "rcs", "display", "ui", "ui-screens", "replay",
)

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
    "crates/display/src/display.rs": (
        "`Fov::apply` scales the tangent of the half-angle, because the "
        "tangent is what a projection matrix is built from - the setting has "
        "to mean the same fraction of the screen at every field width. This "
        "reaches a **pixel**, never a hash: `Fov` appears in no simulation "
        "crate at all (`oag-core`, `oag-physics`, `oag-gameplay`, `oag-race`, "
        "`oag-ai` name it nowhere), and the camera it configures is downstream "
        "of the tick rather than an input to it. Surfaced when ADR-0050's "
        "sibling split moved `display` into its own crate and this scan "
        "covered it for the first time"
    ),
    "crates/formats/src/entropy.rs": (
        "Shannon entropy over a byte histogram, and the decision it is: this "
        "is a *sniffing heuristic* - `oag-unpack sniff` is its only caller in "
        "the workspace - that labels a blob sparse/structured/mixed/compressed "
        "so a human can triage an archive. No parser branches on it, no value "
        "derived from it is handed to the simulation, and a last-bit "
        "disagreement between two platforms' `log2` would at worst move a "
        "printed number. It is f64 as well, which nothing in the simulation "
        "may be"
    ),
    "crates/ui/src/backdrop.rs": (
        "`camera`'s `tan` is the Fury menu backdrop's projection - the tangent "
        "of half the authored `fovy`, the same term `display.rs`'s entry above "
        "is allowed for and for the same reason. The frame it builds is handed "
        "to `oag_game::render` as a `Draw` and reaches a pixel, never a hash: "
        "no simulation crate names `oag_ui::backdrop`, and the menus draw "
        "nothing the tick reads"
    ),
    "crates/ui/src/frontend/draw/widget_alpha.rs": (
        "`pulse_alpha`'s `sin` shapes the PRESS START text's fade-in/pulse "
        "curve, and `menu/skin.rs`'s `cos` (below) shapes a selected row's "
        "highlight the same way. Both return an `f32` alpha or colour channel "
        "consumed only by `Draw::Text`/`Skin::selected`'s own drawing - the "
        "same 'reaches a pixel, never a hash' argument `crates/display/src/"
        "display.rs`'s `Fov::apply` entry above makes. Neither function "
        "appears in `oag-core`, `oag-physics`, `oag-gameplay`, `oag-race` or "
        "`oag-ai`, and both are downstream of the tick rather than an input "
        "to it. Surfaced when `oag-ui` joined `SCANNED_CRATES` on extraction "
        "from `oag-game`, which this scan never covered. Moved out of "
        "`draw.rs` itself into this sibling file (`pure-titlefade`'s own "
        "`fade_alpha`, a plain linear ramp with no transcendental, joined it) "
        "under the 1,000-line rule - same function, same reasoning, new path"
    ),
    "crates/ui/src/menu/skin.rs": (
        "See `crates/ui/src/frontend/draw.rs`'s entry above - `Skin::selected`'s "
        "`cos` is the same presentation-only pulse, just for a menu row's "
        "highlight colour rather than a text alpha"
    ),
}

# Constructors that are a transcendental wearing a type name. `glam` in this
# workspace is `["std", "scalar-math"]` with **no** `libm` feature, so every one
# of these resolves to the platform's `sin_cos`/`atan2`/`acos` exactly the way
# `f32::sin_cos` would - and until 2026-08-18 none of them was matched by
# anything here, because the two patterns above want a leading `.` or an
# `f32::`. `Quat::from_axis_angle` reached `Projectile::velocity` through
# `projectile::launch` for weeks in plain sight (finding D1 of that day's
# review). Matched on the bare name so a `math::Quat::from_axis_angle`, a
# `Quat::from_axis_angle` and a `use`d `from_axis_angle` all report.
#
# Deliberately absent, because they are `sqrt` and arithmetic and nothing else:
# `from_mat3`/`from_mat4`/`from_rotation_arc`/`from_cols`, `lerp`, `normalize`.
# A gate that flags clean code is a gate people learn to skip.
FORBIDDEN_ROTATIONS = (
    "from_axis_angle",
    "from_euler",
    "from_rotation_x",
    "from_rotation_y",
    "from_rotation_z",
    "from_scaled_axis",
    "to_axis_angle",
    "to_euler",
    "to_scaled_axis",
    "slerp",
    "angle_between",
    "angle_to",
    "rotate_towards",
)

METHOD_CALL = re.compile(r"\.\s*(" + "|".join(FORBIDDEN) + r")\s*\(")
PATH_CALL = re.compile(r"\bf(?:32|64)::\s*(" + "|".join(FORBIDDEN) + r")\s*\(")
ROTATION_CALL = re.compile(r"\b(" + "|".join(FORBIDDEN_ROTATIONS) + r")\s*\(")


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


TEST_MOD_DECL = re.compile(r"#\[cfg\(test\)\]\s*\n\s*mod\s+(\w+)\s*;")


def declared_test_files(root: Path) -> set[Path]:
    """Every file in `root` that is a `#[cfg(test)]` module's body, and its subtree.

    `crates/physics/src/airbrake/tests.rs` is what `check-file-size.py` asks a
    long test module to become, and it carries no `#[cfg(test)]` of its own for
    `strip_test_modules` to find. Same licence as an inline block: test code may
    call anything.

    Found by **reading the declaration**, not by trusting the file name. A rule
    of "any file called `tests.rs`" would hand the same licence to a future
    `src/lap_tests.rs` that is ordinary simulation code, and it would do it
    silently, which is the failure this script exists to stop happening twice.
    """
    found: set[Path] = set()
    for path in root.rglob("*.rs"):
        # A crate root or `mod.rs` declares its children in its own directory;
        # any other file declares them in the directory named after it.
        parent = path.parent if path.stem in ("lib", "main", "mod") else path.with_suffix("")
        for name in TEST_MOD_DECL.findall(path.read_text()):
            for candidate in (parent / f"{name}.rs", parent / name / "mod.rs"):
                if candidate.is_file():
                    found.add(candidate.resolve())
            subtree = parent / name
            if subtree.is_dir():
                found.update(child.resolve() for child in subtree.rglob("*.rs"))
    return found


def main() -> int:
    findings: list[str] = []
    scanned = 0
    test_files = 0

    for crate in SCANNED_CRATES:
        root = ROOT / "crates" / crate / "src"
        if not root.is_dir():
            print(f"no such crate directory: {root.relative_to(ROOT)}")
            return 1
        test_bodies = declared_test_files(root)
        for path in sorted(root.rglob("*.rs")):
            relative = path.relative_to(ROOT).as_posix()
            if relative in ALLOWED:
                continue
            if path.resolve() in test_bodies:
                test_files += 1
                continue
            scanned += 1
            source = strip_test_modules(strip_comments(path.read_text()))
            for number, line in enumerate(source.splitlines(), start=1):
                for pattern in (METHOD_CALL, PATH_CALL, ROTATION_CALL):
                    found = pattern.search(line)
                    if found:
                        findings.append(f"{relative}:{number}: {found.group(1)}")

    if findings:
        print("platform transcendentals in simulation code, or in what feeds it:\n")
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
        f"OK: no platform transcendentals in {scanned} scanned source file(s), "
        f"{len(ALLOWED)} allowed by name, {test_files} dedicated test file(s) skipped"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
