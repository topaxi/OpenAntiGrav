#!/usr/bin/env python3
"""Wreck one opponent in a live PPSSPP race and log what becomes of it, frame by frame.

The measurement behind `docs/ghidra/functions/psp-pulse-usa/shield.md`'s "state 6" section: does a
single race's destroyed AI craft ever leave state 6? Run it with the race already live (after
`scripts/psp-drive.py menu --single-race`, which leaves the player on the grid with the countdown
sat out), never during a countdown.

    uv run --with websocket-client scripts/psp-state6-watch.py --port 45491 --out DIR \\
        --slot 3 --inject damage --seconds 15 [--watch]

`--inject state4` calls `Ship_SetState(entity, 4)`; `--inject damage` calls `Ship_Damage(1e6, entity,
0, 0, 0)` so the placement and elimination bookkeeping that follows the destroy in `Ship_Damage`'s own
tail runs too, which is the closest thing to a natural death a stop-and-call can make.

The loop breaks at `Ship_UpdateCraft` (one stop per craft per frame). A row is logged at the target's
own stop (`src: target`) and at the player's (`src: player`), so a craft removed from the update list
still reads (state, timer, flags, body position, shield) from the player's stop. `--watch` arms a write
watchpoint on `entity+0x8C` after the call and logs the writer's `pc`/`ra` for every change.

Writes `DIR/log.json`. Raw captures are derived game data: keep them under `data/`.
"""

import argparse
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "wreck", Path(__file__).resolve().parent / "psp-wreck-capture.py"
)
wreck = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(wreck)

SHIP_DAMAGE = 0x088439AC
SHIELD = 0x88
STATE = 0x8C
FLAGS = 0x860
TIMER = 0x874
CLASS = 0x368


def row(dbg, entity, src, frame, since):
    craft = dbg.read_u32(entity + 0x94)
    body = dbg.read_u32(craft + 0x1CC)
    out = {
        "src": src,
        "frame": frame,
        "since": since,
        "state": dbg.read_u32(entity + STATE),
        "timer": struct.unpack("<f", dbg.read(entity + TIMER, 4))[0],
        "flags": hex(dbg.read_u32(entity + FLAGS)),
        "shield": struct.unpack("<f", dbg.read(entity + SHIELD, 4))[0],
        "pos": [round(v, 2) for v in struct.unpack("<3f", dbg.read(body + 0x30, 12))],
        "live_is_wreck": dbg.read_u32(entity + 0x8B0) == dbg.read_u32(entity + 0x8B8),
        "throttle": round(struct.unpack("<f", dbg.read(craft + 0x2B8, 4))[0], 3),
        "speed": round(struct.unpack("<f", dbg.read(craft + 0x2EC, 4))[0], 2),
    }
    return out


def call_at_stop(dbg, setup, target_pc, timeout=30.0):
    """Run `target_pc` from the current Ship_UpdateCraft stop with `ra` back at that stop."""
    saved = wreck.regs(dbg)
    fsaved = wreck.pair.fpu(dbg)
    for name, value in setup.items():
        if name.startswith("f"):
            dbg.call("cpu.setReg", category=1, name=name, value=value)
        else:
            dbg.call("cpu.setReg", name=name, value=value)
    dbg.call("cpu.setReg", name="ra", value=wreck.SHIP_UPDATE_CRAFT)
    dbg.call("cpu.setReg", name="pc", value=target_pc)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(wreck.SHIP_UPDATE_CRAFT, timeout=timeout)
    for name, value in saved.items():
        if name != "zero":
            dbg.call("cpu.setReg", name=name, value=value)
    for name, value in fsaved.items():
        dbg.call("cpu.setReg", category=1, name=name, value=value)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--slot", type=int, default=-1, help="manager grid slot to wreck (default: nearest opponent)")
    ap.add_argument("--inject", choices=("state4", "damage"), default="state4")
    ap.add_argument("--inject-frame", type=int, default=30, help="target-stop frames to wait before the call")
    ap.add_argument("--seconds", type=float, default=15.0, help="game seconds to log after the call (60 frames each)")
    ap.add_argument("--watch", action="store_true", help="write watchpoint on entity+0x8C after the call")
    ap.add_argument("--timeout", type=float, default=60.0, help="wall seconds to wait for each stop")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    class A:  # what pick_entity reads
        slot = args.slot
        nearest = args.slot < 0

    dbg = Debugger(args.port)
    log = []
    watch_hits = []
    watching = False
    try:
        dbg.brk()
        manager = dbg.read_u32(wreck.RACE_MANAGER)
        player = dbg.read_u32(manager + 0x2C0)
        player_craft = dbg.read_u32(player + 0x94)
        entity = wreck.pick_entity(dbg, A)
        craft = dbg.read_u32(entity + 0x94)
        print("player entity %#x craft %#x; target entity %#x craft %#x" % (player, player_craft, entity, craft), file=sys.stderr)
        header = {
            "target_entity": hex(entity),
            "target_class_0x368": dbg.read_u32(entity + CLASS),
            "grid_slots": [hex(dbg.read_u32(manager + 0x78 + 4 * i)) for i in range(8)],
        }
        log.append({"header": header})
        injected = None
        frame = 0
        wall0 = time.time()
        dbg.add_breakpoint(wreck.SHIP_UPDATE_CRAFT)
        end_frame = None
        while True:
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            msg = None
            end = time.time() + args.timeout
            while time.time() < end:
                msg = dbg._recv()
                if msg is None:
                    continue
                if msg.get("event") == "cpu.stepping" and msg.get("pc") == wreck.SHIP_UPDATE_CRAFT:
                    break
                if msg.get("event") == "cpu.stepping" and watching and msg.get("pc") != wreck.SHIP_UPDATE_CRAFT:
                    r = wreck.regs(dbg)
                    watch_hits.append(
                        {
                            "frame": frame,
                            "since": None if injected is None else frame - injected,
                            "pc": hex(msg.get("pc")),
                            "ra": hex(r["ra"]),
                            "value_now": dbg.read_u32(entity + STATE),
                            "a0": hex(r["a0"]),
                        }
                    )
                    break
                msg = None
            else:
                log.append({"timeout_after_frame": frame, "wall_s": round(time.time() - wall0, 1)})
                print("timed out waiting for a stop at frame %d" % frame, file=sys.stderr)
                break
            if msg is None:
                continue
            if msg.get("pc") != wreck.SHIP_UPDATE_CRAFT:
                continue  # a watch hit, already logged; resume
            a0 = wreck.regs(dbg)["a0"]
            if a0 == craft:
                if injected is None and frame >= args.inject_frame:
                    log.append(dict(row(dbg, entity, "before", frame, 0)))
                    if args.inject == "state4":
                        call_at_stop(dbg, {"a0": entity, "a1": 4}, wreck.SET_STATE)
                    else:
                        call_at_stop(dbg, {"a0": entity, "a1": 0, "a2": 0, "a3": 0, "f12": struct.unpack("<I", struct.pack("<f", 1.0e6))[0]}, SHIP_DAMAGE)
                    injected = frame
                    if args.watch:
                        dbg.call("memory.breakpoint.add", address=entity + STATE, size=4, enabled=True,
                                 log=False, **{"break": True}, read=False, write=True, change=True)
                        watching = True
                    log.append(dict(row(dbg, entity, "after-call", frame, 0)))
                    print("injected %s at frame %d, wall %.0fs" % (args.inject, frame, time.time() - wall0), file=sys.stderr)
                r = row(dbg, entity, "target", frame, None if injected is None else frame - injected)
                log.append(r)
                frame += 1
                if injected is not None and frame - injected >= args.seconds * 60:
                    break
            elif a0 == player_craft and injected is not None:
                log.append(row(dbg, entity, "player", frame, frame - injected))
            if frame % 60 == 0 and a0 == craft:
                print("frame %d state %d wall %.0fs" % (frame, log[-1]["state"], time.time() - wall0), file=sys.stderr)
        if watching:
            dbg.call("memory.breakpoint.remove", address=entity + STATE, size=4)
    finally:
        try:
            dbg.brk()
            dbg.remove_breakpoint(wreck.SHIP_UPDATE_CRAFT)
            dbg.resume()
        except Exception as error:  # noqa: BLE001
            print("cleanup: %s" % error, file=sys.stderr)
        (args.out / "log.json").write_text(json.dumps({"rows": log, "watch": watch_hits}, indent=1))
        dbg.close()
    print("wrote", args.out / "log.json")


if __name__ == "__main__":
    main()
