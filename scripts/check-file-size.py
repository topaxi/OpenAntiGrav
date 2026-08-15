#!/usr/bin/env python3
"""A ratchet on Rust source-file length: nothing new over the limit, nothing big grows.

`crates/game/src/race.rs` reached **11,294 lines** before anything measured it,
which is thirty times this tree's median file and more than twice its next
largest. Nothing was wrong with any single commit that grew it; that is the
failure mode a review cannot catch, because every diff was small.

A plain limit is not adoptable here - 40 of 250 files are already over 1,000
lines, and failing the gate on all of them means turning the gate off. So this
is a **ratchet** instead:

- a file not in `BASELINE` may not exceed `LIMIT`;
- a file in `BASELINE` may not exceed **the size recorded there**, which is what
  it measured on the day the ratchet landed. It may shrink freely;
- once a baselined file fits under `LIMIT`, its row must be deleted, so the
  baseline can only get shorter;
- a row naming a file that no longer exists must be deleted too, or a rename
  would silently carry an exemption forward.

**A ceiling is lowered, never raised.** Raising one is how a ratchet becomes a
record of what happened, which is what the absence of this script already was.
The lint is deliberately dumb about *why* a file is long: splitting one is a
judgement, and the gate's job is only to make the judgement happen at 1,000
lines instead of at 11,000.

Lines are counted whole, tests included. A file's tests move with it when it is
split, and "total lines" is the one measure nobody has to agree on the meaning
of.
"""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# What a new file may not exceed.
LIMIT = 1_000

# Every file already over `LIMIT` when this landed, at the size it was
# (2026-08-15). These are ceilings, not targets: each may shrink, none may grow,
# and a row is deleted the moment its file fits under `LIMIT`.
#
# `crates/game/src/race.rs` is the one that motivated the script and is the
# highest-priority split in the tree - it is four things (asset loading, the
# race, the renderer composition, headless capture) with visible seams. See
# HANDOVER.md's open thread.
BASELINE = {
    "crates/game/src/race.rs": 11294,
    "crates/game/src/main.rs": 4923,
    "crates/game/src/menu.rs": 4095,
    "crates/formats/src/vex.rs": 3517,
    "crates/game/src/frontend.rs": 3229,
    "crates/game/src/movie.rs": 2798,
    "crates/game/src/audio.rs": 2748,
    "crates/ai/src/driver.rs": 2565,
    "crates/game/tests/race_ground_truth.rs": 2464,
    "crates/game/src/boot.rs": 2254,
    "crates/game/src/hud.rs": 2233,
    "crates/formats/src/handling.rs": 2065,
    "crates/render/src/exhaust.rs": 2053,
    "crates/render/src/psys.rs": 1956,
    "crates/trace/src/trace.rs": 1940,
    "crates/game/src/display.rs": 1839,
    "crates/physics/src/hover.rs": 1817,
    "crates/physics/src/wall.rs": 1737,
    "crates/trace/src/main.rs": 1616,
    "crates/trace/src/replay.rs": 1578,
    "crates/trace/src/compare.rs": 1493,
    "crates/formats/src/collision.rs": 1470,
    "crates/physics/src/engine.rs": 1420,
    "crates/formats/src/pob.rs": 1383,
    "crates/physics/tests/ship_dynamics.rs": 1366,
    "crates/render/src/loading.rs": 1363,
    "crates/physics/src/forces.rs": 1327,
    "crates/gameplay/src/projectile.rs": 1321,
    "crates/game/src/loading.rs": 1320,
    "crates/render/src/mesh_render.rs": 1287,
    "crates/render/src/mesh.rs": 1260,
    "crates/formats/tests/audio_ground_truth.rs": 1226,
    "crates/view/src/main.rs": 1170,
    "crates/ai/tests/closed_loop.rs": 1146,
    "crates/physics/src/airbrake.rs": 1146,
    "crates/game/src/font.rs": 1094,
    "crates/game/src/upscale.rs": 1082,
    "crates/game/src/settings.rs": 1079,
    "crates/game/src/render.rs": 1055,
    "crates/formats/src/track.rs": 1005,
}


def lines_in(path: Path) -> int:
    with path.open("rb") as handle:
        return sum(1 for _ in handle)


def main() -> int:
    measured: dict[str, int] = {}
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        measured[path.relative_to(ROOT).as_posix()] = lines_in(path)

    grown: list[str] = []
    over: list[str] = []
    graduated: list[str] = []
    vanished: list[str] = []

    for name, count in measured.items():
        ceiling = BASELINE.get(name)
        if ceiling is None:
            if count > LIMIT:
                over.append(f"{name}: {count} lines, limit is {LIMIT}")
        elif count > ceiling:
            grown.append(f"{name}: {count} lines, ceiling is {ceiling} (+{count - ceiling})")
        elif count <= LIMIT:
            graduated.append(f"{name}: {count} lines, now under {LIMIT}")

    for name in BASELINE:
        if name not in measured:
            vanished.append(name)

    if over:
        print(f"new file(s) over {LIMIT} lines:\n")
        for line in over:
            print(f"  {line}")
        print(
            "\nSplit it along whatever seam it already has. If it genuinely "
            "cannot be split\nyet, add it to BASELINE in this script at its "
            "current size and say why in the\ncommit - that is a decision, not "
            "a formality."
        )

    if grown:
        print(f"\nbaselined file(s) that grew:\n")
        for line in grown:
            print(f"  {line}")
        print(
            "\nThese are ceilings, and a ceiling is lowered, never raised. The "
            "file was already\npast the limit before this change; put the new "
            "code somewhere else, or split the\nfile now and lower its row."
        )

    if graduated:
        print("\nbaselined file(s) that now fit - delete their rows:\n")
        for line in graduated:
            print(f"  {line}")
        print("\nThe baseline only ever gets shorter. Leaving a row here keeps an exemption alive.")

    if vanished:
        print("\nBASELINE row(s) naming a file that no longer exists:\n")
        for name in vanished:
            print(f"  {name}")
        print("\nDelete them, or a rename carries the old exemption forward under a new name.")

    if over or grown or graduated or vanished:
        return 1

    worst = max(measured.items(), key=lambda row: row[1])
    slack = sum(BASELINE[name] - measured[name] for name in BASELINE)
    print(
        f"OK: {len(measured)} Rust file(s), none over {LIMIT} lines except "
        f"{len(BASELINE)} baselined; worst is {worst[0]} at {worst[1]}"
        + (f"; {slack} line(s) already clawed back" if slack else "")
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
