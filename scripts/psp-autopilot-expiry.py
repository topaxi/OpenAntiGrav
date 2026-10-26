#!/usr/bin/env python3
"""What happens when a running Autopilot's timer runs out, per frame, on Pulse (PPSSPP).

Loads a Time Trial or Single Race save state, holds the throttle, waits for the race to be
under way, then arms the player's Autopilot the way `Autopilot_Fire` does (running bit
`0x800` of `craft_record+0x1b8`, timer `+0x148`) and stops once a frame on
`Weapons_DispatchFire` (`0x08861814`, which calls `Autopilot_Update` with the frame's `dt`)
to log, per frame: the running bit, the timer, the controls' blend `craft+0x1d4`, whether
`*(craft+0x78)` is the pad record `*(craft+0x3c)`, the engine bits, the body's right, up and
forward rows and its position. Yaw rate, bank and a centripetal-acceleration proxy come off
those afterwards. The timer's per-frame fall is the frame's `dt`.

Policies for when an expiry happens:

- `natural`: a stint of `--stint` seconds, re-armed three frames after it releases. Expiries
  fall where they fall, one every `--stint` seconds of flying.
- `corner`: the Autopilot is armed for `--stint` seconds and, the first frame its yaw rate
  passes `--corner` degrees a frame, the timer is written down to `--force` seconds (a few
  frames). Every expiry then falls inside a corner, and the timer ends where a hold would
  have to act. The bank at the trigger is logged too.
- `cue`: one stint, with an execution breakpoint on the `jal` to `Sound_PlayDry` inside
  `Autopilot_Update` (`0x0886145c`, the `"disengaging"` cue). The hit is logged with the
  timer, then the per-frame sampling resumes and runs to the release, so the cue can be read
  against the hand-back in timer seconds. Only the most recently added execution breakpoint
  fires on PPSSPP v1.20.4, so the per-frame breakpoint is removed first.

Not a physics reference: the Autopilot flies (maintainer, 2026-10-20). Only its own timer and
hand-back are read here. Raw output is derived game data: keep it under `data/`, never commit.

    source <the env.sh scripts/ppsspp-start.sh wrote>
    uv run --with websocket-client python3 scripts/psp-autopilot-expiry.py \\
        --port $OAG_PPSSPP_PORT --state data/saves/pulse-psp/time-trial-fort-gale-black-venom-grid.ppst \\
        --policy corner --out data/scratch/<lane>/fort-gale-corner.json

`docs/ghidra/functions/psp-pulse-usa/autopilot.md`, "Expiry mid-corner", has what it found.
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

WEAPONS_DISPATCH_FIRE = 0x08861814
CUE_CALL = 0x0886145C
RACE_MANAGER = 0x08B317B4
PLAYER_OFFSET = 0x2C0
RACE_SETTINGS = 0x08B30F90
RUNNING = 0x800


def _load(name, rel):
    spec = importlib.util.spec_from_file_location(name, HERE / rel)
    mod = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(mod)
    return mod


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


class Player:
    def __init__(self, dbg):
        manager = dbg.read_u32(RACE_MANAGER)
        self.entity = dbg.read_u32(manager + PLAYER_OFFSET)
        self.record = dbg.read_u32(self.entity + 0x4C)
        self.craft = dbg.read_u32(self.entity + 0x94)
        self.body = dbg.read_u32(self.entity + 0x794)
        assert all(ram(p) for p in (self.entity, self.record, self.craft, self.body))

    def sample(self, dbg, frame, phase):
        controls = dbg.read_u32(self.craft + 0x78)
        pose = struct.unpack("<12f", dbg.read(self.body + 0x00, 48))
        return {
            "frame": frame,
            "phase": phase,
            "flags": dbg.read_u32(self.record + 0x1B8),
            "held": struct.unpack("<i", dbg.read(self.record + 0x1BC, 4))[0],
            "timer": dbg.read_f32(self.record + 0x148),
            "prev": dbg.read_f32(self.record + 0x144),
            "blend": dbg.read_f32(self.craft + 0x1D4),
            "engine": dbg.read_u32(self.craft + 0x1C0),
            "controls_is_pad": controls == dbg.read_u32(self.craft + 0x3C),
            "controls_is_driver": controls == self.craft + 0x44,
            "steer": dbg.read_f32(controls + 0x00) if ram(controls) else None,
            "finished": dbg.read_u8(self.entity + 0x912),
            "right": pose[0:3],
            "up": pose[4:7],
            "fwd": pose[8:11],
            "pos": list(struct.unpack("<3f", dbg.read(self.body + 0x30, 12))),
        }


def step(dbg):
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    try:
        dbg.wait_for_break(WEAPONS_DISPATCH_FIRE, timeout=30.0)
    except TimeoutError:
        status = dbg.call("cpu.status")
        if not (status["stepping"] and status["pc"] == WEAPONS_DISPATCH_FIRE):
            raise


def arm(dbg, p, timer):
    dbg.write_u32(p.record + 0x1B8, dbg.read_u32(p.record + 0x1B8) | RUNNING)
    dbg.write(p.record + 0x148, struct.pack("<f", timer))


def set_timer(dbg, p, timer):
    dbg.write(p.record + 0x148, struct.pack("<f", timer))


def yaw_rate(a, b):
    """Degrees per frame between two forward rows, in the horizontal plane."""
    d = math.degrees(math.atan2(b[0], b[2]) - math.atan2(a[0], a[2]))
    return (d + 180.0) % 360.0 - 180.0


def bank(row):
    """Degrees the right row leans out of the horizontal (the roll of the craft)."""
    return math.degrees(math.asin(max(-1.0, min(1.0, row["right"][1]))))


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--state", type=Path, required=True)
    parser.add_argument("--serial", default="UCUS98712_1.00")
    parser.add_argument("--policy", choices=("natural", "corner", "cue", "cancel"), default="corner")
    parser.add_argument("--stint", type=float, default=3.0)
    parser.add_argument("--force", type=float, default=0.05)
    parser.add_argument("--corner", type=float, default=2.5,
                        help="degrees of yaw a frame that count as a corner")
    parser.add_argument("--expiries", type=int, default=30)
    parser.add_argument("--lead", type=int, default=420,
                        help="frames to hold the throttle before the Autopilot is armed")
    parser.add_argument("--budget", type=float, default=540.0)
    parser.add_argument("--max-frames", type=int, default=0,
                        help="stop after this many frames in all (0: until --expiries or --budget)")
    parser.add_argument("--stick", type=float, default=0.0,
                        help="hold the left stick sideways, -1.0 to 1.0, so the pad record has a "
                             "steering value to be handed back to")
    parser.add_argument("--weapons-on", action="store_true",
                        help="set the race's weapons byte (the pickup path needs it in a Time Trial)")
    args = parser.parse_args()

    st = _load("st", "psp-state.py")
    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "frames": [], "events": []}
    start = time.time()
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(1)
    target = st.state_dir(os.environ["PP_HOME"]) / ("%s_1.ppst" % args.serial)
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy(args.state, target)
    st.set_menu(dbg, True)
    dbg.brk()
    dbg.add_breakpoint(WEAPONS_DISPATCH_FIRE)
    st.click(st.LOAD_X, st.SLOT_Y0 + st.SLOT_DY * 1)
    time.sleep(2.5)
    dbg.pending = []
    dbg.call("cpu.resume")
    dbg.wait_for_break(WEAPONS_DISPATCH_FIRE, timeout=60)
    p = Player(dbg)
    log["events"].append({"weapons_byte": dbg.read_u8(RACE_SETTINGS + 0x18),
                          "game_mode": dbg.read_u32(RACE_SETTINGS + 0xB8)})
    if args.weapons_on:
        dbg.write(RACE_SETTINGS + 0x18, b"\x01")
    frame = 0

    def run(n, phase):
        nonlocal frame
        for _ in range(n):
            step(dbg)
            frame += 1
            log["frames"].append(p.sample(dbg, frame, phase))
            if frame % 200 == 0:
                print("frame %d phase %s expiries %d" % (frame, phase, expiries), file=sys.stderr,
                      flush=True)

    expiries = 0
    try:
        dbg.hold(cross=True)
        if args.stick:
            dbg.analog(args.stick, 0.0)
        run(args.lead, "lead")
        moved = math.dist(log["frames"][0]["pos"], log["frames"][-1]["pos"])
        log["events"].append({"moved_during_lead": moved})
        if moved < 20.0:
            raise SystemExit("the craft did not leave the grid in %d frames (moved %.1f)"
                             % (args.lead, moved))
        arm(dbg, p, args.stint)
        run(3, "armed")
        if log["frames"][-1]["timer"] >= args.stint - 1e-3:
            raise SystemExit("Autopilot_Update never ran: the timer did not fall "
                             "(try --weapons-on)")
        if args.policy == "cancel":
            # An absorb press with seconds left: does `Autopilot_Update`'s next call take the
            # one-second edge (timer = -dt, previous above 1.0) and say "disengaging"?
            arm(dbg, p, args.stint)
            run(10, "cancel-armed")
            before = log["frames"][-1]
            dbg.remove_breakpoint(WEAPONS_DISPATCH_FIRE)
            dbg.add_breakpoint(CUE_CALL)
            dbg.hold(circle=True)
            dbg.pending = []
            dbg.call("cpu.resume")
            try:
                dbg.wait_for_break(CUE_CALL, timeout=8)
                log["events"].append({"cancel_cue_hit": True,
                                      "timer": dbg.read_f32(p.record + 0x148),
                                      "prev": dbg.read_f32(p.record + 0x144),
                                      "timer_before_press": before["timer"]})
            except TimeoutError:
                dbg.brk()
                log["events"].append({"cancel_cue_hit": False,
                                      "timer": dbg.read_f32(p.record + 0x148),
                                      "flags": dbg.read_u32(p.record + 0x1B8),
                                      "timer_before_press": before["timer"]})
            dbg.remove_breakpoint(CUE_CALL)
            dbg.hold(circle=False)
            dbg.add_breakpoint(WEAPONS_DISPATCH_FIRE)
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break(WEAPONS_DISPATCH_FIRE, timeout=30)
            run(6, "cancel-after")
            expiries = 1
        elif args.policy == "cue":
            arm(dbg, p, args.stint)
            run(1, "cue-armed")
            dbg.remove_breakpoint(WEAPONS_DISPATCH_FIRE)
            dbg.add_breakpoint(CUE_CALL)
            dbg.pending = []
            dbg.call("cpu.resume")
            hit = dbg.wait_for_break(CUE_CALL, timeout=120)
            log["events"].append({"cue_hit_at_frame": frame,
                                  "timer": dbg.read_f32(p.record + 0x148),
                                  "prev": dbg.read_f32(p.record + 0x144),
                                  "flags": dbg.read_u32(p.record + 0x1B8),
                                  "pc": hit.get("pc")})
            dbg.remove_breakpoint(CUE_CALL)
            dbg.add_breakpoint(WEAPONS_DISPATCH_FIRE)
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break(WEAPONS_DISPATCH_FIRE, timeout=30)
            for _ in range(600):
                run(1, "cue-after")
                if not log["frames"][-1]["flags"] & RUNNING:
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
                if row["flags"] & RUNNING:
                    if args.policy == "corner" and not forced and row["timer"] > args.force + 0.3:
                        rate = yaw_rate(log["frames"][-2]["fwd"], row["fwd"])
                        if abs(rate) > args.corner:
                            set_timer(dbg, p, args.force)
                            forced = True
                            log["events"].append({"forced_at_frame": frame, "yaw_rate": rate,
                                                  "bank": bank(row), "timer_before": row["timer"]})
                else:
                    run(2, "released")
                    expiries += 1
                    forced = False
                    arm(dbg, p, args.stint)
                    run(3, "rearmed")
        log["events"].append({"expiries": expiries})
    finally:
        dbg.hold(cross=False, circle=False, square=False)
        dbg.analog(0.0, 0.0)
        dbg.brk()
        for address in (WEAPONS_DISPATCH_FIRE, CUE_CALL):
            try:
                dbg.remove_breakpoint(address)
            except Exception:
                pass
        dbg.resume()
        frames = log["frames"]
        for a, b in zip(frames, frames[1:]):
            b["yaw_rate"] = yaw_rate(a["fwd"], b["fwd"])
            b["dt_timer"] = a["timer"] - b["timer"]
        for row in frames:
            row["bank"] = bank(row)
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(log))
        print("wrote %s (%d frames)" % (args.out, len(frames)), file=sys.stderr)


if __name__ == "__main__":
    main()
