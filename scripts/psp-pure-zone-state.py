#!/usr/bin/env python3
"""Walk Wipeout Pure (PSP, USA) into a Zone race on one circuit, hand it to the Autopilot and save a state a few seconds in.

    source data/scratch/<lane>/pp/env.sh
    uv run --with websocket-client python3 scripts/psp-pure-zone-state.py --port $OAG_PPSSPP_PORT --home $PP_HOME \\
        --circuit-index 0 --after 40 --keep data/saves/pure-psp/zone-pro-tozo-autopilot.ppst [--from race|menu]

The route (measured on a fresh profile, `docs/tools/compare-page.md`, "Zone pairs: Wipeout Pure"): the Main Menu's SINGLE PLAYER,
three downs to ZONE (Single Race, Tournament, Time Trial, Zone), `Zone Track Selection` (Pro Tozo, Mallavol, Coridon 12,
Syncopia: `Data\\Zone\\01_Zone` to `04_Zone`; `--circuit-index` downs), and the race loads with no team or class step. A fresh
profile lists Zone as selectable once the dev-unlock byte is written (`--unlock`, `psp-pure-walk.py`'s write at the Main Menu).
`--from race` pauses and quits to the Main Menu first, as `psp-pure-walk.py` does.

The craft is then handed to the game's own driver: at the per-craft weapons loop's tail (`0x0884d0d4`) the player's Autopilot
pickup record (`a1`) gets its running byte `+0x154` set and its timer `+0x15c` raised (`docs/ghidra/functions/psp-pure-usa/autopilot.md`;
the record is wherever `a1` points at that stop: `0x095bb380` in one boot and `0x095bb340` in the next, so it is written to the
state's `.json` sidecar as `autopilot_record`, which `psp-pure-pair.py --zone` reads). The Autopilot is how a Zone race gets to a late zone (about a zone every 10 s); it is not a physics reference. The state is
saved `--after` seconds later, and the HUD's zone and score are then read off a frame (`psp-pure-pair.py --zone`).
"""

import argparse
import json
import os
import struct
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

PROFILE_PTR = 0x08B17794
UNLOCK_BYTE = 0x381
WEAPONS_LOOP_TAIL = 0x0884D0D4
RUNNING = 0x154
TIMER = 0x15C


def regs(dbg):
    out = {}
    for c in dbg.call("cpu.getAllRegs")["categories"]:
        out.update(zip(c["registerNames"], c["uintValues"]))
    return out


def arm_autopilot(dbg, seconds):
    """Set the player's Autopilot pickup running with `seconds` left; returns the record's address."""
    dbg.brk()
    dbg.add_breakpoint(WEAPONS_LOOP_TAIL)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(WEAPONS_LOOP_TAIL, timeout=60)
    record = regs(dbg)["a1"]
    dbg.write(record + RUNNING, b"\x01")
    dbg.write(record + TIMER, struct.pack("<f", seconds))
    dbg.remove_breakpoint(WEAPONS_LOOP_TAIL)
    if dbg.is_stepping():
        dbg.call("cpu.resume")
    return record


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    ap.add_argument("--home", default=os.environ.get("PP_HOME"))
    ap.add_argument("--circuit-index", type=int, default=0, help="downs on Zone Track Selection: 0 Pro Tozo, 1 Mallavol, 2 Coridon 12, 3 Syncopia")
    ap.add_argument("--from", dest="start", choices=("race", "menu"), default="menu")
    ap.add_argument("--unlock", action="store_true", help="write the dev-unlock byte at the Main Menu")
    ap.add_argument("--load-wait", type=float, default=12.0, help="seconds from Track Selection's cross to the race being up")
    ap.add_argument("--autopilot-s", type=float, default=600.0)
    ap.add_argument("--after", type=float, default=40.0, help="seconds of Autopilot flying before the state is saved")
    ap.add_argument("--keep", required=True, help="copy the saved state here (under data/saves/pure-psp/)")
    ap.add_argument("--pace", type=float, default=1.3)
    args = ap.parse_args()
    if not args.home:
        sys.exit("--home (or PP_HOME) must name the instance's own HOME")
    dbg = Debugger(args.port)
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(0.5)

    def tap(*buttons):
        for b in buttons:
            dbg.press(b, duration=6)
            time.sleep(args.pace)

    if args.start == "race":
        tap("start", "down", "down", "cross")
        time.sleep(3.0)
    if args.unlock:
        profile = dbg.read_u32(PROFILE_PTR)
        dbg.write(profile + UNLOCK_BYTE, b"\x01")
    tap("cross", "down", "down", "down", "cross")
    tap(*(["down"] * args.circuit_index + ["cross"]))
    time.sleep(args.load_wait)
    record = arm_autopilot(dbg, args.autopilot_s)
    print("autopilot armed on record %#x for %.0f s" % (record, args.autopilot_s), file=sys.stderr)
    time.sleep(args.after)
    subprocess.run(["uv", "run", "--with", "websocket-client", "python3", HERE / "psp-state.py", "--port", str(args.port),
                    "--home", args.home, "save", "2", "--serial", "UCUS98612_2.00", "--keep", args.keep], check=True)
    Path(args.keep + ".json").write_text(json.dumps({"mode": "zone", "circuit": "Zone/%02d_Zone" % (args.circuit_index + 1),
                                                     "autopilot_record": hex(record)}) + "\n")


if __name__ == "__main__":
    main()
