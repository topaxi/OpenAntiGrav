#!/usr/bin/env python3
"""Exports a binary's `names.tsv` as a PPSSPP `.sym` symbol map.

`kotcrab/ghidra-allegrex` (the Allegrex processor module `just
build-allegrex` builds) ships two Ghidra scripts, `PpssppImportSymFile.py`
and `PpssppExportSymFile.py`, that move names between Ghidra and PPSSPP's own
Debug menu ("Save symbol map" / "Load symbol map"). This script goes the
other way round from `PpssppExportSymFile.py`: **`names.tsv` straight to
`.sym`, with no Ghidra project and no PPSSPP instance involved at all.**
`names.tsv` is the authoritative source (the docs are authoritative over the
Ghidra database if the two ever disagree, per CLAUDE.md) and the Ghidra
project is not committed, so going through Ghidra would need a database that
does not exist on a fresh checkout and could only ever reproduce what this
script already has in hand.

The offset PPSSPP's import script asks for is **0**: every `names.tsv`
header already states image base `0x08804000`, which is exactly the case
where the addresses need no adjustment.

## The format

Confirmed by reading `PpssppExportSymFile.py` itself
(`data/tools/ghidra-allegrex/ghidra_scripts/`), one line per symbol:

    %08X %s,%04X\\n        # address, name, size - all in hex, no header

`PpssppImportSymFile.py` splits every line on the first space and reads
`parts[0]` as a bare hex int unconditionally - there is no comment syntax,
so a `#` header line would crash the import. This script never writes one;
the binary and its provenance go in the *filename* and in what this script
prints to stderr, never into the `.sym` body.

## Two things this script chooses not to invent

- **Function rows only.** `PpssppExportSymFile.py` itself only ever walks
  `getFunctionManager().getFunctions()` - there is no data-symbol counterpart
  in the format it produces. Emitting a `data` row as a "function" would tell
  PPSSPP's disassembly view something false about it, so `data` rows are
  counted and reported, never exported.
- **Size is always `0000`.** `names.tsv` has no size column - address, kind,
  name, confidence, evidence, nothing else - and inventing one (say, from the
  gap to the next row's address) would claim knowledge the table does not
  carry, the same reason this project refuses a stand-in asset elsewhere. A
  zero-size symbol is still a correctly named point in PPSSPP's disassembly
  view; it just does not claim to know where the function ends.

## Matching the pressing

Only `psp-pulse-usa`, `psp-pulse-eu`, `psp-pure-usa` and `psp-pure-eu` are
PPSSPP-relevant - `ps2-pulse-eu`, `ps3-hdfury-eu` and the Vita binaries do not
run under PPSSPP at all. USA and EU are genuinely different binaries (e.g.
`Cannon_UpdateRound` is `0x0886593c` on USA, `0x08865798` on EU), so loading
the wrong region's `.sym` would silently mislabel every address in PPSSPP.
There is no CLI default binary for that reason - `--binary` is required, and
the output filename always carries it.

Usage:
    scripts/export-ppsspp-sym.py psp-pulse-usa
    scripts/export-ppsspp-sym.py psp-pulse-eu --out /tmp/psp-pulse-eu.sym
"""

from __future__ import annotations

import argparse
import importlib.util
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
FUNCTIONS_DIR = ROOT / "docs" / "ghidra" / "functions"
APPLIER = ROOT / "scripts" / "apply-ghidra-names.py"

# The four PSP binaries PPSSPP can actually run. The other three names.tsv
# directories (ps2-pulse-eu, ps3-hdfury-eu, vita-2048-*) name real CPUs
# PPSSPP does not emulate; refusing them here means a typo'd binary name
# fails loudly instead of quietly emitting a .sym for hardware that will
# never load it.
PSP_BINARIES = {"psp-pulse-usa", "psp-pulse-eu", "psp-pure-usa", "psp-pure-eu"}


def load_applier():
    """Import `apply-ghidra-names.py` for its `Row`/`read_rows`/thresholds.

    Same trick `check-ghidra-names.py` already uses for the same reason: the
    hyphenated filename is not a module name, so this goes through
    `importlib` rather than an `import` statement, and the `_q`-suffix rule
    lives in exactly one place rather than three copies drifting apart.
    """
    spec = importlib.util.spec_from_file_location("apply_ghidra_names", APPLIER)
    if spec is None or spec.loader is None:
        raise SystemExit(f"cannot import {APPLIER}")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def export_sym(rows, applier) -> list[str]:
    """Render `function` rows as PPSSPP `.sym` lines, sorted by address."""
    lines = []
    for row in sorted(rows, key=lambda r: int(r.address, 16)):
        if row.kind != "function":
            continue
        if row.confidence < applier.QUALIFIED_MIN:
            continue  # ADR-0005: below 50, no name was ever written for this row
        lines.append("%08X %s,%04X" % (int(row.address, 16), row.symbol, 0))
    return lines


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("binary", choices=sorted(PSP_BINARIES), help="e.g. psp-pulse-usa")
    ap.add_argument(
        "--out", type=Path, default=None,
        help="output .sym path (default data/ghidra/<binary>.sym)",
    )
    ap.add_argument(
        "--skip-evidence-check", action="store_true",
        help="export even if names.tsv fails the evidence check (run `just check-names` instead of this)",
    )
    args = ap.parse_args()

    applier = load_applier()
    path = FUNCTIONS_DIR / args.binary / "names.tsv"
    if not path.is_file():
        print(f"{path}: not found", file=sys.stderr)
        return 1

    rows = applier.read_rows(path)

    if not args.skip_evidence_check:
        complaints = applier.check_evidence(rows)
        if complaints:
            print("evidence check failed - fix names.tsv first (see `just check-names`):", file=sys.stderr)
            for c in complaints:
                print(f"  {c}", file=sys.stderr)
            return 1

    lines = export_sym(rows, applier)
    out = args.out or (ROOT / "data" / "ghidra" / f"{args.binary}.sym")
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(("\n".join(lines) + "\n") if lines else "")

    data_rows = sum(1 for r in rows if r.kind == "data")
    below_floor = sum(1 for r in rows if r.kind == "function" and r.confidence < applier.QUALIFIED_MIN)
    print(f"{args.binary}: {len(lines)} functions -> {out}")
    if data_rows:
        print(f"  ({data_rows} data rows in names.tsv were not exported - PPSSPP's .sym has no data symbols)")
    if below_floor:
        print(f"  ({below_floor} function rows below confidence {applier.QUALIFIED_MIN} were not exported)")
    print()
    print(f"In PPSSPP: Debug menu -> Load symbol map... -> select {out}")
    print(f"This targets {args.binary} specifically - loading it against a different region's disc")
    print("silently mislabels every address (e.g. Cannon_UpdateRound is 0x0886593c on USA, 0x08865798 on EU).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
