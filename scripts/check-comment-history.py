#!/usr/bin/env python3
"""A ratchet on history in code comments: dates, "maintainer", lane names.

CLAUDE.md: "Code comments say what and why, never when or who." A comment that
says `(measured 2026-10-01)`, `per the maintainer`, or names the drive lane that
wrote it is history, and history belongs in `git log` and on the evidence page,
where the comment can cite it by section. This tree had about 1,650 such
comments when this landed, so the rule is a **ratchet** and not a ban: a crate
over its `BASELINE` count fails, and a crate under its count fails too, so the
row is lowered the day its last comment is cleaned and never raised.

What is counted, per comment line (`//`, `///`, `//!`, and `/* */` in `.rs`,
`.wgsl` and `.wesl` under `crates/`):

- `dated`: an ISO date `20YY-MM-DD` outside a URL and outside a `"..."` string
  literal in the comment. A date in backticks is counted; a date in quotes is
  not, because a quoted string is the text the comment is quoting, not its voice.
- `maintainer`: the word `maintainer` or `maintainers`, any case, outside a URL.
- `lane`: a lane name, conservatively. A lowercase kebab-case token
  (`magstrip-wire-hd`) that is backticked or quoted and followed by `lane`,
  or `lane` followed by a backticked or quoted kebab-case token. A bare
  `magstrip-wire-hd lane` is **not** matched, and neither is `per-seat lane`:
  either would also match a design term. This under-counts on purpose.

Not counted: any line inside an inline `#[cfg(test)] mod NAME { ... }` body
(the module's end is found by indentation, as in `check-file-size.py`), any
file under a `tests/` directory, and any file named `tests.rs`, which is a test
module declared in its parent. A test's history is still history, but the rule
is aimed at the code a reader of the crate meets, and the tests are the part
that most often quotes an original's measured value and its date.

A line that matches several kinds counts once. A crate's count is the number
of such lines in it.

Rules:

- A crate not in `BASELINE` may not have any counted line.
- A crate in `BASELINE` may not exceed its count. Growth lists the crate's
  counted lines, since a diff is not available here; the fix is to write the
  new comment without the date, the name or the "maintainer", and cite the
  evidence page and section instead.
- A crate whose count fell below its row must lower the row to the new count
  (or delete it at zero). A ceiling left above the real count would let the
  comments it freed be written again.

Run `python3 -I scripts/check-comment-history.py --table` to print each
crate's counts by kind and the `BASELINE` block ready to paste.
"""

from __future__ import annotations

import re
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
SUFFIXES = {".rs", ".wgsl", ".wesl"}
LISTING_CAP = 40

# Lines per crate, frozen on the day this landed (counted by
# `--table`). Each row may shrink, never grow; a row is deleted at zero.
BASELINE: dict[str, int] = {
    "2048": 29,
    "ai": 18,
    "assets": 4,
    "audio": 11,
    "core": 1,
    "disc": 7,
    "display": 8,
    "formats": 4,
    "fx": 62,
    "gameplay": 24,
    "hd": 57,
    "hud": 27,
    "input": 7,
    "log": 1,
    "mesh": 46,
    "music": 1,
    "net": 1,
    "omega": 13,
    "physics": 37,
    "pob": 9,
    "post": 13,
    "present": 4,
    "pulse": 21,
    "pure": 37,
    "race": 22,
    "rcs": 19,
    "render": 17,
    "sound": 6,
    "source": 2,
    "tables": 37,
    "texture": 1,
    "trace": 3,
    "ui": 55,
    "ui-screens": 46,
    "vex": 2,
    "view": 2,
    "weapons": 32,
}

URL = re.compile(r"https?://\S+")
QUOTED = re.compile(r'"[^"]*"')
DATE = re.compile(r"\b20\d\d-\d\d-\d\d\b")
MAINTAINER = re.compile(r"\bmaintainers?\b", re.IGNORECASE)
KEBAB = r"[a-z][a-z0-9]*(?:-[a-z0-9]+)+"
LANE = re.compile(rf"(?:`|\"){KEBAB}(?:`|\")?\s+lane\b|\blane\s+[`\"]{KEBAB}[`\"]")

CFG_TEST = re.compile(r"(\s*)#\[cfg\(test\)\]\s*$")
TEST_MOD = re.compile(r"\s*(?:pub(?:\([^)]*\))?\s+)?mod\s+(\w+)\s*\{\s*$")
IDENT_CHAR = re.compile(r"\w")
RAW_START = re.compile(r'b?r(#*)"')


def test_body_lines(lines: list[str]) -> set[int]:
    """1-based line numbers inside every inline `#[cfg(test)] mod NAME { ... }`.

    The body ends at the first line that is exactly the attribute's indent
    followed by `}`, as in `check-file-size.py`: brace counting misreads raw
    strings holding braces.
    """
    excluded: set[int] = set()
    for index, line in enumerate(lines):
        marker = CFG_TEST.match(line)
        if marker is None:
            continue
        indent = marker.group(1)
        head = index + 1
        while head < len(lines) and lines[head].lstrip().startswith("#["):
            head += 1
        if head >= len(lines) or TEST_MOD.match(lines[head]) is None:
            continue
        closing = f"{indent}}}"
        for end in range(head + 1, len(lines)):
            if lines[end] == closing:
                excluded.update(range(index + 1, end + 2))
                break
    return excluded


def comments(text: str) -> list[tuple[int, str]]:
    """Every comment segment as (1-based line, text), one per physical line.

    A small lexer: it skips string, raw string and char literals so that a
    `//` inside `"http://..."` is not a comment, and it tracks nested `/* */`.
    A lifetime (`'a`) is told apart from a char literal by its third byte.
    """
    out: list[tuple[int, str]] = []
    i, n, line = 0, len(text), 1
    while i < n:
        c = text[i]
        if c == "\n":
            line += 1
            i += 1
        elif text.startswith("//", i):
            end = text.find("\n", i)
            end = n if end < 0 else end
            out.append((line, text[i:end]))
            i = end
        elif text.startswith("/*", i):
            depth, j = 1, i + 2
            seg_start, seg_line = i, line
            while j < n and depth > 0:
                if text.startswith("/*", j):
                    depth += 1
                    j += 2
                elif text.startswith("*/", j):
                    depth -= 1
                    j += 2
                elif text[j] == "\n":
                    out.append((seg_line, text[seg_start:j]))
                    line += 1
                    j += 1
                    seg_start, seg_line = j, line
                else:
                    j += 1
            out.append((seg_line, text[seg_start:j]))
            i = j
        elif c == '"':
            j = i + 1
            while j < n and text[j] != '"':
                if text[j] == "\\":
                    j += 1
                if j < n and text[j] == "\n":
                    line += 1
                j += 1
            i = j + 1
        elif c in "rb" and (i == 0 or not IDENT_CHAR.match(text[i - 1])):
            raw = RAW_START.match(text, i)
            if raw is None:
                i += 1
                continue
            terminator = '"' + raw.group(1)
            end = text.find(terminator, raw.end())
            end = n if end < 0 else end + len(terminator)
            line += text.count("\n", i, end)
            i = end
        elif c == "'":
            if i + 1 < n and text[i + 1] == "\\":
                end = text.find("'", i + 2)
                i = n if end < 0 else end + 1
            elif i + 2 < n and text[i + 2] == "'":
                i += 3
            else:
                i += 1
        else:
            i += 1
    return out


def kinds_of(comment: str) -> set[str]:
    """Which history kinds one comment segment carries."""
    body = URL.sub(" ", comment)
    found: set[str] = set()
    if DATE.search(QUOTED.sub(" ", body)):
        found.add("dated")
    if MAINTAINER.search(body):
        found.add("maintainer")
    if LANE.search(body):
        found.add("lane")
    return found


def crate_of(path: Path) -> str | None:
    parts = path.relative_to(ROOT).parts
    if len(parts) < 3 or parts[0] != "crates":
        return None
    if "tests" in parts[2:-1] or parts[-1] == "tests.rs":
        return None
    return parts[1]


def measure() -> dict[str, dict[tuple[str, int], set[str]]]:
    """crate -> {(file, line): kinds} for every counted comment line."""
    hits: dict[str, dict[tuple[str, int], set[str]]] = defaultdict(dict)
    for path in sorted((ROOT / "crates").rglob("*")):
        if path.suffix not in SUFFIXES or not path.is_file():
            continue
        crate = crate_of(path)
        if crate is None:
            continue
        text = path.read_text(encoding="utf-8")
        skip = test_body_lines(text.split("\n")) if path.suffix == ".rs" else set()
        relative = path.relative_to(ROOT).as_posix()
        for line, segment in comments(text):
            if line in skip:
                continue
            kinds = kinds_of(segment)
            if kinds:
                hits[crate].setdefault((relative, line), set()).update(kinds)
    return hits


def show(crate: str, found: dict[tuple[str, int], set[str]], lines: list[str]) -> None:
    for (relative, line), kinds in sorted(found.items())[:LISTING_CAP]:
        lines.append(f"  {relative}:{line}: {', '.join(sorted(kinds))}")
    if len(found) > LISTING_CAP:
        lines.append(f"  ... and {len(found) - LISTING_CAP} more in {crate}")


def table(hits: dict[str, dict[tuple[str, int], set[str]]]) -> int:
    rows = []
    for crate in sorted(hits):
        found = hits[crate]
        counts = {
            kind: sum(1 for kinds in found.values() if kind in kinds)
            for kind in ("dated", "maintainer", "lane")
        }
        rows.append((crate, counts["dated"], counts["maintainer"], counts["lane"], len(found)))
    print(f"{'crate':<16}{'dates':>7}{'maintainer':>12}{'lanes':>7}{'lines':>7}")
    for crate, dated, maint, lane, total in rows:
        print(f"{crate:<16}{dated:>7}{maint:>12}{lane:>7}{total:>7}")
    print(
        f"{'total':<16}{sum(r[1] for r in rows):>7}{sum(r[2] for r in rows):>12}"
        f"{sum(r[3] for r in rows):>7}{sum(r[4] for r in rows):>7}"
    )
    print("\nBASELINE = {")
    for crate, *_rest, total in rows:
        print(f'    "{crate}": {total},')
    print("}")
    return 0


def main() -> int:
    hits = measure()
    if "--table" in sys.argv[1:]:
        return table(hits)

    counts = {crate: len(found) for crate, found in hits.items()}
    lines: list[str] = []
    failed = False
    for crate in sorted(set(counts) | set(BASELINE)):
        count = counts.get(crate, 0)
        ceiling = BASELINE.get(crate, 0)
        if count > ceiling:
            failed = True
            lines.append(
                f"crate {crate}: {count} history comment line(s), ceiling is {ceiling}"
            )
            show(crate, hits.get(crate, {}), lines)
        elif count < ceiling:
            failed = True
            lines.append(
                f"crate {crate}: now {count} history comment line(s), below its "
                f"ceiling of {ceiling} - lower the BASELINE row to {count}"
                + (" (or delete it at zero)" if count == 0 else "")
            )

    if failed:
        print("History in code comments (dates, \"maintainer\", lane names), or a BASELINE row out of step:\n")
        print("\n".join(lines))
        print(
            "\nWrite the comment as what and why. Put the date and the measurement on the "
            "evidence page\nand cite its section from the comment. A ceiling is lowered, "
            "never raised."
        )
        return 1

    total = sum(counts.values())
    print(
        f"OK: {total} history comment line(s) across {len(counts)} crate(s), "
        f"each within its BASELINE row"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
