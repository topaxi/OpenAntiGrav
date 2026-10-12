#!/usr/bin/env python3
"""Per-state craft-against-craft contact for a destroyed opponent, and the collider enable byte.

Companion of `psp-wreck-contact.py` (read its docstring for the method). Wrecks one opponent with
`Ship_Damage(1e6, ...)`, then steps its frames logging `state`, `entity+0x860`, the live-model flag and
the box collider's `+0x68` byte (`body+0x3bc`), and at each named state runs one teleport-onto-the-player
leg counting `Body_ResolveContactPair` stops for exactly that pair.

    uv run --with websocket-client scripts/psp-wreck-phases.py --port 45095 --out DIR --slot 3 \\
        --legs 5,8 --frames 260

`--legs` is the list of states to run a leg in (once each). Run in a Single Race (states 4, 5, 6) or an
Eliminator (4, 5, 8, 1). Derived data; keep the output under `data/`.
"""

import argparse
import importlib.util
import json
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location("contact", Path(__file__).resolve().parent / "psp-wreck-contact.py")
contact = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(contact)
watch = contact.watch
wreck = contact.wreck


def leg(dbg, entity, collider, body, player_body, offset, args):
    """Teleport the target onto the player at rest, then count pair stops, tagging each with the target's state."""
    pp = contact.body_state(dbg, player_body)["pos"]
    contact.teleport(dbg, body, [pp[i] + offset[i] for i in range(3)])
    dbg.write_f32s(body + 0x140, [0.0, 0.0, 0.0])
    hits = contact.count_pair_hits(dbg, (player_body, body), args.hits, args.stop_timeout, entity=entity, collider=collider, stop_after_ours=4)
    return {"stops": len(hits), "ours": sum(h["ours"] for h in hits), "hits": hits}


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--slot", type=int, required=True)
    ap.add_argument("--legs", default="5", help="comma list of states to run a contact leg in")
    ap.add_argument("--frames", type=int, default=260, help="target-stop frames to log after the call")
    ap.add_argument("--offset", default="1,0,1")
    ap.add_argument("--hits", type=int, default=40)
    ap.add_argument("--stop-timeout", type=float, default=2.0)
    ap.add_argument("--timeout", type=float, default=60.0)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)
    offset = [float(v) for v in args.offset.split(",")]
    leg_states = {int(v) for v in args.legs.split(",")}

    dbg = Debugger(args.port)
    log = {"rows": [], "legs": {}}
    try:
        dbg.brk()
        manager = dbg.read_u32(wreck.RACE_MANAGER)
        player = dbg.read_u32(manager + 0x2C0)
        player_body = dbg.read_u32(dbg.read_u32(player + 0x94) + 0x1CC)
        entity = dbg.read_u32(manager + 0x78 + 4 * args.slot)
        craft = dbg.read_u32(entity + 0x94)
        body = dbg.read_u32(craft + 0x1CC)
        collider = dbg.read_u32(body + 0x3BC)
        dbg.add_breakpoint(contact.UPDATE_CRAFT)
        for _ in range(20):
            contact.stop_at(dbg, craft, args.timeout)
        log["header"] = {"mode_class_0x368": dbg.read_u32(entity + 0x368), "game_mode": dbg.read_u32(0x08B30F90 + 0xB8)}
        log["legs"]["control"] = leg(dbg, entity, collider, body, player_body, offset, args)
        print("control leg: %d stops, %d ours" % (log["legs"]["control"]["stops"], log["legs"]["control"]["ours"]), file=sys.stderr, flush=True)
        contact.stop_at(dbg, craft, args.timeout)
        contact.watch.call_at_stop(
            dbg,
            {"a0": entity, "a1": 0, "a2": 0, "a3": 0, "f12": contact.f32bits(1.0e6)},
            watch.SHIP_DAMAGE,
        )
        done = set()
        for frame in range(1, args.frames + 1):
            contact.stop_at(dbg, craft, args.timeout)
            r = watch.row(dbg, entity, "target", frame, None)
            r["collider_0x68"] = dbg.read_u8(collider + 0x68)
            log["rows"].append(r)
            if frame % 30 == 0:
                print("frame %d state %d flags %s wreck_live %s +0x68 %d" % (frame, r["state"], r["flags"], r["live_is_wreck"], r["collider_0x68"]), file=sys.stderr, flush=True)
            state = r["state"]
            if state in leg_states and state not in done and frame >= 5:
                done.add(state)
                result = leg(dbg, entity, collider, body, player_body, offset, args)
                result["frame"] = frame
                log["legs"][str(state)] = result
                print("leg state %d: %d stops, %d ours" % (state, result["stops"], result["ours"]), file=sys.stderr, flush=True)
    finally:
        try:
            contact.clear_breakpoints(dbg)
            dbg.resume()
        except Exception as error:  # noqa: BLE001
            print("cleanup: %s" % error, file=sys.stderr)
        (args.out / "phases.json").write_text(json.dumps(log, indent=1))
        dbg.close()
    print("wrote", args.out / "phases.json")


if __name__ == "__main__":
    main()
