#!/usr/bin/env python3
"""Check every `names.tsv` offline, the way `just apply-names` checks it online.

`apply-ghidra-names.py` already refuses a row whose evidence page has gone
stale, and refuses a malformed row outright. **It only ever gets the chance
when a Ghidra bridge is up**, because refusing is the first step of applying.
CI has no Ghidra, so on 2026-08-19 nothing in `just check` read `names.tsv` at
all: 1,028 rows across six binaries, the file CLAUDE.md calls "the only thing
that makes the database reproducible from the repository", with no gate on it.
This script is that gate. It imports the checks rather than restating them, so
the offline rule and the online rule cannot drift apart.

It also adds the one rule the applier structurally cannot enforce.

## The `_q` suffix belongs to the confidence column, not the name column

`names.tsv` stores the **bare** name, and `Row.symbol` derives the suffix from
the confidence: below `CONFIDENT_MIN` it appends `_q`, at or above it does not.
So a row is not free to spell the suffix itself, and writing it there breaks in
both directions:

- **Below 70 it doubles.** `Weapon_PostBlastImpulse_q` at confidence 68 is
  applied to Ghidra as `Weapon_PostBlastImpulse_q_q`. Three rows did this, and
  `just apply-names` wrote all three into the database.
- **At or above 70 it sticks.** `Rocket_HitCraft_q` at 78 is applied verbatim,
  so a name the documentation rates as verified carries an uncertainty marker
  at every call site - the exact opposite of what the suffix is for. That is
  [ADR-0005](../docs/architecture/adr/0005-ghidra-conventions.md)'s own
  predicted consequence ("names change when confidence crosses 70, so `_q`
  suffixes churn") arriving without the churn.

Neither is visible to the existing evidence check, whose pattern is
`\\b<name>(_q)?\\b` - deliberately, so a page documenting a sub-70 name may
write either spelling. A row that spells the suffix satisfies that pattern
against a page that spells it too, and both stay wrong together.

`SUFFIX_BASELINE` holds the rows that cannot be fixed in the TSV alone, in the
`check-file-size.py` idiom: it may shrink, never grow, and a row leaves it by
being fixed. See that script for why a ratchet rather than a flat rule.

**It is keyed on the address, not the line.** `Row.source` is
`<file>:<lineno>`, and a `names.tsv` gains rows constantly and in the middle -
they are grouped by subsystem, not appended - so a line-number key would go
stale on roughly the next name anyone recovers, and go stale *loudly*: one
inserted row above the first baselined line reports all eight as newly
suffixed and all eight baseline entries as no longer applying, sixteen
complaints for a change that touched none of them. Addresses are unique within
a binary and are what the row is actually about.

Usage:
    scripts/check-ghidra-names.py
"""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FUNCTIONS_DIR = ROOT / "docs" / "ghidra" / "functions"

# Below this the applier appends `_q`; at or above it applies the name as
# written. Read from the applier itself rather than restated, so one edit moves
# both.
APPLIER = ROOT / "scripts" / "apply-ghidra-names.py"

# Rows whose name column spells `_q` at a confidence of 70 or more. Dropping the
# suffix here is one edit; what makes each of these a change rather than a typo
# fix is that its evidence page, and every cross-reference to it, spells the
# suffixed form too, and the rename has to land with them. Recorded at their
# current confidence so a row that moves has to be looked at again.
#
# **Not the sub-70 rows.** Those applied a doubled `_q_q` and were fixed in the
# change that added this script - stripping the suffix is all they needed,
# because the page's own spelling already satisfies the evidence pattern either
# way.
SUFFIX_BASELINE = {
    "ps2-pulse-eu/0x0015a550": ("Ship_UpdateAirbrakes_q", 78),
    "psp-pulse-eu/0x08846904": ("Ship_UpdateSideshiftInput_q", 70),
    "psp-pulse-usa/0x08846a54": ("Ship_UpdateSideshiftInput_q", 72),
    "psp-pulse-usa/0x0886ebdc": ("Rocket_HitCraft_q", 78),
    "psp-pulse-usa/0x0886ed34": ("Rocket_SpawnCraftExplosion_q", 78),
    "psp-pulse-usa/0x08915484": ("Psys_Spawn_q", 72),
}


def load_applier():
    """Import `apply-ghidra-names.py` for its parser and its evidence check.

    The hyphenated filename is not a module name, so this goes through
    `importlib` rather than an `import` statement. Importing it is the point:
    the offline gate and the online applier must agree about what a valid row
    is, and the cheapest way to guarantee that is to have only one of them
    decide.
    """
    spec = importlib.util.spec_from_file_location("apply_ghidra_names", APPLIER)
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot import {APPLIER}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def key_for(path: Path, row) -> str:
    """`<binary>/<address>` - the baseline key, which a row insertion cannot move."""
    return f"{path.parent.name}/{row.address}"


def where(path: Path, row) -> str:
    """`<binary>/names.tsv:<line>` - for a complaint, where a line number helps."""
    return f"{path.parent.name}/{row.source}"


def check_suffix(applier, path: Path, rows: list) -> tuple[list[str], list[str]]:
    """Report rows that spell `_q` themselves, split by what the fix costs."""
    broken, stale = [], []
    for row in rows:
        if not row.name.endswith("_q"):
            continue
        key = key_for(path, row)
        if row.confidence < applier.CONFIDENT_MIN:
            # The applier appends a second suffix to this one. No baseline
            # entry is ever right for it: it is applying a name that exists
            # nowhere in the documentation.
            broken.append(
                f"{where(path, row)}: `{row.name}` at confidence {row.confidence} applies as "
                f"`{row.name}_q` - drop the `_q` from the name column, the applier adds it"
            )
        elif key not in SUFFIX_BASELINE:
            stale.append(
                f"{where(path, row)}: `{row.name}` at confidence {row.confidence} keeps a `_q` "
                f"the applier will not strip - drop it here and on the evidence page"
            )
        elif SUFFIX_BASELINE[key][1] != row.confidence:
            stale.append(
                f"{where(path, row)}: `{row.name}` moved to confidence {row.confidence} from "
                f"{SUFFIX_BASELINE[key][1]} - the suffix decision needs revisiting"
            )
    return broken, stale


def check_floor(applier, path: Path, rows: list) -> list[str]:
    """ADR-0005: below 50, do not rename at all.

    **Deliberately stricter than the applier**, which skips a sub-50 row and
    prints why rather than refusing. That skip is not redundant and is not
    dead: `just apply-names` runs against whatever `names.tsv` is on disk,
    including on a branch that has not been through `just check` yet, and
    skipping is the right thing to do at that moment. What it cannot do is stop
    the row being committed, because it never returns non-zero for it. Nothing
    hits this today - zero rows across all six binaries - so this is a rule
    being written down while it is free, not a state being cleaned up.
    """
    return [
        f"{where(path, row)}: `{row.name}` at confidence {row.confidence} is "
        f"below {applier.QUALIFIED_MIN} - leave it as `FUN_{row.address[2:]}` and write "
        f"the hypothesis on the page instead"
        for row in rows
        if row.confidence < applier.QUALIFIED_MIN
    ]


def check_baseline_rows(seen: set[str]) -> list[str]:
    """A baseline only ever gets shorter - a row for a fixed line must go."""
    return [
        f"{key}: `{name}` is no longer suffixed in names.tsv - delete this "
        f"SUFFIX_BASELINE row, the exemption outlives the problem"
        for key, (name, _) in SUFFIX_BASELINE.items()
        if key not in seen
    ]


def report(heading: str, rows: list[str], advice: str) -> None:
    if not rows:
        return
    print(f"\n{heading}\n")
    for row in rows:
        print(f"  {row}")
    print(f"\n{advice}")


def main() -> int:
    applier = load_applier()

    tsvs = sorted(FUNCTIONS_DIR.glob("*/names.tsv"))
    if not tsvs:
        raise SystemExit(f"no names.tsv found under {FUNCTIONS_DIR}")

    evidence: list[str] = []
    broken: list[str] = []
    stale: list[str] = []
    floor: list[str] = []
    suffixed: set[str] = set()
    total = 0

    for path in tsvs:
        # `read_rows` raises SystemExit on a malformed row, which is the
        # behaviour wanted here too: a row that will not parse cannot be
        # checked against anything else.
        rows = applier.read_rows(path)
        total += len(rows)
        evidence += [f"{path.parent.name}/{c}" for c in applier.check_evidence(rows)]
        row_broken, row_stale = check_suffix(applier, path, rows)
        broken += row_broken
        stale += row_stale
        floor += check_floor(applier, path, rows)
        suffixed |= {key_for(path, r) for r in rows if r.name.endswith("_q")}

    report(
        "row(s) whose evidence page no longer supports them:",
        evidence,
        "The pages are the record of truth, so this means the row is stale, not the\n"
        "page. Repoint the row, or delete it if the name was withdrawn.",
    )
    report(
        "row(s) that apply a doubled `_q_q` to Ghidra:",
        broken,
        "The name column holds the bare name; the applier derives the suffix from the\n"
        "confidence column. Drop the `_q` here - the evidence page keeps its own\n"
        "spelling either way, so no page edit is needed.",
    )
    report(
        "row(s) carrying a `_q` the confidence column says to drop:",
        stale,
        "Drop it from the name column and from the evidence page in the same change,\n"
        "along with every cross-reference to the suffixed spelling. If that cannot\n"
        "happen now, add the row to SUFFIX_BASELINE and say why in the commit.",
    )
    report(
        "SUFFIX_BASELINE row(s) that no longer apply - delete them:",
        check_baseline_rows(suffixed),
        "The baseline only ever gets shorter. Leaving a row keeps an exemption alive.",
    )
    report(
        "row(s) named below the confidence floor:",
        floor,
        "A guess dressed as a name stops other people from looking. See ADR-0005.",
    )

    if evidence or broken or stale or floor or check_baseline_rows(suffixed):
        return 1

    print(f"{total} names.tsv row(s) across {len(tsvs)} binaries: evidence, suffix and floor rules hold")
    return 0


if __name__ == "__main__":
    sys.exit(main())
