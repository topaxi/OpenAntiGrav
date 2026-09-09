#!/usr/bin/env python3
"""Harvests the names PPSSPP's own analysis already knows, over the debugger.

PPSSPP identifies a chunk of a PSP binary automatically: HLE syscall stubs
(the same ones `scripts/psp-import-names.txt` already names by NID) and, more
interestingly, a hash-matched function-replacement database - `memcpy`,
`strlen`, `sinf`, `cosf`, `sqrtf` and the rest of newlib/libnids, recognised
by their machine code regardless of which game shipped them. `hle.func.list`
over the websocket debugger (`docs/reverse-engineering/ppsspp-debugger.md`)
returns every one PPSSPP found for the loaded module, with address, name and
size - no GUI, no Debug menu, and (per that doc's own note that
`memory.disasm` carries "PPSSPP's own symbol names for calls") this is the
purpose-built listing rather than a disasm-scrape of the same information.

**This never writes to `names.tsv` and never should.** That file's contract
(CLAUDE.md, `scripts/apply-ghidra-names.py`) is that every row cites an
evidence page whose text still contains the address and the name -
`just check-names` refuses a row that doesn't. A name PPSSPP's hash database
recognised has no such page: nobody here read the function and wrote why it
is `sinf`, PPSSPP's own signature matcher decided it silently. So this writes
a separate table with its own provenance column, the same shape
`scripts/psp-import-names.txt` already established for "a name from a
mechanical source, not a documented reading."

PPSSPP also skips unresolved addresses under a `z_un_<addr>` placeholder
(the same convention `PpssppImportSymFile.py` filters with `skipZun`); this
script drops those too; they carry no name at all.

Needs a PPSSPP debugger already running and connectable - `just scripted-emu`
gets one to a live race, but that is more than this needs: **module analysis
happens at load time**, so connecting right after boot (the CPU breaks at the
entry point automatically under `--debugger`) already has the full picture.
See docs/reverse-engineering/ppsspp-debugger.md for how to start one:

    PPSSPPHeadless data/cache/pulse-psp-usa.iso --debugger=47800 --graphics=software --timeout=1800

Usage:
    uv run --with websocket-client scripts/harvest-ppsspp-symbols.py psp-pulse-usa --port 47800
"""

from __future__ import annotations

import argparse
import sys
from datetime import date
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

ROOT = Path(__file__).resolve().parent.parent

# The four PSP binaries PPSSPP can run - the same set export-ppsspp-sym.py
# accepts, for the same reason: the other three names.tsv binaries are not
# PSP at all.
PSP_BINARIES = {"psp-pulse-usa", "psp-pulse-eu", "psp-pure-usa", "psp-pure-eu"}


def harvest(dbg: Debugger) -> list[dict]:
    reply = dbg.call("hle.func.list")
    functions = reply["functions"]
    return [f for f in functions if not f["name"].startswith("z_un_")]


def write_table(rows: list[dict], binary: str, ppsspp_version: str, out: Path) -> None:
    out.parent.mkdir(parents=True, exist_ok=True)
    lines = [
        f"# PPSSPP's own auto-detected symbols for {binary}, harvested live over its",
        f"# websocket debugger's `hle.func.list` (docs/reverse-engineering/ppsspp-debugger.md),",
        f"# {ppsspp_version}, {date.today().isoformat()}.",
        "#",
        "# Provenance: PPSSPP's own HLE-syscall detection and hash-matched",
        "# function-replacement database, not a documented reading. **These rows",
        "# have no evidence page and MUST NOT be merged into names.tsv** - see",
        "# scripts/harvest-ppsspp-symbols.py's own docstring and",
        "# docs/reverse-engineering/ppsspp-symbol-bridge.md.",
        "#",
        "# address\tname\tsize\tsource",
    ]
    for row in sorted(rows, key=lambda r: r["address"]):
        lines.append(
            "0x%08x\t%s\t%d\tppsspp-hle.func.list" % (row["address"], row["name"], row["size"])
        )
    out.write_text("\n".join(lines) + "\n")


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("binary", choices=sorted(PSP_BINARIES), help="which disc is loaded right now")
    ap.add_argument("--port", type=int, default=47800)
    ap.add_argument(
        "--out", type=Path, default=None,
        help="output table path (default scripts/ppsspp-detected-<binary>.tsv)",
    )
    args = ap.parse_args()

    try:
        dbg = Debugger(args.port, connect_timeout=15.0)
    except Exception as e:  # noqa: BLE001 - connection diagnosis is the whole point
        print(f"cannot reach a PPSSPP debugger on ws://127.0.0.1:{args.port}/debugger: {e}", file=sys.stderr)
        print("start one first - see docs/reverse-engineering/ppsspp-debugger.md", file=sys.stderr)
        return 1

    try:
        rows = harvest(dbg)
    finally:
        dbg.close()

    out = args.out or (ROOT / "scripts" / f"ppsspp-detected-{args.binary}.tsv")
    write_table(rows, args.binary, "PPSSPP v1.20.4", out)

    zz = sum(1 for r in rows if r["name"].startswith("zz_"))
    real = len(rows) - zz
    print(f"{args.binary}: {len(rows)} named functions -> {out}")
    print(f"  {zz} are zz_-prefixed HLE syscall stubs (likely already in psp-imports.tsv)")
    print(f"  {real} are real detections (libc/newlib, soft-float helpers, math routines, sceGu*)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
