#!/usr/bin/env python3
"""Validate `docs/ghidra/captures/` offline, with no Ghidra bridge.

A capture is a snapshot of what a Ghidra database held that `names.tsv` does
not carry. `names.tsv` has `check-ghidra-names.py`; without an equivalent here
these files would be the museum piece ADR-0047 warns about - written once,
never read, quietly rotting into a wrong record of a database nobody can
reopen.

What this asserts, and deliberately what it does not:

**Structure.** Every file parses, has the right column count for its kind, and
uses a well-formed address. A capture with a ragged row is a capture nobody can
diff against a future database.

**Provenance.** Every capture directory names a binary that
`docs/ghidra/functions/` also knows about. A capture for a binary with no
evidence tree is orphaned by construction.

**Honesty of the README.** The measurement table in `captures/README.md` is the
part anyone actually reads; if it says 11 plate comments and the file holds 9,
the table is the thing that gets believed. So the counts are checked against
the files.

**Documentation references resolve.** A plate comment that cites a `docs/` page
is making the same promise a `docs/` link makes, and `check-doc-links.py` never
sees it because it only walks `docs/**/*.md` prose, not TSV payloads. This
found `psp-pure-usa/rocket-visuals.md` on the first run: a comment in the Pure
database pointed at a page that does not exist, and nothing else in the project
could have told us.

**One file per binary directory is not a Ghidra capture at all:
`ppsspp-detected.tsv`.** `scripts/harvest-ppsspp-symbols.py` writes it,
straight from PPSSPP's own websocket debugger, not from a Ghidra database -
see that script's docstring and `docs/reverse-engineering/ppsspp-symbol-bridge.md`.
It sits beside the real captures because the column shape and the
structural checks below (parses, right column count, well-formed address)
are the same, and a second validator for one file would be more code than
the difference is worth. It is still exempt from every capture-specific
check past structure: it does not need an evidence tree the way a *Ghidra*
capture does (`ppsspp-hle.func.list` is not `names.tsv`-eligible in the
first place, so there is nothing for it to be orphaned from), and it never
appears in the README's Ghidra measurement table.

**What it does not assert: agreement with `names.tsv`.** A capture records what
a database held on a date, and a name in `names.tsv` may legitimately be
corrected afterwards. Failing the gate because a snapshot disagrees with
today's table would force the snapshot to be edited, and an edited snapshot is
not a snapshot - it is a second, weaker copy of `names.tsv`. Use
`scripts/audit-ghidra-names.py` for that comparison, against a live database,
where a disagreement is actionable.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CAPTURES = ROOT / "docs" / "ghidra" / "captures"
FUNCTIONS = ROOT / "docs" / "ghidra" / "functions"

# `address`, or `namespace::address` for a symbol the ELF loader put in a
# synthetic namespace (`_elfSectionHeaders::00000050`).
ADDRESS = re.compile(r"^(?:[A-Za-z_][A-Za-z0-9_]*::)?[0-9a-fA-F]+$")

# columns per file kind
SCHEMA = {
    "functions.tsv": 3,
    "labels.tsv": 4,
    "comments.tsv": 4,
    "signatures.tsv": 4,
    "variables.tsv": 6,
    # Not a Ghidra capture - PPSSPP's own auto-detected symbols, harvested by
    # scripts/harvest-ppsspp-symbols.py. Structure only: `source` here is
    # never Ghidra's SourceType (see that script's docstring), so it is not
    # held to the same vocabulary the real captures' `source` columns are.
    # Every other check below is either directory-scoped (the evidence-tree
    # check, which this file's own binary directory already satisfies) or
    # keyed to a specific real-capture filename (the README table, the
    # comments.tsv doc-reference check), so nothing further has to name this
    # file to leave it out of what only applies to Ghidra state.
    "ppsspp-detected.tsv": 4,
}

# A path anywhere under `docs/` or `handover/`, tolerating the `\n` escape a
# hard-wrapped comment puts in the middle of one. Ghidra's plate comments wrap
# at ~76 columns and wrap *inside* a path without hesitating, so
# `handover/ghidra-applies-no-\npsp-relocation-the-patch-is.md` is one
# reference, not two fragments. A regex that cannot span the escape silently
# passes every wrapped reference, which is worse than not checking at all: it
# reports success over the ones most likely to be stale.
DOC_REF = re.compile(r"(?:docs|handover)/[a-z0-9./-]+(?:\\n[a-z0-9./-]+)*\.md")

# A reference that was already dangling when the databases were captured. It is
# recorded rather than repaired because a capture is verbatim: the database
# really did point at a page that does not exist, and editing the row would
# erase the finding. The list may shrink - a new entry means a plate comment
# cites something nobody wrote, or a `handover/` thread landed and was deleted
# while a comment still pointed at it. Both are the defect this check exists to
# catch, and the second is the one CLAUDE.md predicts in as many words.
KNOWN_DANGLING = {
    ("psp-pure-usa", "docs/ghidra/functions/psp-pure-usa/rocket-visuals.md"),
    ("psp-pulse-usa", "handover/the-airbrake-flap-rotation-axis-is-chosen-not.md"),
}

# The measurement in README.md. Structs, prototypes and variable names are all
# zero project-wide, which is the whole point of the table, so they are asserted
# as absent files rather than as counts.
README_TABLE = re.compile(
    r"^\|\s*`(?P<binary>[a-z0-9-]+)`\s*\|\s*(?P<ver>\d+)\s*\|\s*(?P<structs>\d+)\s*\|"
    r"\s*(?P<protos>\d+)\s*\|\s*(?P<vars>\d+)\s*\|\s*(?P<comments>\d+)\s*\|"
    r"\s*(?P<labels>\d+)\s*\|\s*(?P<funcs>\d+)\s*\|"
)


def rows(path: Path) -> list[list[str]]:
    out = []
    for line in path.read_text().splitlines():
        if not line.strip() or line.startswith("#"):
            continue
        out.append(line.split("\t"))
    return out


def main() -> int:
    problems: list[str] = []

    if not CAPTURES.is_dir():
        print(f"{CAPTURES} does not exist", file=sys.stderr)
        return 1

    counts: dict[str, dict[str, int]] = {}
    for directory in sorted(p for p in CAPTURES.iterdir() if p.is_dir()):
        binary = directory.name
        counts[binary] = {}

        if not (FUNCTIONS / binary).is_dir():
            problems.append(
                f"{directory}: no evidence tree at docs/ghidra/functions/{binary}/ - "
                f"a capture for a binary the project does not document is orphaned"
            )

        for path in sorted(directory.iterdir()):
            if path.name not in SCHEMA:
                problems.append(f"{path}: unknown capture file (expected one of {sorted(SCHEMA)})")
                continue

            width = SCHEMA[path.name]
            data = rows(path)
            counts[binary][path.name] = len(data)

            if not data:
                problems.append(
                    f"{path}: no rows - an empty capture file is indistinguishable from a "
                    f"missing one, so delete it instead"
                )

            for n, fields in enumerate(data, 1):
                if len(fields) != width:
                    problems.append(
                        f"{path}:{n}: {len(fields)} columns, expected {width}"
                    )
                    continue
                if not ADDRESS.match(fields[0]):
                    problems.append(f"{path}:{n}: {fields[0]!r} is not an address")
                if not fields[1].strip():
                    problems.append(f"{path}:{n}: second column is empty")

            if path.name == "comments.tsv":
                for n, fields in enumerate(data, 1):
                    for wrapped in DOC_REF.findall(fields[-1]):
                        ref = wrapped.replace("\\n", "")
                        if (ROOT / ref).is_file():
                            continue
                        if (binary, ref) in KNOWN_DANGLING:
                            continue
                        problems.append(
                            f"{path}:{n}: cites {ref}, which does not exist"
                        )

    problems.extend(check_readme(counts))

    for p in problems:
        print(p, file=sys.stderr)
    if problems:
        print(f"\n{len(problems)} problem(s) in docs/ghidra/captures/", file=sys.stderr)
        return 1

    total = sum(sum(v.values()) for v in counts.values())
    print(f"docs/ghidra/captures: {len(counts)} binaries, {total} rows, all consistent")
    return 0


def check_readme(counts: dict[str, dict[str, int]]) -> list[str]:
    """The README's measurement table must match the files beside it."""
    readme = CAPTURES / "README.md"
    if not readme.is_file():
        return [f"{readme}: missing"]

    problems: list[str] = []
    seen: set[str] = set()
    for line in readme.read_text().splitlines():
        m = README_TABLE.match(line.strip())
        if not m:
            continue
        binary = m.group("binary")
        seen.add(binary)
        if binary not in counts:
            problems.append(f"{readme}: table row for {binary}, which has no capture directory")
            continue
        got = counts[binary]
        for column, filename in (
            ("comments", "comments.tsv"),
            ("labels", "labels.tsv"),
            ("funcs", "functions.tsv"),
        ):
            claimed = int(m.group(column))
            actual = got.get(filename, 0)
            if claimed != actual:
                problems.append(
                    f"{readme}: table says {binary} has {claimed} {column}, "
                    f"but {filename} holds {actual}"
                )
        # The whole argument of the capture is that these are zero everywhere.
        # If one is ever non-zero the table must say so and the file must exist.
        for column, filename in (("structs", None), ("protos", "signatures.tsv"), ("vars", "variables.tsv")):
            claimed = int(m.group(column))
            actual = got.get(filename, 0) if filename else 0
            if claimed != actual:
                problems.append(
                    f"{readme}: table says {binary} has {claimed} {column}, but the "
                    f"capture holds {actual}"
                )

    missing = set(counts) - seen
    for binary in sorted(missing):
        problems.append(f"{readme}: capture directory {binary}/ has no row in the measurement table")
    return problems


if __name__ == "__main__":
    sys.exit(main())
