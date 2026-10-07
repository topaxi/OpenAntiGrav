#!/usr/bin/env python3
"""A ratchet on generic crates reaching into a title package by path.

ADR-0058 moved per-title behaviour into `oag_title::Title` data, and
`check-title-branching.py` guards the branches. This guards the other half of the
same coupling: a generic crate that names `oag_pulse::hud::ART`,
`oag_hd::campaign::SCREEN_ENTRY` and the like, so that it cannot be built, tested or
read without that title's crate. The end state is that generic crates depend on
`oag-title` only, take a title as a `&'static Title` (or a value off it), and reach
the title packages from the composition root alone, so `check-deps` can then
forbid the Cargo edge.

It is a ratchet in `check-title-branching.py`'s shape, over a per-file `BASELINE`:

- a file may have **at most** the count recorded for it, and may drop freely;
- a file not in `BASELINE` may have **none**, so a new file cannot appear;
- a file below its row prints a hint to lower it, and a row that reached zero (or
  names a file that no longer exists) must be deleted. A ceiling is lowered, never
  raised.

What counts: a line of non-comment code in a generic crate containing
`oag_(pulse|pure|hd|omega|2048)::`. Comments and doc comments are not counted (they
name no dependency). Skipped, as in `check-title-branching.py`: any file under a
`tests/`, `examples/` or `benches/` directory, any `tests.rs`, and
`#[cfg(test)] mod NAME { ... }` bodies. Also skipped: files named `*_tests.rs`,
which are `#[cfg(test)] mod` files declared elsewhere.

Allowed reach points, never counted:

- the title packages themselves (`oag-pulse`, `oag-pure`, `oag-hd`, `oag-omega`,
  `oag-2048`) and `oag-title`, which is where a title's data is typed;
- the registry in `crates/source/src/title.rs`, the one place a disc image becomes
  a `Title`;
- `crates/game/src/main.rs` (with `main_body.rs`, which it and `android.rs`
  `include!`) and `crates/game/src/bin/`, the composition root's entry.

`python3 scripts/check-title-reach.py --list` prints every counted site.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

TITLE_PACKAGES = {"pulse", "pure", "hd", "omega", "2048", "title"}
SKIPPED_DIRS = {"tests", "examples", "benches"}
ALLOWED_FILES = {
    "crates/source/src/title.rs",
    "crates/game/src/main.rs",
    "crates/game/src/main_body.rs",
    "crates/game/src/android.rs",
}
ALLOWED_PREFIXES = ("crates/game/src/bin/",)

REACH = re.compile(r"\boag_(?:pulse|pure|hd|omega|2048)::")

# `oag_<title>::` path references per file at the 2026-10-06 landing. Lower a row
# when a file drops; delete it at zero. Never raise one.
BASELINE: dict[str, int] = {
    "crates/game/src/boot.rs": 2,
    "crates/game/src/boot/campaign2048.rs": 27,
    "crates/game/src/boot/fury.rs": 1,
    "crates/game/src/boot/movies.rs": 1,
    "crates/game/src/boot/roster.rs": 2,
    "crates/game/src/boot/sprites.rs": 1,
    "crates/game/src/campaign.rs": 33,
    "crates/game/src/capture/campaign_page.rs": 6,
    "crates/game/src/capture/card.rs": 1,
    "crates/game/src/capture/endrace_touch_page.rs": 1,
    "crates/game/src/capture/menu_page.rs": 4,
    "crates/game/src/endrace.rs": 9,
    "crates/game/src/endrace/touch.rs": 6,
    "crates/game/src/loading.rs": 1,
    "crates/game/src/loading/assets.rs": 1,
    "crates/game/src/main/args.rs": 1,
    "crates/game/src/main/campaign_stage.rs": 3,
    "crates/game/src/main/race_stage.rs": 4,
    "crates/game/src/main/race_stage/hd_loyalty.rs": 1,
    "crates/game/src/main/session/campaign.rs": 1,
    "crates/game/src/main/session/pilot_editor.rs": 4,
    "crates/game/src/main/session/placeholder.rs": 6,
    "crates/game/src/prefetch.rs": 1,
    "crates/game/src/preview.rs": 1,
    "crates/game/src/settings/render_profile.rs": 5,
    "crates/livery/src/absorb.rs": 2,
    "crates/livery/src/entry.rs": 2,
    "crates/livery/src/flare.rs": 1,
    "crates/livery/src/shield.rs": 1,
    "crates/livery/src/ship_skin.rs": 1,
    "crates/livery/src/wreck.rs": 1,
    "crates/raceplay/src/assets.rs": 1,
    "crates/raceplay/src/effects.rs": 4,
    "crates/raceplay/src/hud.rs": 2,
    "crates/raceplay/src/lib.rs": 2,
    "crates/raceplay/src/load/campaign.rs": 19,
    "crates/raceplay/src/load/track_stats.rs": 1,
    "crates/raceplay/src/options.rs": 1,
    "crates/raceplay/src/scene/frame/shadow.rs": 2,
    "crates/raceplay/src/track_panel.rs": 4,
    "crates/render/src/loading.rs": 1,
    "crates/source/src/source.rs": 1,
    "crates/ui-screens/src/campaign/selection.rs": 3,
    "crates/ui-screens/src/track_panel.rs": 4,
    "crates/ui/src/frontend.rs": 6,
    "crates/ui/src/frontend/campaign_map.rs": 2,
    "crates/ui/src/frontend/event_card.rs": 1,
    "crates/ui/src/frontend/options2048.rs": 1,
    "crates/ui/src/frontend/team.rs": 2,
    "crates/ui/src/frontend/touch.rs": 1,
    "crates/ui/src/frontend/wipeout2048.rs": 1,
}


def code_part(line: str) -> str:
    stripped = line.lstrip()
    if stripped.startswith("//"):
        return ""
    return re.sub(r'\s//[^"]*$', "", line)


def counted_lines(path: Path) -> list[tuple[int, str]]:
    found: list[tuple[int, str]] = []
    skip_depth = 0
    pending_cfg_test = False
    in_block_comment = False
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = code_part(raw)
        if in_block_comment:
            if "*/" in line:
                in_block_comment = False
            continue
        if line.lstrip().startswith("/*") and "*/" not in line:
            in_block_comment = True
            continue
        if skip_depth:
            skip_depth += line.count("{") - line.count("}")
            continue
        if pending_cfg_test and re.match(r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+\w+\s*\{", line):
            skip_depth = line.count("{") - line.count("}")
            pending_cfg_test = False
            continue
        if line.strip().startswith("#[cfg(test)]"):
            pending_cfg_test = True
            continue
        if line.strip() and not line.strip().startswith("#["):
            pending_cfg_test = False
        if REACH.search(line):
            found.append((number, raw.strip()))
    return found


def generic_sources() -> list[Path]:
    out = []
    for path in sorted((ROOT / "crates").glob("*/**/*.rs")):
        rel = path.relative_to(ROOT / "crates")
        if rel.parts[0] in TITLE_PACKAGES:
            continue
        if SKIPPED_DIRS & set(rel.parts[1:-1]):
            continue
        if path.name == "tests.rs" or path.name.endswith("_tests.rs"):
            continue
        posix = path.relative_to(ROOT).as_posix()
        if posix in ALLOWED_FILES or posix.startswith(ALLOWED_PREFIXES):
            continue
        out.append(path)
    return out


def main() -> int:
    measured: dict[str, list[tuple[int, str]]] = {}
    for path in generic_sources():
        hits = counted_lines(path)
        if hits:
            measured[path.relative_to(ROOT).as_posix()] = hits

    if "--list" in sys.argv:
        for rel, hits in measured.items():
            for number, text in hits:
                print(f"{rel}:{number}: {text}")
        print(f"{sum(len(h) for h in measured.values())} site(s) in {len(measured)} file(s)")
        return 0

    if "--by-crate" in sys.argv:
        per: dict[str, int] = {}
        for rel, hits in measured.items():
            per[rel.split("/")[1]] = per.get(rel.split("/")[1], 0) + len(hits)
        for name, count in sorted(per.items(), key=lambda kv: -kv[1]):
            print(f"{count:5} oag-{name}")
        return 0

    failures: list[str] = []
    hints: list[str] = []
    for rel, hits in measured.items():
        allowed = BASELINE.get(rel, 0)
        if len(hits) > allowed:
            where = ", ".join(str(n) for n, _ in hits)
            failures.append(
                f"{rel}: {len(hits)} title-crate path reference(s), baseline {allowed} (lines {where})"
            )
        elif len(hits) < allowed:
            hints.append(f"{rel}: {len(hits)} now, baseline {allowed} - lower the row to {len(hits)}")
    for rel in sorted(BASELINE):
        if rel not in measured:
            hints.append(f"{rel}: none left or file gone - delete its BASELINE row")

    for hint in hints:
        print(f"hint: {hint}")
    if failures:
        print("generic crates may not reach into a title package; take a `&Title` field instead (ADR-0058):")
        for failure in failures:
            print(f"  {failure}")
        return 1
    print(f"title reach: {sum(len(h) for h in measured.values())} reference(s) in {len(measured)} file(s), within baseline")
    return 0


if __name__ == "__main__":
    sys.exit(main())
