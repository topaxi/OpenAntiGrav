#!/usr/bin/env python3
"""What ends a running Autopilot on the original, per frame: the buttons, and the corners.

Loads `data/saves/pulse-psp/race-talons-venom-alongside.ppst` (Single Race, Talon's Junction
White, VENOM, the player's Autopilot already running with its timer at `1e9`) into the
instance's slot 2 through the pause menu, then stops once a frame on `Weapons_DispatchFire`
(`0x08861814`) and logs, for the player:

- the pickup record (`entity+0x4c`): fire flags `+0x1b8` (running bit `0x800`), held id
  `+0x1bc`, timer `+0x148`;
- the craft (`entity+0x94`): the controls' blend `+0x1d4`, engine bits `+0x1c0`, and the two
  button bytes of its control record (`*(craft+0x78)`: `+0x15` fire, `+0x17` absorb);
- `entity+0x860` (the release guards `0x8`/`0x10`) and the byte `entity+0x6c`;
- the body's forward row and position (`body+0x20`, `body+0x30`), for the yaw rate.

Phases, all on one load:

1. `absorb`: hold CIRCLE for a few frames with nothing held.
2. `fire`: re-arm (bit `0x800`, timer `1e9`), hold SQUARE with nothing held.
3. `fire-held-autopilot`: held id `6` written, hold SQUARE: `Ship_FireHeldWeapon` should refuse.
4. `corners`: the timer is set to `--stint` seconds and allowed to run out, then re-armed on the
   first frame the running bit reads clear, `--expiries` times; each expiry logs the frames
   around it, so the release can be read against the yaw rate where it happened.

Not a physics reference: the Autopilot flies (maintainer, 2026-10-20). Raw output is derived
game data: keep it under `data/`, never commit it.

    source data/scratch/<lane>/pp/env.sh
    uv run --with websocket-client python3 scripts/psp-autopilot-cancel.py \\
        --port $OAG_PPSSPP_PORT --out data/scratch/<lane>/cancel.json
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
REPO = HERE.parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

WEAPONS_DISPATCH_FIRE = 0x08861814
RACE_MANAGER = 0x08B317B4
PLAYER_OFFSET = 0x2C0
RUNNING = 0x800
# The race settings (`FUN_088418e0` keeps them in `s4`, `0x08b30f90`): `+0x18` is the byte that
# lets `FUN_0883f540`, the per-craft weapon update that calls `Ship_FireHeldWeapon` and
# `Ship_AbsorbHeldPickup`, run at all (`0x08842270`); `+0xb8` is the game mode.
RACE_SETTINGS = 0x08B30F90


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
        buttons = dbg.read(controls + 0x14, 4) if ram(controls) else b"\0\0\0\0"
        pose = struct.unpack("<12f", dbg.read(self.body + 0x00, 48))
        return {
            "frame": frame,
            "phase": phase,
            "flags": dbg.read_u32(self.record + 0x1B8),
            "held": struct.unpack("<i", dbg.read(self.record + 0x1BC, 4))[0],
            "timer": dbg.read_f32(self.record + 0x148),
            "blend": dbg.read_f32(self.craft + 0x1D4),
            "engine": dbg.read_u32(self.craft + 0x1C0),
            # `Ship_UpdateCraft` points `craft+0x78` at the pad record `*(craft+0x3c)`, or at its
            # own buffer `craft+0x44` holding the driver's record (`0x088496a4`, `0x088496b8`).
            "controls_is_pad": controls == dbg.read_u32(self.craft + 0x3C),
            "fire_byte": buttons[1],
            "absorb_byte": buttons[3],
            "entity860": dbg.read_u32(self.entity + 0x860),
            "entity6c": dbg.read_u8(self.entity + 0x6C),
            # `PlayerStatus_Update`'s first arm: a finished racer keeps blend 1.0 and uncapped.
            "finished": dbg.read_u8(self.entity + 0x912),
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


def yaw_rate(a, b):
    """Degrees per frame between two forward rows, in the horizontal plane."""
    ha = math.atan2(a[0], a[2])
    hb = math.atan2(b[0], b[2])
    d = math.degrees(hb - ha)
    return (d + 180.0) % 360.0 - 180.0


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--state", type=Path,
                        default=REPO / "data/saves/pulse-psp/race-talons-venom-alongside.ppst")
    parser.add_argument("--stint", type=float, default=0.25)
    parser.add_argument("--expiries", type=int, default=60)
    parser.add_argument("--budget", type=float, default=540.0)
    parser.add_argument("--weapons-on", action="store_true",
                        help="set the race's weapons byte; the saved race is Weapons OFF")
    args = parser.parse_args()

    st = _load("st", "psp-state.py")
    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "frames": [], "events": []}
    start = time.time()
    if dbg.is_stepping():
        dbg.call("cpu.resume")
        time.sleep(1)
    target = st.state_dir(os.environ["PP_HOME"]) / "UCUS98712_1.00_1.ppst"
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
    if dbg.read_f32(p.record + 0x148) < 1.0e6 or dbg.read_u8(p.entity + 0x912):
        raise SystemExit("the state did not load: no Autopilot at 1e9, or the race is finished")
    log["events"].append({"weapons_byte": dbg.read_u8(RACE_SETTINGS + 0x18),
                          "game_mode": dbg.read_u32(RACE_SETTINGS + 0xB8)})
    if args.weapons_on:
        dbg.write(RACE_SETTINGS + 0x18, b"\x01")
    frame = 0

    def run(n, phase, hold=None):
        nonlocal frame
        for i in range(n):
            if hold is not None:
                dbg.hold(**{hold: i < 4})
            step(dbg)
            frame += 1
            log["frames"].append(p.sample(dbg, frame, phase))
            if frame % 200 == 0:
                # emu-run.py stops a run that prints nothing for 150 s.
                print("frame %d phase %s" % (frame, phase), file=sys.stderr, flush=True)

    try:
        dbg.hold(cross=True)
        run(20, "baseline")
        run(30, "absorb", hold="circle")
        arm(dbg, p, 1.0e9)
        run(20, "rearm")
        run(30, "fire", hold="square")
        dbg.write_u32(p.record + 0x1BC, 6)
        run(30, "fire-held-autopilot", hold="square")
        dbg.write_u32(p.record + 0x1BC, 0xFFFFFFFF)
        arm(dbg, p, 1.0e9)
        run(10, "rearm2")
        expiries = 0
        arm(dbg, p, args.stint)
        while expiries < args.expiries and time.time() - start < args.budget:
            run(1, "corners")
            row = log["frames"][-1]
            if not row["flags"] & RUNNING:
                # Two more frames unarmed: the release is read on the frame after expiry.
                run(2, "corners-released")
                expiries += 1
                arm(dbg, p, args.stint)
        log["events"].append({"expiries": expiries})
    finally:
        dbg.hold(cross=False, circle=False, square=False)
        dbg.brk()
        dbg.remove_breakpoint(WEAPONS_DISPATCH_FIRE)
        dbg.resume()
        frames = log["frames"]
        for a, b in zip(frames, frames[1:]):
            b["yaw_rate"] = yaw_rate(a["fwd"], b["fwd"])
        args.out.parent.mkdir(parents=True, exist_ok=True)
        args.out.write_text(json.dumps(log))
        print("wrote %s (%d frames)" % (args.out, len(frames)), file=sys.stderr)


if __name__ == "__main__":
    main()
