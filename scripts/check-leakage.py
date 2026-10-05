#!/usr/bin/env python3
"""Assert no game content is tracked by git, and that this is the only place
the list of forbidden extensions has to be kept accurate.

Finding 1 of the 2026-07-30 code review: `--screenshot out.png` (documented in
CLAUDE.md) and the movie cache's `.ivf`/`.h264`/`.ipu`/`.pss` demux writes were
not covered by any enforcement layer, and the three lists that used to encode
this check by hand (.gitignore, ci.yml, justfile) had already drifted from
each other (`.umd`/`.gcm` were gitignored but absent from both audits). This
script is now the single source of truth: `just audit-leakage` and CI's
`leakage` job both just run it, and it also checks .gitignore for the same
list, so there is nothing left to hand-copy out of sync.
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

# Every extension that names game content itself, or a reproduction this
# project generates from it (a screenshot, the movie cache's demuxed streams,
# extracted asset formats). See docs/overview/legal.md.
EXTENSIONS = [
    # Disc images and packaged formats
    "chd", "iso", "cso", "pkg", "pbp", "umd", "gcm", "vpk",
    # PSP/PS2 executables and containers
    "wad", "elf", "prx", "self", "bin", "img", "edat",
    # PS3/Vita containers and modules. HD/Fury and 2048 ship PSARC archives
    # rather than WADs, and Vita modules are `.suprx`/`.skprx`. `dkey` is the
    # PS3 disc key: not content itself, but the thing that decrypts it.
    "psarc", "sprx", "suprx", "skprx", "dkey",
    # Emulator save states
    "ppst", "p2s", "state",
    # Rendered/extracted reproductions this project itself writes
    "png", "ivf", "h264", "ipu", "pss", "mip", "vex", "fnt", "pmf",
]

# Small, hand-authored fixtures that legitimately carry a forbidden extension
# because they are synthetic, not extracted content. See legal.md's "Test
# fixtures" section.
ALLOWED_TRACKED = {
    "crates/video/tests/data/testsrc-64x64.ivf",
    # SMAA's own precomputed AreaTex/SearchTex, MIT-licensed - see
    # crates/post/src/smaa.rs's module docs and licences/SMAA-MIT.txt.
    "crates/post/src/smaa_area.bin",
    "crates/post/src/smaa_search.bin",
}

ROOT = Path(__file__).resolve().parent.parent


def tracked_leaks() -> list[str]:
    pattern = re.compile(r"\.(" + "|".join(EXTENSIONS) + r")$", re.IGNORECASE)
    result = subprocess.run(
        ["git", "ls-files"], cwd=ROOT, capture_output=True, text=True, check=True
    )
    return [
        path
        for path in result.stdout.splitlines()
        if pattern.search(path) and path not in ALLOWED_TRACKED
    ]


def gitignore_gaps() -> list[str]:
    """Extensions with no `*.ext` line anywhere in .gitignore."""
    text = (ROOT / ".gitignore").read_text()
    covered = set(re.findall(r"^\*\.([A-Za-z0-9]+)$", text, re.MULTILINE))
    return [ext for ext in EXTENSIONS if ext not in covered]


def main() -> int:
    problems = []

    leaks = tracked_leaks()
    if leaks:
        problems.append(
            "game content tracked by git:\n" + "\n".join(f"  {p}" for p in leaks)
        )

    gaps = gitignore_gaps()
    if gaps:
        problems.append(
            "extensions missing a `*.ext` line in .gitignore: " + ", ".join(gaps)
        )

    if problems:
        print("\n\n".join(problems), file=sys.stderr)
        return 1

    print(f"OK: no game content tracked, {len(EXTENSIONS)} extensions covered by .gitignore")
    return 0


if __name__ == "__main__":
    sys.exit(main())
