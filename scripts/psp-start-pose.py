#!/usr/bin/env python3
"""The original's craft pose, once per frame, from the grid to GO and beyond.

The question this answers: where is the player's craft, and which way does it
face, on the frame the race first runs, on the frame the gantry drops (GO), and
every frame between. `data/traces/` captures all start at "whenever the capture
attached", and they disagree with each other on heading at their tick 0 by up to
4 degrees, so a single capture cannot say which frame is the start.

Method: `RESTART RACE` is walked from the pause menu, then the breakpoint on
`Weapons_DispatchFire` (once per frame per craft, the same probe
`psp-weapon-pair.py` uses to find the player) logs, for the **player's** craft
only, the body's three axes, position, velocity, angular momentum, the craft's
throttle word and the animation clock. Rows carry the PSP cycle counter's frame
index so a row's time is the emulator's own rather than the wall clock's.

    python3 scripts/psp-start-pose.py --port 45681 --out data/scratch/<lane>/orig-tt.json

`--hold` holds cross from just after the restart confirm, which dismisses the
track description by itself and keeps thrust on through the countdown, so the
first non-zero throttle word is GO. Without it nothing is held and the
description has to be dismissed by hand (`--dismiss-after` seconds after the
restart, one press), and GO is never seen.

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

WEAPONS_DISPATCH_FIRE = 0x08861814
RACE_MANAGER = 0x08B317B4
PLAYER_OFFSET = 0x2C0
ENTITY_CRAFT = 0x94
BODY_OFFSET = 0x1CC
THROTTLE = 0x2B8
DT = 0x1C8
G_INGAME = 0x08AB0818
INGAME_CLOCK = 0x40
CYCLES_PER_FRAME = 222_000_000 / 59.940059940059946


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


def restart(dbg, hold):
    dbg.resume()
    dbg.hold(cross=False)
    if "Pause" not in dbg.state_name():
        dbg.press("start", duration=6)
        time.sleep(1.5)
    for _ in range(4):
        dbg.press("down", duration=4)
        time.sleep(0.35)
    for _ in range(6):
        dbg.press("cross", duration=6)
        time.sleep(1.5)
        if "Pause" not in dbg.state_name():
            break
    else:
        raise SystemExit("RESTART RACE never took")
    if hold:
        dbg.hold(cross=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--hold", action="store_true")
    parser.add_argument("--no-restart", action="store_true")
    parser.add_argument("--after-go", type=int, default=150,
                        help="keep logging this many frames after GO")
    parser.add_argument("--release-at", type=int, default=None,
                        help="let go of cross at this row, to see whether holding thrust through "
                        "the countdown changes anything")
    parser.add_argument("--press-at", type=int, default=None,
                        help="hold cross again at this row (after --release-at): a launch whose "
                        "thrust was not already held when the light went green")
    parser.add_argument("--full", action="store_true",
                        help="also keep the craft's first 0x400 bytes as raw words in every row")
    parser.add_argument("--max-rows", type=int, default=2400)
    parser.add_argument("--timeout", type=float, default=90.0)
    args = parser.parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)

    dbg = Debugger(args.port)
    rows = []
    go = None
    first_ticks = None
    try:
        if not args.no_restart:
            restart(dbg, args.hold)
        elif args.hold:
            dbg.resume()
            dbg.hold(cross=True)
        for _, _ in dbg.each_hit(WEAPONS_DISPATCH_FIRE, 100000, timeout=args.timeout):
            manager = dbg.read_u32(RACE_MANAGER)
            if not ram(manager):
                continue
            player = dbg.read_u32(manager + PLAYER_OFFSET)
            if not ram(player):
                continue
            craft = dbg.read_u32(player + ENTITY_CRAFT)
            if not ram(craft):
                continue
            body = dbg.read_u32(craft + BODY_OFFSET)
            if not ram(body):
                continue
            ticks = dbg.call("cpu.status")["ticks"]
            if first_ticks is None:
                first_ticks = ticks
            blob = dbg.read(body, 0x400)
            f = lambda at, n: list(struct.unpack_from("<%df" % n, blob, at))  # noqa: E731
            throttle = struct.unpack("<f", dbg.read(craft + THROTTLE, 4))[0]
            dt = struct.unpack("<f", dbg.read(craft + DT, 4))[0]
            row = {
                "frame": round((ticks - first_ticks) / CYCLES_PER_FRAME, 2),
                "craft": craft,
                "dt": dt,
                "throttle": throttle,
                "left": f(0x00, 3), "up": f(0x10, 3), "fwd": f(0x20, 3), "pos": f(0x30, 3),
                "vel": f(0x140, 3), "omega": f(0x150, 3), "avel": f(0x160, 3),
            }
            row["craft_block"] = list(struct.unpack("<32f", dbg.read(craft + 0x140, 0x80)))
            if args.full:
                row["craft_full"] = list(struct.unpack("<256I", dbg.read(craft, 0x400)))
            record = dbg.read_u32(craft + 0x78)
            if ram(record):
                row["control_record"] = list(struct.unpack("<12f", dbg.read(record, 0x30)))
            row["craft_2b0"] = list(struct.unpack("<12f", dbg.read(craft + 0x2B0, 0x30)))
            row["flags_1c0"] = dbg.read_u32(craft + 0x1C0)
            row["state_2a4"] = dbg.read_u32(craft + 0x2A4)
            ingame = dbg.read_u32(G_INGAME)
            if ram(ingame):
                row["clock"] = struct.unpack("<f", dbg.read(ingame + INGAME_CLOCK, 4))[0]
            rows.append(row)
            if args.release_at is not None and len(rows) == args.release_at:
                dbg.hold(cross=False)
            if args.press_at is not None and len(rows) == args.press_at:
                dbg.hold(cross=True)
            if go is None and throttle > 0.0 and (args.release_at is None or len(rows) > args.release_at):
                go = len(rows) - 1
                print("GO at row %d (frame %.1f)" % (go, row["frame"]), file=sys.stderr)
            if go is not None and len(rows) - go > args.after_go:
                break
            if len(rows) >= args.max_rows:
                break
    finally:
        try:
            dbg.resume()
            dbg.hold(cross=False)
        except Exception:  # noqa: BLE001
            pass
        args.out.write_text(json.dumps({"args": {k: str(v) for k, v in vars(args).items()},
                                        "go_row": go, "rows": rows}))
        print("wrote %d rows, go row %s -> %s" % (len(rows), go, args.out), file=sys.stderr)


if __name__ == "__main__":
    main()
