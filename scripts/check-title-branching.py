#!/usr/bin/env python3
"""A ratchet on title-identity comparisons in generic crates.

What a title *ships* is data (`oag_title::Title`, ADR-0022/0023/0025). What a
title *does differently* was, by 2026-10-06, mostly `title.name ==
oag_hd::TITLE.name` in a crate that is not that title's: about 42 non-test
sites, and the unstated rule behind them is "Pulse's law is the default and
everyone else opts out by branch", which records no provenance. ADR-0058 decides
the replacement (per-title behaviour is `Title` data with a provenance tag).
This script is its guard, and it does **not** migrate anything.

It is a ratchet in `check-file-size.py`'s shape, over a per-file `BASELINE` of
the comparisons that existed the day it landed:

- a file may have **at most** the count recorded for it, and may drop freely;
- a file not in `BASELINE` may have **none**, so a new file cannot appear;
- a file that dropped below its row prints a hint to lower the row, and a row
  that reached zero (or names a file that no longer exists) must be deleted, so
  the baseline can only shrink. A ceiling is lowered, never raised.

What counts, per line of non-test, non-comment code in a generic crate:

- `X == oag_<title>::TITLE.name` and the reverse, with `==` or `!=`;
- a comparison against a literal title name (`"Wipeout ..."`);
- `title.name` or `title_name` on the left of `==`/`!=`;
- `match title.name {`.

Generic crates are every crate except the title packages (`oag-pulse`,
`oag-pure`, `oag-hd`, `oag-omega`, `oag-2048`, `oag-title`). Skipped: any file
under a `tests/`, `examples/` or `benches/` directory, any `tests.rs`, and
`#[cfg(test)] mod NAME { ... }` bodies. Not counted on purpose: a lookup that
matches a survey entry against a runtime-chosen name (`c.title() == requested`,
`theirs.name != title.name`) selects data and branches on no title. A comparison
split across two lines is not seen; `--list` shows what is counted.

`python3 scripts/check-title-branching.py --list` prints every counted site.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent

TITLE_PACKAGES = {"pulse", "pure", "hd", "omega", "2048", "title"}
SKIPPED_DIRS = {"tests", "examples", "benches"}

CMP = r"(?:==|!=)"
PATTERNS = [
    re.compile(rf"{CMP}\s*oag_\w+::TITLE\.name|oag_\w+::TITLE\.name\s*{CMP}"),
    re.compile(rf'{CMP}\s*"Wipeout[^"]*"|"Wipeout[^"]*"\s*{CMP}'),
    re.compile(
        rf"\btitle(?:_ref)?\.name\s*{CMP}|\btitle_name\s*{CMP}"
    ),
    re.compile(r"\bmatch\s+[\w.()]*title[\w.()]*name\b"),
]

# Comparisons per file at the 2026-10-06 landing (main b115ec3ef). Lower a row
# when a file drops; delete it at zero. Never raise one.
BASELINE: dict[str, int] = {
    "crates/game/src/boot.rs": 1,
    "crates/game/src/boot/movies.rs": 2,
    "crates/game/src/campaign.rs": 3,
    "crates/game/src/main/args.rs": 2,
    "crates/game/src/main/session/campaign.rs": 1,
    "crates/game/src/main/session/remix.rs": 6,
    "crates/game/src/settings/race.rs": 1,
    "crates/game/src/unlock.rs": 2,
    "crates/source/src/title.rs": 1,
    "crates/ui/src/frontend.rs": 1,
}


def code_part(line: str) -> str:
    stripped = line.lstrip()
    if stripped.startswith("//"):
        return ""
    return re.sub(r'\s//[^"]*$', "", line)


def counted_lines(path: Path) -> list[tuple[int, str]]:
    """Lines of `path` outside `#[cfg(test)] mod { }` bodies that match."""
    found: list[tuple[int, str]] = []
    skip_depth = 0
    pending_cfg_test = False
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = code_part(raw)
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
        if any(p.search(line) for p in PATTERNS):
            found.append((number, raw.strip()))
    return found


def generic_sources() -> list[Path]:
    out = []
    for path in sorted((ROOT / "crates").glob("*/**/*.rs")):
        rel = path.relative_to(ROOT / "crates")
        if rel.parts[0] in TITLE_PACKAGES:
            continue
        if SKIPPED_DIRS & set(rel.parts[1:-1]) or path.name == "tests.rs":
            continue
        out.append(path)
    return out


def main() -> int:
    measured: dict[str, list[tuple[int, str]]] = {}
    for path in generic_sources():
        hits = counted_lines(path)
        if hits:
            measured[str(path.relative_to(ROOT))] = hits

    if "--list" in sys.argv:
        for rel, hits in measured.items():
            for number, text in hits:
                print(f"{rel}:{number}: {text}")
        print(f"{sum(len(h) for h in measured.values())} site(s) in {len(measured)} file(s)")
        return 0

    failures: list[str] = []
    hints: list[str] = []
    for rel, hits in measured.items():
        allowed = BASELINE.get(rel, 0)
        if len(hits) > allowed:
            where = ", ".join(str(n) for n, _ in hits)
            failures.append(
                f"{rel}: {len(hits)} title-identity comparison(s), baseline {allowed} (lines {where})"
            )
        elif len(hits) < allowed:
            hints.append(f"{rel}: {len(hits)} now, baseline {allowed} - lower the row to {len(hits)}")
    for rel in sorted(BASELINE):
        if rel not in measured:
            hints.append(f"{rel}: none left or file gone - delete its BASELINE row")

    for hint in hints:
        print(f"hint: {hint}")
    if failures:
        print("title-branching ratchet: a generic crate compares a title's identity.")
        print("Per-title behaviour is `Title` data (ADR-0058); do not add a branch:")
        for failure in failures:
            print(f"  {failure}")
        return 1
    if hints:
        return 1
    total = sum(len(h) for h in measured.values())
    print(f"check-title-branching: ok ({total} baselined site(s) in {len(measured)} file(s))")
    return 0


if __name__ == "__main__":
    sys.exit(main())
