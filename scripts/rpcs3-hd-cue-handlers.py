#!/usr/bin/env python3
"""Log every SCREAM sound a running Wipeout HD/Fury race starts, without stopping the game.

`Scream_TickHandlers` (`0x0062e098`) walks the live handler list at `*(*(0x008c00fc) + 0x24)`,
next at `handler + 0x70`; an `SBlk` handler has type nibble 5. This polls that list through the
`serve`'s memory channel (about 150 polls a second, the race runs at full speed) and prints one
JSON line per new handler:

  bank_cues / bank_cmds   the bank's cue and command counts (header +0x16/+0x18), which name the file
  index                   the cue's position in the bank's 12-byte cue table (handler +0x04)
  vars                    the four cue-variable bytes the guards read (handler +0xa4..+0xa7)
  vol16                   the handler's volume word (+0x16; 1024 is a full-volume, unplaced cue)
  floats                  the eight floats of the request's parameter block (handler +0x48..)

A cue that replaces itself with a child (opcode 0x08: `.COLLISIONS`, `.GRIND`, `ready`) shows as
the child, because the handler's cue pointer is the child's by the time it is read.

`--sweep 0,1,...` also arms each pickup state on the player's slot (`*(ship + 0x5edc) + 0x204`
and `+0x208`, see docs/reverse-engineering/rpcs3-capture.md) and presses triangle, tagging the rows
with the armed state, so every weapon's cues start once. States 11 and up crash the game.

Evidence page: docs/ghidra/functions/ps3-hdfury-eu/cue-variables.md.

    scripts/emu-env.sh <lane> <display> <gdb-port>; source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/<a race state>.SAVESTAT.zst
    export OAG_RPCS3_ATTACH=1
    uv run --with evdev python3 scripts/emu-run.py -- \\
        python3 -u scripts/rpcs3-hd-cue-handlers.py --out <dir> --seconds 200
"""
import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)

GUEST = 0x300000000
LIST_SLOT = 0x008C00FC
SCREAM_TICK = 0x013BC500
SLOT_OFFSET = 0x5EDC
SLOT_STATE = 0x204
HANDLER_LEN = 0x190
PRINTED = ("state", "t", "tick", "bank_cues", "bank_cmds", "index", "vars", "vol16", "floats")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--out", type=Path, required=True, help="directory for handlers.json")
    ap.add_argument("--seconds", type=float, default=200.0)
    ap.add_argument("--sweep", default="", help="comma-separated pickup states to arm and fire in turn")
    ap.add_argument("--dwell", type=float, default=7.0, help="seconds per swept state")
    ap.add_argument("--interpreter", action="store_true", help="the serve runs the PPU interpreter")
    ap.add_argument("--no-pad", action="store_true", help="leave the pad alone (no held cross)")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    states = [int(x) for x in args.sweep.split(",") if x]
    if states:
        args.seconds = args.dwell * len(states) + 2
    rows, banks, seen = [], {}, set()

    def save():
        (args.out / "handlers.json").write_text(json.dumps({"rows": rows, "banks": banks}))

    with drive.Session("x", str(args.out / "logs"), interpreter=args.interpreter) as session:
        mem = session.open_mem()

        def rd(address, n):
            mem.seek(GUEST + address)
            return mem.read(n)

        def u32(address):
            return struct.unpack(">I", rd(address, 4))[0]

        root = u32(LIST_SLOT)
        slot = None
        if states:
            import rpcs3_place as place

            class Reader:
                def read(self, address, n):
                    return rd(address, n)

            ship, _body = place.find_player(Reader())
            slot = u32(ship + SLOT_OFFSET)
            print("player ship %#x, pickup slot %#x" % (ship, slot), flush=True)
        if not args.no_pad:
            session.pad.set("cross", True)
        started = time.time()
        armed, fired, polls = None, False, 0
        while time.time() - started < args.seconds:
            now = time.time()
            if states:
                step = int((now - started) / args.dwell)
                if step < len(states) and armed != (step, states[step]):
                    armed, fired = (step, states[step]), False
                    mem.seek(GUEST + slot + SLOT_STATE)
                    mem.write(struct.pack(">ii", states[step], states[step]))
                    print("armed state %d at %.2f s" % (states[step], now - started), flush=True)
                if armed and not fired and (now - started) - armed[0] * args.dwell > 0.6:
                    fired = True
                    session.pad.press("triangle", 0.15)
            polls += 1
            try:
                tick = u32(SCREAM_TICK)
                handler = u32(root + 0x24)
                hops = 0
                while handler and hops < 200:
                    hops += 1
                    raw = rd(handler, HANDLER_LEN)
                    following = struct.unpack_from(">I", raw, 0x70)[0]
                    key = (handler, struct.unpack_from(">I", raw, 0x184)[0])
                    if key not in seen and raw[0] & 0x1F == 5:
                        seen.add(key)
                        cue = struct.unpack_from(">I", raw, 4)[0]
                        bank = struct.unpack_from(">I", raw, 0xB0)[0]
                        name = "%#x" % bank
                        if name not in banks:
                            head = rd(bank, 0x40)
                            banks[name] = {"cues": struct.unpack_from(">H", head, 0x16)[0],
                                           "commands": struct.unpack_from(">H", head, 0x18)[0],
                                           "cue_table": struct.unpack_from(">I", head, 0x1C)[0]}
                        table = banks[name]["cue_table"]
                        row = {"state": armed[1] if armed else None, "t": round(now - started, 3), "tick": tick,
                               "bank_cues": banks[name]["cues"], "bank_cmds": banks[name]["commands"],
                               "index": (cue - (table if table > 0x10000 else bank + table)) // 12,
                               "vars": list(struct.unpack("4b", raw[0xA4:0xA8])),
                               "vol16": struct.unpack_from(">h", raw, 0x16)[0],
                               "floats": [round(x, 4) for x in struct.unpack_from(">8f", raw, 0x48)]}
                        rows.append(row)
                        print(json.dumps({k: row[k] for k in PRINTED}), flush=True)
                    handler = following
            except (OSError, struct.error, KeyError) as error:
                print("poll error %r" % error, flush=True)
                time.sleep(0.2)
            if polls % 500 == 0:
                save()
        save()
        print("%d polls, %d handlers, %.1f s" % (polls, len(rows), time.time() - started), flush=True)
        if not args.no_pad:
            session.pad.set("cross", False)
    return 0


if __name__ == "__main__":
    sys.exit(main())
