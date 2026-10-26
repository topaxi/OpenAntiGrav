#!/usr/bin/env python3
"""What happens when a running Autopilot's timer runs out on Pure (PPSSPP), per frame.

The Pure half of `psp-autopilot-expiry.py` (read its docstring for the policies and the
reasons). Pure's pickup record keeps the running flag as a byte (`+0x154`), the timer at
`+0x15c` and the previous timer at `+0x158` (`docs/ghidra/functions/psp-pure-usa/autopilot.md`);
`Autopilot_Update` is `0x0884c794`, called from the per-craft weapons loop that starts at
`0x0884cbbc` (`a0` the weapons world, `f12` the frame's `dt`, the records at `world+0x160`,
a stride of `0x200`). The craft is read at `Ship_UpdateCraft` (`0x0892d214`, `a0` craft, `a1`
body, the body's basis rows at `+0` and position at `+0x30`, `scripts/psp-pure-pair.py`), which
runs once a frame per craft. Pure has no Ghidra database; the controls' blend is not read.

    source <the env.sh scripts/ppsspp-start.sh wrote, started on the Pure USA image>
    uv run --with websocket-client python3 scripts/psp-pure-autopilot-expiry.py \\
        --port $OAG_PPSSPP_PORT --state data/saves/pure-psp/time-trial-chenghou-vector-feisar-restart.ppst \\
        --policy corner --out data/scratch/<lane>/pure-chenghou-corner.json

Not a physics reference: the Autopilot flies (maintainer, 2026-10-20). Output is derived game
data: keep it under `data/`, never commit.
"""

import argparse
import importlib.util
import json
import math
import os
import shutil
import struct
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

WEAPONS_LOOP = 0x0884CBBC
#: The loop body's last call (`jal 0x48bb4`, `a1` the record, `a2` the craft index), reached for every craft.
WEAPONS_LOOP_TAIL = 0x0884D0D4
SHIP_UPDATE_CRAFT = 0x0892D214
CUE_CALL = 0x0884C7EC
RECORDS = 0x160
RECORD_STRIDE = 0x200
RUNNING = 0x154
PREV = 0x158
TIMER = 0x15C
SERIAL = "UCUS98612_2.00"


def _load(name, rel):
    spec = importlib.util.spec_from_file_location(name, HERE / rel)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def regs(dbg):
    category = dbg.call("cpu.getAllRegs")["categories"][0]
    return dict(zip(category["registerNames"], category["uintValues"]))


def stop_at(dbg, address, timeout=30.0):
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    try:
        dbg.wait_for_break(address, timeout=timeout)
    except TimeoutError:
        status = dbg.call("cpu.status")
        if not (status["stepping"] and status["pc"] == address):
            raise
    return regs(dbg)


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--state", type=Path, required=True)
    parser.add_argument("--policy", choices=("natural", "corner", "cue", "buttons"), default="corner")
    parser.add_argument("--stint", type=float, default=4.0)
    parser.add_argument("--force", type=float, default=0.05)
    parser.add_argument("--corner", type=float, default=2.0,
                        help="degrees of yaw a frame that count as a corner")
    parser.add_argument("--expiries", type=int, default=20)
    parser.add_argument("--lead", type=int, default=420)
    parser.add_argument("--budget", type=float, default=540.0)
    parser.add_argument("--max-frames", type=int, default=0)
    args = parser.parse_args()

    st = _load("st", "psp-state.py")
    ex = _load("ex", "psp-autopilot-expiry.py")
    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "frames": [], "events": []}
    start = time.time()
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(1)
    target = st.state_dir(os.environ["PP_HOME"]) / ("%s_1.ppst" % SERIAL)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy(args.state, target)
    st.set_menu(dbg, True)
    dbg.brk()
    dbg.add_breakpoint(WEAPONS_LOOP_TAIL)
    st.click(st.LOAD_X, st.SLOT_Y0 + st.SLOT_DY * 1)
    time.sleep(2.5)
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(WEAPONS_LOOP_TAIL, timeout=60)
    g = regs(dbg)
    for _ in range(8):
        if g["a2"] == 0:
            break
        g = stop_at(dbg, WEAPONS_LOOP_TAIL)
    record = g["a1"]
    world = g["s4"]
    craft_count = dbg.read_u32(0x08857518)
    log["events"].append({"world": world, "record": record, "record_index": g["a2"], "craft_count_at_0x08857518": craft_count,
                          "running": dbg.read_u8(record + RUNNING),
                          "timer": dbg.read_f32(record + TIMER)})
    dbg.remove_breakpoint(WEAPONS_LOOP_TAIL)
    dbg.add_breakpoint(SHIP_UPDATE_CRAFT)
    player = {"craft": None}

    def step():
        while True:
            g = stop_at(dbg, SHIP_UPDATE_CRAFT)
            if player["craft"] is None:
                player["craft"] = g["a0"]
            if g["a0"] == player["craft"]:
                return g

    frame = 0

    def run(n, phase):
        nonlocal frame
        for _ in range(n):
            g = step()
            frame += 1
            body = g["a1"]
            rows = struct.unpack("<12f", dbg.read(body, 48))
            log["frames"].append({
                "frame": frame, "phase": phase,
                "running": dbg.read_u8(record + RUNNING),
                "timer": dbg.read_f32(record + TIMER),
                "prev": dbg.read_f32(record + PREV),
                "right": rows[0:3], "up": rows[4:7], "fwd": rows[8:11],
                "pos": list(struct.unpack("<3f", dbg.read(body + 0x30, 12))),
            })
            if frame % 200 == 0:
                print("frame %d phase %s expiries %d" % (frame, phase, expiries), file=sys.stderr,
                      flush=True)

    def arm(timer):
        dbg.write(record + RUNNING, b"\x01")
        dbg.write(record + TIMER, struct.pack("<f", timer))

    expiries = 0
    try:
        dbg.hold(cross=True)
        run(args.lead, "lead")
        moved = math.dist(log["frames"][0]["pos"], log["frames"][-1]["pos"])
        log["events"].append({"moved_during_lead": moved})
        if moved < 20.0:
            raise SystemExit("the craft did not leave the grid in %d frames (moved %.1f)"
                             % (args.lead, moved))
        arm(args.stint)
        run(3, "armed")
        if log["frames"][-1]["timer"] >= args.stint - 1e-3:
            raise SystemExit("Autopilot_Update never ran: the timer did not fall")
        if args.policy == "buttons":
            # Whether a face button cancels a running Autopilot here (the static read says none does).
            for button in ("circle", "triangle", "square"):
                arm(args.stint)
                run(10, "%s-armed" % button)
                dbg.hold(**{button: True})
                run(10, "%s-held" % button)
                dbg.hold(**{button: False})
                run(4, "%s-after" % button)
                log["events"].append({"button": button, "running_after": log["frames"][-1]["running"],
                                      "timer_after": log["frames"][-1]["timer"]})
            expiries = 1
        elif args.policy == "cue":
            arm(args.stint)
            run(1, "cue-armed")
            dbg.remove_breakpoint(SHIP_UPDATE_CRAFT)
            dbg.add_breakpoint(CUE_CALL)
            dbg.pending = []
            dbg.call("cpu.resume")
            hit = dbg.wait_for_break(CUE_CALL, timeout=120)
            log["events"].append({"cue_hit_at_frame": frame,
                                  "timer": dbg.read_f32(record + TIMER),
                                  "prev": dbg.read_f32(record + PREV),
                                  "running": dbg.read_u8(record + RUNNING), "pc": hit.get("pc")})
            dbg.remove_breakpoint(CUE_CALL)
            dbg.add_breakpoint(SHIP_UPDATE_CRAFT)
            for _ in range(600):
                run(1, "cue-after")
                if not log["frames"][-1]["running"]:
                    break
            run(6, "cue-released")
            expiries = 1
        else:
            forced = False
            while expiries < args.expiries and time.time() - start < args.budget:
                if args.max_frames and frame >= args.max_frames:
                    break
                run(1, "flying")
                row = log["frames"][-1]
                if row["running"]:
                    if args.policy == "corner" and not forced and row["timer"] > args.force + 0.3:
                        rate = ex.yaw_rate(log["frames"][-2]["fwd"], row["fwd"])
                        if abs(rate) > args.corner:
                            dbg.write(record + TIMER, struct.pack("<f", args.force))
                            forced = True
                            log["events"].append({"forced_at_frame": frame, "yaw_rate": rate,
                                                  "bank": ex.bank(row), "timer_before": row["timer"]})
                else:
                    run(2, "released")
                    expiries += 1
                    forced = False
                    arm(args.stint)
                    run(3, "rearmed")
        log["events"].append({"expiries": expiries})
    finally:
        dbg.hold(cross=False, circle=False, square=False)
        dbg.brk()
        for address in (WEAPONS_LOOP_TAIL, SHIP_UPDATE_CRAFT, CUE_CALL):
            try:
                dbg.remove_breakpoint(address)
            except Exception:
                pass
        dbg.resume()
        frames = log["frames"]
        for a, b in zip(frames, frames[1:]):
            b["yaw_rate"] = ex.yaw_rate(a["fwd"], b["fwd"])
            b["dt_timer"] = a["timer"] - b["timer"]
        for row in frames:
            row["bank"] = ex.bank(row)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(log))
        print("wrote %s (%d frames)" % (args.out, len(frames)), file=sys.stderr)


if __name__ == "__main__":
    main()
