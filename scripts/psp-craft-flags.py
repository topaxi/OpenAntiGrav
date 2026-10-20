#!/usr/bin/env python3
"""Live census of the craft flag word `craft+0x1c0` on Pulse PSP (USA).

    source data/scratch/<lane>/pp/env.sh
    uv run --with websocket-client python3 scripts/psp-craft-flags.py \\
        --port $OAG_PPSSPP_PORT --out flags.jsonl [--frames 900] [--no-watch]

Run from a Time Trial grid state (`data/saves/pulse-psp/time-trial-*-grid.ppst`,
loaded with `psp-state.py load 1 --file ...`). Once a frame, at
`Ship_UpdateSideshiftInput_q` (`0x08846a54`) for the player's entity, it logs the
flag word, the craft state `craft+0x2a4`, the sideshift timers and lockout
(`entity+0x89c/8a0/8a4/8a8/8ac`), the pad-fade clock `craft+0x2cc`, the pickup
object's fire word and held id (`*(entity+0x4c)+0x1b8/+0x1bc`) and the forward
speed `craft+0x2ec`.

With the write watch on (the default) a memory breakpoint on `craft+0x1c0`
(write, on change) stops at every store that changes the word; the pc of each is
logged as a `write` row, so a writer missing from the static census shows up as
an unknown pc.

The script drives the pad itself: cross held throughout, a double tap of `L` and
then of `R` during the countdown (the veteran gesture, scheme byte `0x08ab0cd0`
is logged), the same two double taps after the release, and `SQUARE` once the
pickup slot holds a weapon (the free Turbo of a Time Trial).
"""

import argparse
import json
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

ENTRY = 0x08846A54  # Ship_UpdateSideshiftInput_q, a0 = entity
SCHEME = 0x08AB0CD0


def regs(dbg):
    r = dbg.call("cpu.getAllRegs")
    gpr = next(c for c in r["categories"] if c["name"] == "GPR")
    return dict(zip(gpr["registerNames"], gpr["uintValues"]))


def snapshot(dbg, ent, craft):
    pickup = dbg.read_u32(ent + 0x4C)
    return {
        "flags": dbg.read_u32(craft + 0x1C0),
        "state": dbg.read_u32(craft + 0x2A4),
        "tap_l": round(dbg.read_f32(ent + 0x89C), 3),
        "tap_r": round(dbg.read_f32(ent + 0x8A0), 3),
        "shift_l": round(dbg.read_f32(ent + 0x8A4), 3),
        "shift_r": round(dbg.read_f32(ent + 0x8A8), 3),
        "lockout": round(dbg.read_f32(ent + 0x8AC), 3),
        "pad_clock": round(dbg.read_f32(craft + 0x2CC), 3),
        "speed": round(dbg.read_f32(craft + 0x2EC), 3),
        "stun": round(dbg.read_f32(craft + 0x290), 3),
        "slow": round(dbg.read_f32(craft + 0x2E0), 3),
        "fire": dbg.read_u32(pickup + 0x1B8) if pickup else None,
        "held": dbg.read_u32(pickup + 0x1BC) if pickup else None,
    }


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--frames", type=int, default=900)
    ap.add_argument("--no-watch", action="store_true")
    args = ap.parse_args()
    out = open(args.out, "w")

    with Debugger(args.port) as dbg:
        dbg.brk()
        dbg.add_breakpoint(ENTRY)
        player = craft = None
        frame = 0
        go = None
        fired = False
        taps = {}
        writes = {}
        watching = False
        try:
            while frame < args.frames:
                dbg.call("cpu.status")
                dbg.pending = []
                dbg.call("cpu.resume")
                while True:
                    msg = dbg._recv()
                    if msg is not None and msg.get("event") == "cpu.stepping":
                        break
                pc = msg.get("pc")
                if pc != ENTRY:
                    r = regs(dbg)
                    key = "0x%08x" % pc
                    writes[key] = writes.get(key, 0) + 1
                    out.write(json.dumps({"kind": "write", "frame": frame, "pc": key, "ra": "0x%08x" % r["ra"],
                                          "flags": dbg.read_u32(craft + 0x1C0)}) + "\n")
                    continue
                ent = regs(dbg)["a0"]
                if player is None:
                    if dbg.read_u32(ent + 0x78) == 0:
                        continue
                    player = ent
                    craft = dbg.read_u32(ent + 0x94)
                    if not args.no_watch:
                        dbg.call("memory.breakpoint.add", address=craft + 0x1C0, size=4, enabled=True,
                                 log=False, read=False, write=True, change=True)
                        watching = True
                    print("player entity 0x%08x craft 0x%08x scheme %d" % (player, craft, dbg.read_u8(SCHEME)),
                          file=sys.stderr)
                if ent != player:
                    continue
                row = snapshot(dbg, ent, craft)
                if go is None and row["state"] == 1:
                    go = frame
                    for t, b in ((60, "ltrigger"), (66, "ltrigger"), (150, "rtrigger"), (156, "rtrigger")):
                        taps[frame + t] = b
                if frame == 0:
                    for t, b in ((60, "ltrigger"), (66, "ltrigger"), (150, "rtrigger"), (156, "rtrigger")):
                        taps[t] = b
                pad = taps.get(frame)
                buttons = {"cross": True, "ltrigger": False, "rtrigger": False, "square": False}
                if pad:
                    buttons[pad] = True
                if go is not None and not fired and row["held"] not in (None, 0xFFFFFFFF) and frame > go + 240:
                    buttons["square"] = True
                    fired = True
                    pad = (pad or "") + "+square"
                dbg.hold(**buttons)
                row.update({"kind": "tick", "frame": frame, "pad": pad, "go": go})
                out.write(json.dumps(row) + "\n")
                out.flush()
                if frame % 100 == 0:
                    print("frame %d flags 0x%x state %d" % (frame, row["flags"], row["state"]), file=sys.stderr)
                frame += 1
        finally:
            dbg.brk()
            if watching:
                dbg.call("memory.breakpoint.remove", address=craft + 0x1C0, size=4)
            dbg.remove_breakpoint(ENTRY)
            dbg.hold(cross=False, ltrigger=False, rtrigger=False, square=False)
            dbg.resume()
            out.write(json.dumps({"kind": "writers", "pcs": writes}) + "\n")
            print("writers: %s" % writes, file=sys.stderr)


if __name__ == "__main__":
    t0 = time.time()
    main()
    print("wall %.0fs" % (time.time() - t0), file=sys.stderr)
