#!/usr/bin/env python3
"""Report every name in a Ghidra program that `names.tsv` does not sanction.

`apply-ghidra-names.py` is **additive**: it writes the names the documentation
has evidence for and never removes anything. So a database accumulates names
from sources that left no record - a fuzzy cross-application sweep, a manual
rename during an investigation, an experiment nobody undid - and those names
outlive the session that made them. `just apply-names` cannot clean them,
because it does not know they are there.

That matters more than it sounds. A wrong name is worse than no name: the
decompiler answers to it, so a search lands on the wrong function and the reader
has no signal that anything is off. Run on `psp-pulse-usa` on 2026-08-09 this
found **54 of 64** `_q`-suffixed names unsanctioned, including
`Ship_UpdateCameraRigs` (confidence 82, the spine of `camera.md`'s field-of-view
chain) displayed as `Ship_UpdateSideshiftInput_q`, and `Gu_Fog` (95) displayed
as `Gu_DepthMask_q`. The GE state-cache wrappers are 9-22 instructions and
structurally identical, which is exactly the shape a fuzzy matcher permutes.

**The docs win.** A disagreement is a defect in the database, never in
`names.tsv` - see `docs/ghidra/workflow.md`. This script only reports; repairing
is `just apply-names` for the addresses that *are* documented, and a manual
revert to `FUN_<addr>` for the rest.

Usage:
    scripts/audit-ghidra-names.py                     # every binary with a names.tsv
    scripts/audit-ghidra-names.py --binary psp-pulse-usa
"""

from __future__ import annotations

import argparse
import json
import re
import sys
import urllib.parse
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FUNCTIONS_DIR = ROOT / "docs" / "ghidra" / "functions"

# Shared with apply-ghidra-names.py. Kept as its own copy rather than imported
# because that file's name has a hyphen in it and is not importable as a module.
BINARY_PROGRAMS = {
    "psp-pulse-usa": "/psp-pulse-usa/BOOT.BIN",
    "psp-pulse-eu": "/psp-pulse-eu/BOOT.BIN",
    "psp-pure-usa": "/psp-pure-usa/BOOT.BIN",
    "psp-pure-eu": "/psp-pure-eu/BOOT.BIN",
    "ps2-pulse-eu": "/ps2-pulse-eu/SCES_547.48",
}

CONFIDENT_MIN = 70

# Names Ghidra or the toolchain produces on its own. Anything matching these is
# not a claim about what the function is, so it is not the audit's business.
GENERATED = re.compile(
    r"^(FUN_|LAB_|SUB_|thunk_|DAT_|_?_?start$|entry$|caseD_|switchD_)", re.IGNORECASE
)


def sanctioned(binary: str) -> dict[str, tuple[str, int, str]]:
    """{address: (applied symbol, confidence, evidence page)} for a binary.

    `names.tsv` is not the only sanctioned input. `resolve-imports` writes
    `data/ghidra/psp-imports.tsv` - 306 rows of PSP kernel imports named from
    their NIDs - and `apply-ghidra-names.py` applies it to `psp-pulse-usa`
    alongside that binary's own table. It lives under gitignored `data/`, so it
    is absent on a fresh checkout; treat a missing file as "no extra rows"
    rather than as an error, or the audit reports 300 kernel functions as
    unsanctioned on every machine that has not run `just resolve-imports`.
    """
    out: dict[str, tuple[str, int, str]] = {}
    sources = [FUNCTIONS_DIR / binary / "names.tsv"]
    if binary == "psp-pulse-usa":
        sources.append(ROOT / "data" / "ghidra" / "psp-imports.tsv")
    for path in sources:
        out.update(_read_table(path))
    return out


def _read_table(path: Path) -> dict[str, tuple[str, int, str]]:
    out: dict[str, tuple[str, int, str]] = {}
    if not path.is_file():
        return out
    for lineno, line in enumerate(path.read_text().splitlines(), 1):
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        if len(fields) != 5:
            print(f"{path}:{lineno}: expected 5 fields, got {len(fields)}", file=sys.stderr)
            continue
        address, _kind, name, confidence, evidence = (f.strip() for f in fields)
        conf = int(confidence)
        # The `_q` is derived from the confidence column, not stored - the same
        # rule apply-ghidra-names.py applies. A row that bakes the suffix into
        # the name gets it twice, which is its own defect and shows up here.
        symbol = name if conf >= CONFIDENT_MIN else f"{name}_q"
        out[address.lower().replace("0x", "")] = (symbol, conf, evidence)
    return out


def named_functions(url: str, program: str) -> list[tuple[str, str]]:
    """[(name, address)] for every function whose name is not machine-generated.

    One request, no paging: the bridge's `list_functions` **ignores `limit` and
    `offset`** and returns the whole program either way, so a paging loop never
    reaches a short page and spins forever. Its rows are `NAME at ADDRESS`.
    """
    params = urllib.parse.urlencode({"program": program})
    with urllib.request.urlopen(f"{url}/list_functions?{params}", timeout=180) as r:
        body = r.read().decode("utf-8", "replace")
    try:
        parsed = json.loads(body)
        lines = parsed if isinstance(parsed, list) else parsed.get("result", "").splitlines()
    except json.JSONDecodeError:
        lines = body.splitlines()

    found: list[tuple[str, str]] = []
    for line in lines:
        name, sep, addr = line.strip().rpartition(" at ")
        if not sep:
            continue
        name, addr = name.strip(), addr.strip().lower().replace("0x", "")
        if name and not GENERATED.match(name):
            found.append((name, addr))
    return found


def audit(url: str, binary: str) -> int:
    program = BINARY_PROGRAMS[binary]
    table = sanctioned(binary)
    if not table:
        print(f"== {binary}: no names.tsv, skipped")
        return 0
    try:
        live = named_functions(url, program)
    except OSError as exc:
        print(f"== {binary}: bridge unreachable ({exc})", file=sys.stderr)
        return 1

    unsanctioned, mismatched = [], []
    for name, addr in live:
        entry = table.get(addr)
        if entry is None:
            unsanctioned.append((name, addr))
        elif entry[0] != name:
            mismatched.append((name, addr, entry))

    print(f"\n== {binary} ({program})")
    print(f"   {len(live)} named function(s) live, {len(table)} row(s) in names.tsv")
    if mismatched:
        print(f"\n   MISMATCHED - names.tsv documents this address as something else:")
        for name, addr, (symbol, conf, page) in sorted(mismatched, key=lambda r: -r[2][1]):
            print(f"     {addr}  database `{name}`  ->  names.tsv `{symbol}` ({conf}, {page})")
    if unsanctioned:
        print(f"\n   UNSANCTIONED - no names.tsv row for this address at all:")
        for name, addr in sorted(unsanctioned):
            print(f"     {addr}  `{name}`")
    if not mismatched and not unsanctioned:
        print("   clean: every live name is sanctioned by names.tsv")
    return 1 if (mismatched or unsanctioned) else 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--url", default="http://127.0.0.1:8089", help="GhidraMCP bridge URL")
    ap.add_argument("--binary", choices=sorted(BINARY_PROGRAMS), help="just this one")
    args = ap.parse_args()

    binaries = [args.binary] if args.binary else sorted(BINARY_PROGRAMS)
    worst = 0
    for binary in binaries:
        worst = max(worst, audit(args.url, binary))
    if worst:
        print(
            "\nThe docs win: a disagreement is a defect in the database, not in "
            "names.tsv. Repair with `just apply-names` for documented addresses, "
            "and revert the rest to FUN_<addr>."
        )
    return worst


if __name__ == "__main__":
    sys.exit(main())
