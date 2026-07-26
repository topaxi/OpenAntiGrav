#!/usr/bin/env python3
"""Verify that every relative link in the Markdown tree resolves.

Documentation is a deliverable here, and a docs tree quietly rots into broken
cross-references faster than code does, because nothing compiles it. This runs
in CI for the same reason clippy does.

Links inside fenced code blocks are skipped: those are examples, not
navigation.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

LINK = re.compile(r"\[([^\]]*)\]\(([^)]+)\)")
FENCE = re.compile(r"^\s*(```|~~~)")
SKIP_DIRS = {"target", ".git", "data"}


def strip_code_fences(text: str) -> list[tuple[int, str]]:
    """Returns (line number, line) for lines outside fenced code blocks."""
    out = []
    in_fence = False
    for number, line in enumerate(text.splitlines(), start=1):
        if FENCE.match(line):
            in_fence = not in_fence
            continue
        if not in_fence:
            out.append((number, line))
    return out


def is_external(link: str) -> bool:
    return link.startswith(("http://", "https://", "mailto:", "#"))


def check(root: Path) -> list[str]:
    problems = []

    for md in sorted(root.rglob("*.md")):
        if SKIP_DIRS & set(md.parts):
            continue

        for number, line in strip_code_fences(md.read_text(encoding="utf-8")):
            for match in LINK.finditer(line):
                raw = match.group(2).strip()
                if is_external(raw):
                    continue

                target = (md.parent / raw.split("#", 1)[0]).resolve()
                if not target.exists():
                    rel = md.relative_to(root)
                    problems.append(f"{rel}:{number}: broken link -> {raw}")

    return problems


def main() -> int:
    root = Path(__file__).resolve().parent.parent
    problems = check(root)

    if problems:
        print(f"{len(problems)} broken link(s):\n", file=sys.stderr)
        for p in problems:
            print(f"  {p}", file=sys.stderr)
        return 1

    print("all documentation links resolve")
    return 0


if __name__ == "__main__":
    sys.exit(main())
