#!/usr/bin/env python3
"""Walk Wipeout Pure (PSP, USA) to a Time Trial on one circuit and save a state a few seconds after the grid, by pad presses.

    source data/scratch/<lane>/pp/env.sh
    HOME=$PP_HOME uv run --with websocket-client python3 scripts/psp-pure-walk.py --port $OAG_PPSSPP_PORT \\
        --track-index 2 --keep data/saves/pure-psp/time-trial-chenghou-vector-feisar-restart.ppst [--from race|menu]

The route (measured 2026-10-22 on a fresh profile, class Vector, league Alpha, the only ones unlocked; Feisar is the
craft the Team Selection opens on): `--from race` pauses (`start`), Quit (`down down cross`) to the Main Menu; then
Single Player, Time Trial (`down down`), Vector, Alpha, `track-index` times `down` on Track Selection, Feisar, and the
race loads. Back is `triangle` on this disc, not `circle` (the screens print it). Each press holds 6 frames and waits
1.3 s: a faster pace dropped presses at screen changes. After the load the intro fly-past and countdown run (the
clock reads 0.0 until GO) and the walk saves `--after` seconds later with the craft untouched on the grid.
Track index 0 Vineta K, 1 Modesto Heights, 2 Chenghou Project, 3 Blue Ridge.
"""

import argparse
import os
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
PROFILE_PTR = 0x08B17794  # the loaded profile object's pointer (Definition_IsUnlocked, 0x088b43d4: lui 0x08b1, lw 0x7794)
UNLOCK_BYTE = 0x381  # nonzero: every circuit, league and team passes the unlock gate
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    ap.add_argument("--home", default=os.environ.get("PP_HOME"))
    ap.add_argument("--track-index", type=int, required=True)
    ap.add_argument("--league", type=int, default=0, help="downs on League Selection: 0 Alpha, 1 Beta, 2 Classic (the last two need --unlock)")
    ap.add_argument("--unlock", action="store_true",
                    help="write the dev-unlock byte (`*(*0x08b17794 + 0x381) = 1`) at the Main Menu so League Selection lists Beta and Classic")
    ap.add_argument("--from", dest="start", choices=("race", "menu"), default="race")
    ap.add_argument("--keep", required=True, help="copy the saved state here (under data/saves/pure-psp/)")
    ap.add_argument("--after", type=float, default=16.0, help="seconds to run after Track Selection's cross before saving")
    ap.add_argument("--pace", type=float, default=1.3)
    args = ap.parse_args()
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
        print("dev-unlock byte set (profile %#x)" % profile, file=sys.stderr)
    tap("cross", "down", "down", "cross", "cross")
    tap(*(["down"] * args.league + ["cross"]))
    tap(*(["down"] * args.track_index + ["cross"]))
    tap("cross")
    time.sleep(args.after)
    subprocess.run(["uv", "run", "--with", "websocket-client", "python3", HERE / "psp-state.py", "--port", str(args.port),
                    "--home", args.home, "save", "2", "--serial", "UCUS98612_2.00", "--keep", args.keep], check=True)


if __name__ == "__main__":
    main()
