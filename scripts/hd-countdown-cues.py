#!/usr/bin/env python3
"""Which cues Wipeout HD starts across a race start, in order, with the race manager's phase.

Boots HD on RPCS3's interpreter (the only decoder `Z0` breakpoints fire under), walks to a race,
restarts it back to the START RACE prompt, then for each cue-start entry in turn arms one
breakpoint, presses START RACE and logs every hit until the quiet period ends: the cue name (`r5`
for the announcer queue and the bank-cue player), `r3..r6`, the race manager's phase
(`*(*0x008a6814 + 0x1970)`: 1 is the countdown, 2 the race, `docs/ghidra/functions/ps3-hdfury-eu/
countdown-voice.md`), and the SCREAM master tick (`0x013bc500`, 240 a second of **audio** time,
which runs while the game thread is parked, so it orders hits and says nothing of game time).

    scripts/emu-env.sh <lane> 94 <gdb-port>; source data/scratch/<lane>/env.sh
    export XDG_CACHE_HOME=/tmp/oag-<lane>-cache
    uv run --with evdev python3 scripts/emu-run.py -- \\
        python3 -u scripts/hd-countdown-cues.py [--out DIR] [--which announcer,bank,named]
"""

import argparse
import importlib.util
import json
import os
import sys
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(ROOT / "scripts"))


def _load(name, filename):
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / filename)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


drive = _load("rpcs3_drive", "rpcs3-drive.py")
flare = _load("hd_flare_owner_break", "hd-flare-owner-break.py")
from rpcs3_debugger import Debugger  # noqa: E402

IMAGE = ROOT / "data/images/hdfury-ps3-eu-dec.iso"
ENTRIES = {
    "announcer": 0x00310BD8,  # Sound_QueueAnnouncerCue(manager, bank slot, name, priority)
    "bank": 0x002FFA58,       # Sound_PlayBankCue(bank, slot, name, 0x400, ...)
    "named": 0x0062C500,      # Sound_PlayNamedCue trampoline
}
GDB_PORT = int(os.environ.get("OAG_RPCS3_GDB", "127.0.0.1:2345").rsplit(":", 1)[1])
SCREAM_TICK = 0x013BC500
RACE_MANAGER_SLOT = 0x008A6814
PHASE_AT = 0x1970


def cstring(dbg, address, limit=40):
    if not 0x00010000 <= address < 0x0C000000:
        return None
    try:
        raw = dbg.read(address, limit)
    except Exception:
        return None
    text = raw.split(b"\0")[0]
    return text.decode("latin-1") if text and all(32 <= b < 127 for b in text) else None


def phase(dbg):
    """Not read: the singleton slot's content did not resolve to a manager (null on every hit)."""
    return None, None


def readout(dbg, regs, which):
    row = {"which": which, "scream_tick": flare.u32(dbg.read(SCREAM_TICK, 4))}
    row["phase"], row["manager"] = phase(dbg)
    for n in range(3, 9):
        value = flare.gpr(regs, n) & 0xFFFFFFFF
        row["r%d" % n] = "%#x" % value
        text = cstring(dbg, value)
        if text:
            row["r%d_str" % n] = text
    return row


def collect(session, dbg, breaker, which, address, quiet, budget, out):
    rows = []
    print("== %s at %#x" % (which, address), flush=True)
    dbg.pause()
    dbg.drain()
    session.pad.set("cross", True)
    end = time.time() + budget
    last = time.time()
    while time.time() < end and time.time() - last < quiet:
        tid, regs = breaker.stop_at(address, tries=2, slice_seconds=1.0)
        if tid is None:
            continue
        row = readout(dbg, regs, which)
        rows.append(row)
        last = time.time()
        print("  " + json.dumps(row, sort_keys=True), flush=True)
        (out / ("%s.json" % which)).write_text(json.dumps(rows, indent=1))
    session.pad.set("cross", False)
    return rows


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--out", type=Path, required=True, help="a scratch directory")
    ap.add_argument("--which", default="announcer,bank,named")
    ap.add_argument("--quiet", type=float, default=240.0,
                    help="seconds without a hit before a run ends")
    ap.add_argument("--budget", type=float, default=420.0)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    with drive.Session(str(IMAGE), str(args.out / "logs"), interpreter=True) as session:
        if not flare.boot_to_main_menu(session):
            return 1
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            print("did not reach a race (%s)" % drive.current_screen(), file=sys.stderr)
            return 1
        flare.wait_for_load(session, args.out)
        for which in args.which.split(","):
            session.restart_race()
            time.sleep(8.0)
            dbg = Debugger(port=GDB_PORT)
            breaker = flare.Breaker(dbg)
            try:
                collect(session, dbg, breaker, which, ENTRIES[which], args.quiet, args.budget,
                        args.out)
            finally:
                try:
                    dbg.resume()
                except Exception:
                    pass
                dbg.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
