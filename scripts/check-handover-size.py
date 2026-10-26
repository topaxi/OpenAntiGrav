#!/usr/bin/env python3
"""`HANDOVER.md` may not exceed 256 KiB, and every `handover/` thread needs frontmatter.

The second check (`scripts/handover-threads.py` owns the rules): a thread
with missing or invalid `status`, `titles`, `updated` or `summary` fails.

The limit is not arbitrary: it is the `Read` tool's own ceiling. A fresh
session opens this file before anything else in the workflow this project
runs on, and past 262,144 bytes that first read fails outright rather than
returning a truncated file - the session has to fall back to `offset`/`limit`
before it has read enough of the file to know what to ask for. `HANDOVER.md`
hit 321,071 bytes on 2026-08-18 and did exactly that, on the very session
sent to fix it.

This is a hard ceiling, not a ratchet like `check-file-size.py`'s `BASELINE`.
There is no file this applies to but one, so there is nothing to grandfather
and nothing to graduate - `HANDOVER.md` is either under the limit or it is
not.

Fixing an overage means pruning content, not code: delete threads whose work
has landed (the file's own header rule - "a row whose work landed is deleted,
not annotated"), and compress narrative in the biggest `## Open threads`
table rows down to current state plus whatever is still open, pointing at
the docs page that already carries the evidence instead of repeating it here.
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
TARGET = ROOT / "HANDOVER.md"

# The `Read` tool's own ceiling - see the module docstring for why this
# number and not a round one.
LIMIT = 256 * 1024


def check_threads() -> int:
    spec = importlib.util.spec_from_file_location(
        "handover_threads", ROOT / "scripts" / "handover-threads.py"
    )
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    bad = 0
    paths = mod.threads()
    for path in paths:
        _, errors = mod.parse(path)
        for e in errors:
            print(f"{path.relative_to(ROOT)}: {e}")
            bad += 1
    if bad:
        print(f"\n{bad} frontmatter problem(s). See the header of scripts/handover-threads.py.")
        return 1
    print(f"OK: {len(paths)} handover threads have valid frontmatter")
    return 0


def main() -> int:
    if check_threads():
        return 1
    if not TARGET.is_file():
        print(f"{TARGET} not found")
        return 1

    size = TARGET.stat().st_size
    if size > LIMIT:
        over = size - LIMIT
        print(
            f"HANDOVER.md is {size:,} bytes, {over:,} over the {LIMIT:,}-byte limit.\n\n"
            "That limit is the Read tool's own ceiling, not a style preference - past it "
            "a fresh\nsession's first read of this file fails outright. Prune it: delete "
            "threads whose work\nhas landed rather than annotating them, and compress "
            "narrative in the largest\n`## Open threads` rows down to current state plus "
            "what is still open, pointing at\nthe docs page that already carries the "
            "evidence instead of repeating it here."
        )
        return 1

    headroom = LIMIT - size
    print(f"OK: HANDOVER.md is {size:,} bytes, {headroom:,} under the {LIMIT:,}-byte limit")
    return 0


if __name__ == "__main__":
    sys.exit(main())
