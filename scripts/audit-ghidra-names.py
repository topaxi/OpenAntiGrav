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
`names.tsv` - see `docs/ghidra/workflow.md`. Repair a MISMATCHED name with
`just apply-names`, which has a documented name to restore; an UNSANCTIONED one
has no correct name to apply, so `--prune` reverts it to `FUN_<addr>`. That is
the honest state: it says "nobody has identified this", which is true, where a
stale name says something false.

Usage:
    scripts/audit-ghidra-names.py                     # every binary with a names.tsv
    scripts/audit-ghidra-names.py --binary psp-pulse-usa
    scripts/audit-ghidra-names.py --binary psp-pulse-usa --prune
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
    "ps3-hdfury-eu": "/ps3-hdfury-eu/EBOOT.elf",
}

CONFIDENT_MIN = 70

# Names Ghidra or the toolchain produces on its own. Anything matching these is
# not a claim about what the function is, so it is not the audit's business.
# `.opd.FUN_` is `ps3-hdfury-eu`'s share of this: PowerPC ELFs have a separate
# official procedure descriptor table, and Ghidra auto-labels each entry
# `.opd.FUN_<addr>` - 23,162 of them on this binary alone, all noise the same
# way `FUN_<addr>` itself is.
GENERATED = re.compile(
    r"^(FUN_|LAB_|SUB_|thunk_|DAT_|_?_?start$|entry$|caseD_|switchD_|\.opd\.FUN_)",
    re.IGNORECASE,
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
    `psp-imports-eu.tsv` is the same idea for `psp-pulse-eu`, the Ghidra target
    of record as of ADR-0048 - nothing generates it yet, so it is absent
    everywhere today, treated the same way.
    """
    out: dict[str, tuple[str, int, str]] = {}
    sources = [FUNCTIONS_DIR / binary / "names.tsv"]
    if binary == "psp-pulse-usa":
        sources.append(ROOT / "data" / "ghidra" / "psp-imports.tsv")
    if binary == "psp-pulse-eu":
        # Mirrors apply-ghidra-names.py's own additive pairing: absent on a
        # fresh checkout until `just resolve-imports` is run against the EU
        # BOOT.BIN, per ADR-0048. `_read_table` treats a missing file as no
        # extra rows, same as the USA branch above already relies on.
        sources.append(ROOT / "data" / "ghidra" / "psp-imports-eu.tsv")
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


def _executable_path(metadata_text: str) -> str:
    for line in metadata_text.splitlines():
        if line.startswith("Executable Path:"):
            return line.removeprefix("Executable Path:").strip()
    return ""


def ensure_active(url: str, program: str) -> bool:
    """Make `program` the bridge's true active program before any write.

    `rename_function_by_address` ignores its own `program` field and acts on
    whatever the bridge currently considers active - and `switch_program`'s
    `"success": true` reply is not trustworthy: confirmed live against this
    same bridge (see apply-ghidra-names.py's `Bridge.switch_program`), it
    reports success while leaving the true active program unchanged for any
    binary that shares a display name with another open program, `BOOT.BIN`
    among them. Without this check, `--prune` would revert names in whatever
    program happened to be truly active, not the one `--binary` named -
    exactly the bug that put 234 stray names into `psp-pulse-usa`.

    So this does not trust `switch_program`'s reply. It re-reads the true
    active program afterward with no `program` parameter - the same lookup a
    write implicitly uses - and compares its executable path against what
    `get_metadata` reports *for the program requested*, which, like every
    other GET, resolves correctly. Only a path match clears a write to proceed.
    """
    params = urllib.parse.urlencode({"program": program})
    urllib.request.urlopen(f"{url}/switch_program?{params}", timeout=30).read()
    with urllib.request.urlopen(f"{url}/get_metadata?{params}", timeout=30) as r:
        wanted = _executable_path(r.read().decode("utf-8", "replace"))
    with urllib.request.urlopen(f"{url}/get_metadata", timeout=30) as r:
        actual = _executable_path(r.read().decode("utf-8", "replace"))
    return bool(wanted) and wanted == actual


def revert(url: str, program: str, addr: str) -> bool:
    """Rename a function back to Ghidra's own `FUN_<addr>`.

    Removing a name is the only repair for an unsanctioned one: `names.tsv` has
    nothing to say about that address, so there is no correct name to apply.
    Ghidra's generated form is the honest state - it says "nobody has
    identified this", which is true, where the stale name says something false.
    """
    body = json.dumps(
        {"program": program, "function_address": f"0x{addr}", "new_name": f"FUN_{addr}"}
    ).encode()
    req = urllib.request.Request(
        f"{url}/rename_function_by_address",
        data=body,
        headers={"Content-Type": "application/json"},
    )
    try:
        with urllib.request.urlopen(req, timeout=30) as r:
            return "success" in r.read().decode("utf-8", "replace").lower()
    except OSError:
        return False


def audit(url: str, binary: str, prune: bool = False) -> int:
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

    if prune and unsanctioned:
        if not ensure_active(url, program):
            print(
                f"\n   refusing to prune: could not make {program!r} the bridge's true "
                "active program - a revert here would land in whichever program actually "
                "is active instead. Open it directly in Ghidra, or the bridge cannot "
                "currently reach it for writes.",
                file=sys.stderr,
            )
            return 1
        print(f"\n   reverting {len(unsanctioned)} unsanctioned name(s) to FUN_<addr>")
        done = sum(revert(url, program, addr) for _name, addr in unsanctioned)
        print(f"   reverted {done}, failed {len(unsanctioned) - done}")
        if done:
            urllib.request.urlopen(
                urllib.request.Request(
                    f"{url}/save_program",
                    data=json.dumps({"program": program}).encode(),
                    headers={"Content-Type": "application/json"},
                ),
                timeout=120,
            ).read()
            print("   saved")
        return 0 if done == len(unsanctioned) else 1

    return 1 if (mismatched or unsanctioned) else 0


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("--url", default="http://127.0.0.1:8089", help="GhidraMCP bridge URL")
    ap.add_argument("--binary", choices=sorted(BINARY_PROGRAMS), help="just this one")
    ap.add_argument(
        "--prune",
        action="store_true",
        help="revert UNSANCTIONED names to FUN_<addr>. Does not touch MISMATCHED "
        "ones - those have a documented name and `just apply-names` restores it. "
        "Run the plain audit first and read what it lists.",
    )
    args = ap.parse_args()

    binaries = [args.binary] if args.binary else sorted(BINARY_PROGRAMS)
    worst = 0
    for binary in binaries:
        worst = max(worst, audit(args.url, binary, prune=args.prune))
    if worst:
        print(
            "\nThe docs win: a disagreement is a defect in the database, not in "
            "names.tsv. Repair with `just apply-names` for documented addresses, "
            "and revert the rest to FUN_<addr>."
        )
    return worst


if __name__ == "__main__":
    sys.exit(main())
