#!/usr/bin/env python3
"""The original's launch boost, once per frame: which multiplier, when thrust first lands.

`Ship_UpdateStartBoost` (0x0883fdec) writes `craft+0x294` from the start-grade
`player+0x36c`; `FUN_0882773c` writes the grade from `race_manager+0x2bc`, the
seconds since GO, on the one frame `craft+0x2d5` pulses (the first frame the
control record's throttle is non-zero). This logs every one of those words, per
frame, from the restart to GO + `--after-go`, so the law can be checked against
the original rather than argued from the decompile.

    python3 scripts/psp-launch-boost.py --port 45682 --press-after-go 10 \\
        --out data/scratch/<lane>/launch-p10.json

`--press-after-go K` holds cross at the K-th row after the craft enters state 1
(`craft+0x2a4 == 1`, the frame before the throttle word steps). `--hold` holds
cross from the restart instead, so thrust is down through the whole countdown.
Neither, and nothing is ever pressed.

Rows are taken at `Weapons_DispatchFire`, once per craft per frame, which is
once per frame in a Time Trial. Raw captures are derived game data: write them
under `data/`, never commit them.
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
    parser.add_argument("--press-after-go", type=int, default=None)
    parser.add_argument("--after-go", type=int, default=100)
    parser.add_argument("--max-rows", type=int, default=2400)
    parser.add_argument("--timeout", type=float, default=90.0)
    args = parser.parse_args()
    args.out.parent.mkdir(parents=True, exist_ok=True)

    dbg = Debugger(args.port)
    rows = []
    state1 = None
    first_ticks = None
    pressed = False
    try:
        restart(dbg, args.hold)
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
            fwd = struct.unpack_from("<3f", blob, 0x20)
            pos = struct.unpack_from("<3f", blob, 0x30)
            vel = struct.unpack_from("<3f", blob, 0x140)
            c = struct.unpack("<%dI" % (0x340 // 4), dbg.read(craft, 0x340))
            cf = lambda off: struct.unpack("<f", struct.pack("<I", c[off // 4]))[0]  # noqa: E731
            record = dbg.read_u32(craft + 0x78)
            control = list(struct.unpack("<6f", dbg.read(record, 0x18))) if ram(record) else None
            clock = struct.unpack("<f", dbg.read(manager + 0x2BC, 4))[0]
            timer = struct.unpack("<f", dbg.read(player + 0x888, 4))[0]
            grade = dbg.read_u32(player + 0x36C)
            row = {
                "frame": round((ticks - first_ticks) / CYCLES_PER_FRAME, 2),
                "state": c[0x2A4 // 4],
                "flags": c[0x1C0 // 4],
                "mul294": cf(0x294),
                "edge_2d5": c[0x2D4 // 4] >> 8 & 0xFF,
                "latch_2d4": c[0x2D4 // 4] & 0xFF,
                "air_2d8": cf(0x2D8),
                "air_2dc": cf(0x2DC),
                "throttle_2b8": cf(0x2B8),
                "thrust_328": cf(0x328),
                "control": control,
                "mgr_clock": clock,
                "start_timer_888": timer,
                "grade_36c": grade,
                "pos": pos, "fwd": fwd, "vel": vel,
            }
            rows.append(row)
            if state1 is None and row["state"] == 1:
                state1 = len(rows) - 1
                print("state 1 at row %d" % state1, file=sys.stderr)
            if (not pressed and args.press_after_go is not None and state1 is not None
                    and len(rows) - 1 - state1 >= args.press_after_go):
                dbg.hold(cross=True)
                pressed = True
                print("pressed at row %d" % (len(rows) - 1), file=sys.stderr)
            if state1 is not None and len(rows) - state1 > args.after_go:
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
                                        "state1_row": state1, "rows": rows}))
        print("wrote %d rows, state-1 row %s -> %s" % (len(rows), state1, args.out),
              file=sys.stderr)


if __name__ == "__main__":
    main()
