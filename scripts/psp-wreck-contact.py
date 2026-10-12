#!/usr/bin/env python3
"""Does a destroyed craft still take part in craft-against-craft contact? Measure it on a live PPSSPP.

`Body_ResolveContactPair` (`0x0884ef30`) is called from `Body_StepWorld` once per narrowphase contact
between two bodies. This script teleports one opponent's body on top of the (stationary) player's and
counts the breakpoint hits at `0x0884ef30` whose two body arguments are exactly that pair:

1. `control`: the opponent alive, state 1 - the contact must fire, or the method measures nothing.
2. `wreck`: the same opponent after `Ship_Damage(1e6, ...)` has run it through states 4, 5 and 6.

    uv run --with websocket-client scripts/psp-wreck-contact.py --port 45095 --out DIR --slot 3

Run with the race already live (`scripts/psp-drive.py menu --single-race`). Only one execution
breakpoint fires at a time on this PPSSPP, so the two stops (`Ship_UpdateCraft`, then the pair
resolver) are never armed together. Writes `DIR/contact.json`; derived data, keep it under `data/`.
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
    "watch", Path(__file__).resolve().parent / "psp-state6-watch.py"
)
watch = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(watch)
wreck = watch.wreck

PAIR_RESOLVER = 0x0884EF30
SINGLE_RESOLVER = 0x0884E968
BODY_SET_POSITION = 0x0884D840
UPDATE_CRAFT = wreck.SHIP_UPDATE_CRAFT


def f32bits(value):
    return struct.unpack("<I", struct.pack("<f", value))[0]


def stop_at(dbg, craft, timeout):
    """Resume until Ship_UpdateCraft stops for `craft`."""
    while True:
        dbg.call("cpu.status")
        dbg.pending = []
        dbg.call("cpu.resume")
        dbg.wait_for_break(UPDATE_CRAFT, timeout=timeout)
        if wreck.regs(dbg)["a0"] == craft:
            return


def body_state(dbg, body):
    return {
        "pos": [round(v, 2) for v in struct.unpack("<3f", dbg.read(body + 0x30, 12))],
        "vel": [round(v, 2) for v in struct.unpack("<3f", dbg.read(body + 0x140, 12))],
    }


def teleport(dbg, body, target):
    """Body_SetPosition(x, y, z, body), called from the current Ship_UpdateCraft stop."""
    watch.call_at_stop(
        dbg,
        {"a0": body, "f12": f32bits(target[0]), "f13": f32bits(target[1]), "f14": f32bits(target[2])},
        BODY_SET_POSITION,
    )


def clear_breakpoints(dbg):
    dbg.brk()
    for address in (UPDATE_CRAFT, PAIR_RESOLVER, SINGLE_RESOLVER):
        try:
            dbg.remove_breakpoint(address)
        except Exception:  # noqa: BLE001
            pass


def count_pair_hits(dbg, pair_bodies, max_stops, timeout, resolver=PAIR_RESOLVER, entity=None, collider=None, stop_after_ours=None):
    """Arm the pair resolver and log every stop for ~`max_stops` (all pairs, flagged when ours)."""
    clear_breakpoints(dbg)
    dbg.add_breakpoint(resolver)
    hits = []
    seen = 0
    try:
        for _ in range(max_stops):
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            try:
                dbg.wait_for_break(resolver, timeout=timeout)
            except TimeoutError:
                break
            r = wreck.regs(dbg)
            seen += 1
            if resolver == PAIR_RESOLVER:
                ours = {r["a1"], r["a2"]} == set(pair_bodies)
                tag = {}
                if entity is not None:
                    tag = {
                        "state": dbg.read_u32(entity + 0x8C),
                        "timer": round(dbg.read_f32(entity + 0x874), 3),
                        "collider_0x68": dbg.read_u8(collider + 0x68),
                    }
                hits.append(
                    {
                        **tag,
                        "ours": ours,
                        "a1": hex(r["a1"]),
                        "a2": hex(r["a2"]),
                        "vel_a1": body_state(dbg, r["a1"])["vel"],
                        "vel_a2": body_state(dbg, r["a2"])["vel"],
                    }
                )
            else:
                ids = struct.unpack("<2I", dbg.read(r["a0"] + 0x20, 4) + dbg.read(r["a0"] + 0x28, 4))
                hits.append(
                    {
                        "ours": r["a1"] in pair_bodies,
                        "body": hex(r["a1"]),
                        "contact_ids_0x20_0x28": ids,
                        "vel": body_state(dbg, r["a1"])["vel"],
                    }
                )
            if stop_after_ours is not None and sum(h["ours"] for h in hits) >= stop_after_ours:
                break
    finally:
        clear_breakpoints(dbg)
        dbg.add_breakpoint(UPDATE_CRAFT)
    return hits


def teleport_after_pair(dbg, craft, body, target_pos, args, log, leg):
    """Second look at the same leg: the single-body resolver, for a pair demoted to a wall contact."""
    stop_at(dbg, craft, args.timeout)
    teleport(dbg, body, target_pos())
    hits = count_pair_hits(
        dbg, (log["player_body_int"], body), args.hits, args.stop_timeout, resolver=SINGLE_RESOLVER
    )
    log[leg + "_single_hits"] = hits
    print("%s: %d single-body resolver stops" % (leg, len(hits)), file=sys.stderr)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--slot", type=int, required=True)
    ap.add_argument("--offset", default="1,0,1", help="teleport target relative to the player body")
    ap.add_argument("--hits", type=int, default=30)
    ap.add_argument("--stop-timeout", type=float, default=6.0, help="wall s without a resolver stop = none")
    ap.add_argument("--timeout", type=float, default=60.0)
    ap.add_argument("--rest-frames", type=int, default=300, help="frames to wait after state 6 before the wreck leg")
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    offset = [float(v) for v in args.offset.split(",")]

    dbg = Debugger(args.port)
    log = {"offset": offset}
    try:
        dbg.brk()
        manager = dbg.read_u32(wreck.RACE_MANAGER)
        player = dbg.read_u32(manager + 0x2C0)
        player_craft = dbg.read_u32(player + 0x94)
        player_body = dbg.read_u32(player_craft + 0x1CC)
        entity = dbg.read_u32(manager + 0x78 + 4 * args.slot)
        craft = dbg.read_u32(entity + 0x94)
        body = dbg.read_u32(craft + 0x1CC)
        log["player_body_int"] = player_body
        log["player_body"], log["target_body"] = hex(player_body), hex(body)
        dbg.add_breakpoint(UPDATE_CRAFT)

        def target_pos():
            p = body_state(dbg, player_body)["pos"]
            return [p[i] + offset[i] for i in range(3)]

        stop_at(dbg, craft, args.timeout)
        for _ in range(20):
            stop_at(dbg, craft, args.timeout)
        log["control_before"] = watch.row(dbg, entity, "target", 0, None)
        teleport(dbg, body, target_pos())
        log["control_after_teleport"] = body_state(dbg, body)
        log["control_player"] = body_state(dbg, player_body)
        hits = count_pair_hits(dbg, (player_body, body), args.hits, args.stop_timeout)
        log["control_hits"] = hits
        log["control_ours"] = sum(h["ours"] for h in hits)
        print("control: %d resolver stops, %d ours" % (len(hits), log["control_ours"]), file=sys.stderr)
        teleport_after_pair(dbg, craft, body, target_pos, args, log, "control")

        stop_at(dbg, craft, args.timeout)
        watch.call_at_stop(
            dbg,
            {"a0": entity, "a1": 0, "a2": 0, "a3": 0, "f12": f32bits(1.0e6)},
            watch.SHIP_DAMAGE,
        )
        states = []
        frame = 0
        while True:
            stop_at(dbg, craft, args.timeout)
            frame += 1
            r = watch.row(dbg, entity, "target", frame, None)
            if frame % 30 == 0:
                states.append((frame, r["state"], r["flags"], r["speed"]))
                print("wreck frame %d state %d speed %s" % (frame, r["state"], r["speed"]), file=sys.stderr, flush=True)
            if r["state"] == 6 and frame >= 120 + args.rest_frames:
                break
            if frame > 2000:
                break
        log["wreck_states"] = states
        log["wreck_row"] = watch.row(dbg, entity, "target", frame, None)
        teleport(dbg, body, target_pos())
        log["wreck_after_teleport"] = body_state(dbg, body)
        log["wreck_player"] = body_state(dbg, player_body)
        hits = count_pair_hits(dbg, (player_body, body), args.hits, args.stop_timeout)
        log["wreck_hits"] = hits
        log["wreck_ours"] = sum(h["ours"] for h in hits)
        print("wreck: %d resolver stops, %d ours" % (len(hits), log["wreck_ours"]), file=sys.stderr)
        log["wreck_player_after"] = body_state(dbg, player_body)
        teleport_after_pair(dbg, craft, body, target_pos, args, log, "wreck")
    finally:
        try:
            clear_breakpoints(dbg)
            dbg.resume()
        except Exception as error:  # noqa: BLE001
            print("cleanup: %s" % error, file=sys.stderr)
        (args.out / "contact.json").write_text(json.dumps(log, indent=1))
        dbg.close()
    print("wrote", args.out / "contact.json")


if __name__ == "__main__":
    main()
