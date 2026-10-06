#!/usr/bin/env python3
"""Two ratchets on Rust source length: whole files, and the tests inside them.

`crates/raceplay/src/lib.rs` reached **11,294 lines** before anything measured it,
which is thirty times this tree's median file and more than twice its next
largest. Nothing was wrong with any single commit that grew it; that is the
failure mode a review cannot catch, because every diff was small. It is 795
lines now, and the twenty-four modules it split into are the worked example of
what these two rules ask for.

A plain limit is not adoptable here - 40 of 250 files were already over 1,000
lines when this landed, and failing the gate on all of them means turning the
gate off. So this is a **ratchet** instead:

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

Lines are counted whole, tests included - "total lines" is the one measure
nobody has to agree on the meaning of.

**The second rule is about what those lines are.** Counting whole files says
nothing about the shape of them, and the shape here was lopsided: of the 150
inline `#[cfg(test)]` modules in the tree on 2026-08-16, **55 were over 200
lines** and the largest was 3,429 - `race.rs` was 7,700 lines of engine followed
by a third of a file of tests, and `airbrake.rs` was 650 lines of tests behind
490 lines of force law, so the module you had to read to change one force was
three-quarters something else. Tests are worth every line; they are just not
worth being in the way of the code they test.

So an inline `#[cfg(test)] mod` body may not exceed `TEST_LIMIT`, and past that
it moves to a file of its own:

    // crates/physics/src/airbrake.rs
    #[cfg(test)]
    mod tests;

    // crates/physics/src/airbrake/tests.rs - what was inside the braces
    use super::*;

`super::*` still reaches everything private, so this is a move and not a
rewrite. Check that it was one, because a `mod tests;` that never landed looks
exactly like a green run with fewer tests in it:

- **A plain move** keeps every test's path, so `cargo nextest list -p <crate>`
  before and after must `diff` empty.
- **A module too big for one file** splits into `<module>/tests/<theme>.rs`, and
  that renames every path from `m::tests::<name>` to `m::tests::<theme>::<name>`
  - so a `nextest list` diff is 100% churn and proves nothing. Compare the
  **test function names** instead: `fn` names under `#[test]` in
  `git show HEAD:<file>`, sorted, against the leaf names in the new listing.
  Names do not move when modules do, which is what makes it the invariant.

Two things this rule deliberately does not do. It keys on `#[cfg(test)]` and
nothing else, so a `mod` of shared helpers colocates as freely as it ever did.
And **a dedicated test file is governed by `LIMIT` alone** - it has no
`#[cfg(test)]` block left inside it to measure, which is what makes the rule
satisfiable at all.

`TEST_BASELINE` exists to ratchet what could not be split at once, exactly as
`BASELINE` does for files. **It is empty, and that is the finding, not an
oversight**: all 55 were moved out in the change that added the rule, so this
one lands with no exemptions at all. Keep it that way. A row added here is a
statement that a test module could not be moved, and moving one is the most
mechanical refactor in this tree - `use super::*` reaches every private item, so
nothing needs rewriting to make it compile.

Clearing the 55 took 21,704 lines out of the files that held them and dropped 12
files off `BASELINE` outright, which is the second thing worth knowing: for a
file that was long because its tests were long, this rule *is* the split.

What it measures, and therefore what it misses: a `#[cfg(test)] mod NAME { ... }`
written out in the file. Free `#[cfg(test)] fn` items at file scope are not
counted, and neither is a `tests.rs` that got long again. Both are reachable
from here, and neither is worth guessing at before it happens - every test in
this tree today is inside a named module.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

# What a new file may not exceed.
LIMIT = 1_000

# What a `#[cfg(test)]` module sharing a file with the code it tests may not
# exceed. Its own file has no such ceiling - only `LIMIT`.
TEST_LIMIT = 200

# Every file still over `LIMIT`, at the size it measured on 2026-08-16 - after
# the test-module rule below took 21,704 lines out of them and graduated 12 of
# the original 40 outright. These are ceilings, not targets: each may shrink,
# none may grow, and a row is deleted the moment its file fits under `LIMIT`.
#
# **Neither `crates/raceplay/src/lib.rs` nor `crates/game/src/main.rs` is here any
# more, and that is what this script was written for.** `race.rs` motivated the
# ratchet at 11,294 lines, dropped to 7,751 when its test module moved out, and
# was split along its own seams on 2026-08-16 into `race.rs` plus twenty-four
# modules under `crates/raceplay/src/`. `main.rs` followed it the same day:
# 4,808 lines into 430 plus eighteen modules under `crates/game/src/main/`.
# Both splits were move-only and left nothing over 800 lines. The largest thing
# left is `crates/game/tests/race_ground_truth.rs`.
BASELINE = {
    # Ratcheted down from 2,387 on 2026-09-02, when
    # `a_ship_spawns_on_the_track_and_flies_along_it` moved to
    # `crates/game/tests/ship_spawn_ground_truth.rs`. The move was forced the
    # right way round: a disc-backed fix needed twelve lines in a file already
    # at its ceiling, and a ceiling only ever lowers.
    "crates/game/tests/race_ground_truth.rs": 2266,
    "crates/vex/src/vex.rs": 1651,
    # Ratcheted down from 1,719 when `Options`/`impl Options` moved to
    # `crates/game/src/boot/options.rs`, split out for the `platform` field
    # the title+platform render-profile split (`crates/game/src/settings.rs`)
    # needed on `Boot`/`Shell` - a move, with room left for it.
    "crates/game/src/boot.rs": 1661,
    # Ratcheted down from 1,981 when `Player`, `FRAME_RATE` and
    # `PS2_DISPLAY_ASPECT` moved to `crates/game/src/frontend/player.rs` -
    # prep for the `oag-ui` extraction, which cannot name a type that lives in
    # the composition root.
    "crates/game/src/movie.rs": 1756,
    # Ratcheted down from 2,005 when the two drawing idioms split out into
    # `menu/rows.rs` and `menu/strip.rs`; `draw_list` picks between them and
    # draws nothing itself. Ratcheted down again from 1,785 to 1,276 when
    # `Definition`, `Error`, the raw TOML shape and the parse/check functions
    # split out into `menu/definition.rs` - the seam `string_id` resolution
    # needs, and the file had zero lines of headroom to grow it in place.
    "crates/ui/src/menu.rs": 1201,
    # Ratcheted down from 1,956 to 1,782 when the draw pipeline moved out into
    # `psys/pipeline.rs`, making room for the sprite sheet and the emitter
    # extent, which live in `psys/sprite.rs` and `psys/spawn.rs`; to 1,781
    # when the streak strips moved into `psys/streak.rs`; to 1,740 when
    # `Particle` moved into `psys/particle.rs` to make room for the rotating
    # template sprite's roll.
    "crates/fx/src/psys.rs": 1549,
    "crates/sound/src/lib.rs": 1771,
    "crates/trace/src/main.rs": 1543,
    # Ratcheted down from 1,506 when `FlareTexture` moved out into
    # `exhaust/texture.rs`, which is where the ribbon's second texture is
    # uploaded from.
    "crates/fx/src/exhaust.rs": 1430,
    "crates/physics/tests/ship_dynamics.rs": 1262,
    "crates/display/src/display.rs": 1043,
    "crates/formats/tests/audio_ground_truth.rs": 1226,
    "crates/view/src/main.rs": 1152,
    "crates/ai/tests/closed_loop.rs": 1146,
    "crates/trace/src/compare.rs": 1038,
}

# Inline `#[cfg(test)]` modules over `TEST_LIMIT`, keyed `<file>::<module>`.
# **Empty on purpose**: all 55 that existed were moved out in the change that
# added the rule. `crates/physics/src/airbrake.rs` and its
# `crates/physics/src/airbrake/tests.rs` are the worked example of the move.
#
# Adding a row here says a test module cannot be moved into its own file. Before
# you believe that, try it - it is `#[cfg(test)] mod tests;` in place of the
# block and the body dedented one level into a sibling file, and `use super::*`
# still reaches every private item.
TEST_BASELINE: dict[str, int] = {}

CFG_TEST = re.compile(r"(\s*)#\[cfg\(test\)\]\s*$")
TEST_MOD = re.compile(r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*\{\s*$")


def lines_in(path: Path) -> int:
    with path.open("rb") as handle:
        return sum(1 for _ in handle)


def test_modules(lines: list[str]) -> dict[str, int]:
    """Measure every inline `#[cfg(test)]` module in one file, by name.

    The end of a module is found by **indentation** - the first line that is
    exactly the `#[cfg(test)]`'s own indentation followed by `}` - and not by
    counting braces. This tree is full of raw strings holding Windows paths and
    of WGSL in `include_str!`-shaped literals, and a gate that mismeasures is
    worse than no gate. Both methods were run over all 150 modules on the day
    this landed and agreed everywhere; indentation is the one that stays right
    when a brace turns up inside a string.

    A module declared rather than written out (`mod tests;`) has no body here to
    measure, which is exactly the state this rule asks for.
    """
    found: dict[str, int] = {}
    for index, line in enumerate(lines):
        marker = CFG_TEST.match(line)
        if marker is None:
            continue
        indent = marker.group(1)
        head = index + 1
        while head < len(lines) and lines[head].lstrip().startswith("#["):
            head += 1
        if head >= len(lines):
            continue
        opened = TEST_MOD.match(lines[head])
        if opened is None:
            continue
        closing = f"{indent}}}"
        for end in range(head + 1, len(lines)):
            if lines[end] == closing:
                found[opened.group(1)] = end - index + 1
                break
    return found


class Ratchet:
    """One measurement set against one baseline, sorted into the four verdicts."""

    def __init__(self, measured: dict[str, int], baseline: dict[str, int], limit: int):
        self.over: list[str] = []
        self.grown: list[str] = []
        self.graduated: list[str] = []
        self.vanished: list[str] = []

        for name, count in measured.items():
            ceiling = baseline.get(name)
            if ceiling is None:
                if count > limit:
                    self.over.append(f"{name}: {count} lines, limit is {limit}")
            elif count > ceiling:
                self.grown.append(
                    f"{name}: {count} lines, ceiling is {ceiling} (+{count - ceiling})"
                )
            elif count <= limit:
                self.graduated.append(f"{name}: {count} lines, now under {limit}")

        for name in baseline:
            if name not in measured:
                self.vanished.append(name)

        self.slack = sum(baseline[name] - measured[name] for name in baseline if name in measured)

    def failed(self) -> bool:
        return bool(self.over or self.grown or self.graduated or self.vanished)


def report(heading: str, rows: list[str], advice: str) -> None:
    if not rows:
        return
    print(f"\n{heading}\n")
    for row in rows:
        print(f"  {row}")
    print(f"\n{advice}")


def main() -> int:
    files: dict[str, int] = {}
    modules: dict[str, int] = {}
    for path in sorted((ROOT / "crates").rglob("*.rs")):
        relative = path.relative_to(ROOT).as_posix()
        files[relative] = lines_in(path)
        text = path.read_text(encoding="utf-8")
        for module, count in test_modules(text.split("\n")).items():
            modules[f"{relative}::{module}"] = count

    size = Ratchet(files, BASELINE, LIMIT)
    tests = Ratchet(modules, TEST_BASELINE, TEST_LIMIT)

    report(
        f"new file(s) over {LIMIT} lines:",
        size.over,
        "Split it along whatever seam it already has. If it genuinely cannot be "
        "split\nyet, add it to BASELINE in this script at its current size and "
        "say why in the\ncommit - that is a decision, not a formality.",
    )
    report(
        "baselined file(s) that grew:",
        size.grown,
        "These are ceilings, and a ceiling is lowered, never raised. The file "
        "was already\npast the limit before this change; put the new code "
        "somewhere else, or split the\nfile now and lower its row.",
    )
    report(
        "baselined file(s) that now fit - delete their rows:",
        size.graduated,
        "The baseline only ever gets shorter. Leaving a row here keeps an exemption alive.",
    )
    report(
        "BASELINE row(s) naming a file that no longer exists:",
        size.vanished,
        "Delete them, or a rename carries the old exemption forward under a new name.",
    )

    report(
        f"inline #[cfg(test)] module(s) over {TEST_LIMIT} lines:",
        tests.over,
        "Move the body to its own file: `#[cfg(test)] mod tests;` where the "
        "block was, and\n`<module>/tests.rs` beside it holding what was inside "
        "the braces - `use super::*;`\nstill reaches every private item, so it "
        "is a move and not a rewrite. Check it with\n`cargo nextest list -p "
        "<crate>` either side: a declaration that never landed looks\nexactly "
        "like a green run with fewer tests in it.",
    )
    report(
        "baselined test module(s) that grew:",
        tests.grown,
        "A ceiling is lowered, never raised. Put the new test in the file its "
        "module is\nheaded for, and start the split there.",
    )
    report(
        f"baselined test module(s) now under {TEST_LIMIT} - delete their rows:",
        tests.graduated,
        "The baseline only ever gets shorter. Leaving a row here keeps an exemption alive.",
    )
    report(
        "TEST_BASELINE row(s) naming a module that is no longer there:",
        tests.vanished,
        "Delete them - the module was split, renamed or moved out, and the row "
        "would carry\nits exemption to whatever takes the name next.",
    )

    if size.failed() or tests.failed():
        return 1

    worst = max(files.items(), key=lambda row: row[1])
    worst_test = max(modules.items(), key=lambda row: row[1])
    print(
        f"OK: {len(files)} Rust file(s), none over {LIMIT} lines except "
        f"{len(BASELINE)} baselined; worst is {worst[0]} at {worst[1]}"
        + (f"; {size.slack} line(s) already clawed back" if size.slack else "")
    )
    print(
        f"OK: {len(modules)} inline test module(s), none over {TEST_LIMIT} "
        f"lines except {len(TEST_BASELINE)} baselined; worst is "
        f"{worst_test[0]} at {worst_test[1]}"
        + (f"; {tests.slack} line(s) already clawed back" if tests.slack else "")
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
