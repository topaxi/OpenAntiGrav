#!/usr/bin/env python3
"""What the original's race does after the player crosses the finish line.

The maintainer's play report (2026-10-01): once a race is over the race keeps running behind
the end-race panels, the player's craft driven by AI. This measures it on a running Pulse PSP.

How a whole race is finished without steering: the player's weapon record is given the
Autopilot pickup (fire bit `0x1000`, the same word `scripts/psp-weapon-pair.py` writes) and its
countdown (`record+0x148`) is raised to a huge value, so the original's own autopilot drives
the lap(s). That is the "autopilot input" driver the original also hands the player at the
flag (`Race_FinishAllCrafts`), so it is a shared driver, not a stand-in. `--laps-hack N` writes
`g_race_laps` to shorten the race; without it the stock lap count is raced.

Phases:

1. `RESTART RACE` (the pause-menu walk `psp-weapon-pair.py` uses), thrust held through the
   countdown, the autopilot armed `--arm-after` frames after GO. The emulator then free-runs.
2. A cheap poll (a memory read on a free-running emulator costs ~0.5 s) watches the player's
   crossing count, `craft+0xac8`.
3. On the final lap the script breaks in `Weapons_DispatchFire` once a frame and logs, for
   every craft, the state word, the flag word, the control record, the driver pointer, the
   body's position and velocity, plus the manager's state, the HUD's hidden flag and the camera
   object's mode and subject. `craft+0x912` going non-zero is the finish; frames are
   photographed at `--shots` frames after it, until `--after` seconds have passed.

    python3 scripts/psp-postrace.py --port 45681 --display :91 --out data/scratch/<lane>/sr-a

Raw captures are derived game data: write them under `data/`, never commit them.
"""

import argparse
import base64
import importlib.util
import json
import struct
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

_spec = importlib.util.spec_from_file_location(
    "pair", Path(__file__).resolve().parent / "psp-weapon-pair.py"
)
pair = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(pair)

WEAPONS_DISPATCH_FIRE = 0x08861814
RACE_MANAGER = 0x08B317B4
G_HUD = 0x08AB0838
CAMERA_OBJECT = 0x08B32C64
G_RACE_LAPS = 0x08B30F9C
G_RACER_COUNT = 0x08B30F90
G_GAME_MODE = None  # filled from names.tsv if ever needed
CYCLES_PER_FRAME = 222_000_000 / 59.940059940059946

PLAYER_OFFSET = 0x2C0
SLOTS = 0x78
ENTITY_CRAFT = 0x94
ENTITY_RECORD = 0x4C
AUTOPILOT_BIT = 0x1000
FIRE_WORD = 0x1B8
HELD_WORD = 0x1BC
AUTO_TIMER = 0x148


def hits(dbg, address, count, timeout=120.0):
    """Like `Debugger.each_hit`, but a stop whose message was missed still counts.

    On v1.20.4 a resume can be answered by a stop at the breakpoint's own address that the
    wait never sees, which reads as a timeout while `cpu.status` says the CPU is stopped
    right there. When that happens the stop is yielded instead of raised.
    """
    dbg.brk()
    dbg.add_breakpoint(address)
    try:
        for index in range(count):
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            try:
                yield index, dbg.wait_for_break_any((address,), timeout=timeout)
            except TimeoutError:
                status = dbg.call("cpu.status")
                if status["stepping"] and status["pc"] == address:
                    print("missed stop recovered", file=sys.stderr)
                    yield index, status
                else:
                    raise
    finally:
        dbg.brk()
        dbg.remove_breakpoint(address)


SHIP_SET_STATE = 0x08844100


def regs(dbg):
    category = dbg.call("cpu.getAllRegs")["categories"][0]
    return dict(zip(category["registerNames"], category["uintValues"]))


def wait_any_stop(dbg, timeout):
    """Wait for any stop and return `cpu.status`, whatever caused it (a watchpoint's pc is not known)."""
    end = time.time() + timeout
    dbg.pending = []
    while time.time() < end:
        msg = dbg._recv()
        if msg is not None and msg.get("event") == "cpu.stepping":
            return dbg.call("cpu.status")
    raise TimeoutError("no stop")


def probe_set_state(dbg, player, frames_after, log):
    """Break on `Ship_SetState` and log every call (entity, state, ra) until the player's state 2."""
    dbg.brk()
    dbg.add_breakpoint(SHIP_SET_STATE)
    seen_player_two = 0
    try:
        for _ in range(400):
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            dbg.wait_for_break(SHIP_SET_STATE, timeout=120.0)
            r = regs(dbg)
            row = {"entity": r["a0"], "state": r["a1"], "ra": r["ra"], "player": r["a0"] == player,
                   "sp": r["sp"]}
            log["set_state"].append(row)
            print("Ship_SetState(entity %#x%s, %d) ra=%#x" % (
                r["a0"], " PLAYER" if row["player"] else "", r["a1"], r["ra"]), file=sys.stderr)
            if row["player"] and r["a1"] == 2:
                seen_player_two = 1
            if seen_player_two:
                frames_after -= 1
                if frames_after <= 0:
                    break
    finally:
        dbg.brk()
        dbg.remove_breakpoint(SHIP_SET_STATE)


def probe_ctl_write(dbg, ctl, count, log, size=8):
    """Stop on every write to `ctl` (the player's control record, or the autopilot scale) and log the writer."""
    dbg.brk()
    dbg.call("memory.breakpoint.add", address=ctl, size=size, enabled=True, log=False,
             read=False, write=True, change=False)
    try:
        for _ in range(count):
            dbg.call("cpu.status")
            dbg.pending = []
            dbg.call("cpu.resume")
            status = wait_any_stop(dbg, 60.0)
            r = regs(dbg)
            row = {"pc": status["pc"], "ra": r["ra"], "a0": r["a0"], "sp": r["sp"],
                   "value": list(f32s(dbg, ctl, min(5, size // 4)))}
            log["ctl_writes"].append(row)
            print("ctl write pc=%#x ra=%#x record=%s" % (status["pc"], r["ra"],
                  ["%.1f" % v for v in row["value"]]), file=sys.stderr)
    finally:
        dbg.brk()
        dbg.call("memory.breakpoint.remove", address=ctl, size=size)


def ram(pointer):
    return 0x08800000 <= pointer < 0x0A000000


def u32s(dbg, address, count):
    return struct.unpack("<%dI" % count, dbg.read(address, 4 * count))


def f32s(dbg, address, count):
    return struct.unpack("<%df" % count, dbg.read(address, 4 * count))


def craft_row(dbg, entity, full):
    """Everything one craft contributes to a frame row."""
    craft = dbg.read_u32(entity + ENTITY_CRAFT)
    if not ram(craft):
        return None
    flags, controller = u32s(dbg, entity + 0x860, 1)[0], dbg.read_u32(entity + 0x368)
    block = dbg.read(craft + 0x1C0, 0x130)
    flags_1c0 = struct.unpack_from("<I", block, 0)[0]
    state_2a4 = struct.unpack_from("<I", block, 0x2A4 - 0x1C0)[0]
    throttle, brake, steer, airl, airr = struct.unpack_from("<5f", block, 0x2B8 - 0x1C0)
    speed = struct.unpack_from("<f", block, 0x2EC - 0x1C0)[0]
    stun = struct.unpack_from("<f", block, 0x290 - 0x1C0)[0]
    body = dbg.read_u32(craft + 0x1CC)
    driver = dbg.read_u32(craft + 0x3C)
    ctl = dbg.read_u32(craft + 0x78)
    row = {
        "entity": entity,
        "craft": craft,
        "slot_class": controller,
        "flags860": flags,
        "flags1c0": flags_1c0,
        "state": state_2a4,
        "throttle": throttle, "brake": brake, "steer": steer, "airl": airl, "airr": airr,
        "speed": speed, "stun": stun,
        "driver": driver,
        "ctl_ptr": ctl,
        "alt_record": dbg.read_u32(craft + 0x40),
        "scale_1d4": f32s(dbg, craft + 0x1D4, 1)[0],
    }
    if ram(ctl):
        row["ctl"] = list(f32s(dbg, ctl, 5))
    if ram(body):
        row["pos"] = list(f32s(dbg, body + 0x30, 3))
        row["vel"] = list(f32s(dbg, body + 0x140, 3))
        row["fwd"] = list(f32s(dbg, body + 0x20, 3))
    crossings, next_lap = u32s(dbg, entity + 0xAC8, 2)
    started, crossed, finished = dbg.read(entity + 0x912 - 2, 3)
    row["crossings"] = crossings
    row["next_lap"] = next_lap
    row["started"], row["crossed"], row["finished"] = started, crossed, finished
    row["lap_clock"] = f32s(dbg, entity + 0x920, 1)[0]
    if full:
        row["progress"] = f32s(dbg, entity + 0xAD0, 2)
        row["best_cs"] = u32s(dbg, entity + 0x92C, 2)
        row["lap_times"] = list(u32s(dbg, entity + 0x934, 6))
    return row


def manager_row(dbg, manager):
    out = {}
    out["mode_state"], out["mode_sub"] = u32s(dbg, manager + 0x7C8, 2)
    out["race_time"] = f32s(dbg, manager + 0x2B8, 1)[0]
    out["flag_1a78"] = dbg.read_u8(manager + 0x1A78)
    out["flag_1a79"] = dbg.read_u8(manager + 0x1A79)
    out["flag_1987"] = dbg.read_u8(manager + 0x1987)
    out["flag_19ed"] = dbg.read_u8(manager + 0x19ED)
    out["spectate_slot"] = dbg.read_u32(manager + 0x1A0C)
    hud = dbg.read_u32(G_HUD)
    out["hud_hidden"] = dbg.read_u8(hud + 0x168) if ram(hud) else None
    cam = dbg.read_u32(CAMERA_OBJECT)
    if ram(cam):
        out["cam_mode"] = dbg.read_u32(cam + 0x1DC)
        out["cam_subject"] = dbg.read_u32(cam + 0x1E0)
        out["cam_subject_prev"] = dbg.read_u32(cam + 0x1E4)
        out["cam_flags2c"] = dbg.read_u32(cam + 0x2C)
    return out


def camera_dump(dbg):
    cam = dbg.read_u32(CAMERA_OBJECT)
    if not ram(cam):
        return None
    return base64.b64encode(dbg.read(cam, 0x300)).decode()


def poll_final_lap(dbg, timeout):
    """Free-running wait until the player is on the last lap (crossings == laps)."""
    end = time.time() + timeout
    last = None
    while time.time() < end:
        time.sleep(2.0)
        manager = dbg.read_u32(RACE_MANAGER)
        if not ram(manager):
            continue
        player = dbg.read_u32(manager + PLAYER_OFFSET)
        if not ram(player):
            continue
        laps = dbg.read_u32(G_RACE_LAPS)
        crossings = dbg.read_u32(player + 0xAC8)
        key = (crossings, laps)
        if key != last:
            print("crossings %d of %d laps (state %s)" % (crossings, laps, dbg.state_name()),
                  file=sys.stderr)
            last = key
        if laps and crossings >= laps:
            return True
        if dbg.read(player + 0x912, 1)[0]:
            return True
    return False


def main():
    parser = argparse.ArgumentParser(description=__doc__,
                                     formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--port", type=int, required=True)
    parser.add_argument("--display", required=True)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--arm-after", type=int, default=60,
                        help="arm the autopilot this many frames after GO")
    parser.add_argument("--laps-hack", type=int, help="write g_race_laps (a shortcut, said so in the log)")
    parser.add_argument("--no-restart", action="store_true")
    parser.add_argument("--disarm-x", type=float,
                        help="on the final lap, once the player's world x passes this, stop the "
                        "autopilot pickup (its timer to 0) and release thrust, so nothing but the "
                        "game's own post-finish driver is left (Talon's Junction: -60)")
    parser.add_argument("--probe", choices=["setstate", "ctl", "scale"],
                        help="instead of logging frames: break on Ship_SetState once the autopilot is "
                        "disarmed, or watch writes to the player's control record once the player "
                        "has finished")
    parser.add_argument("--probe-count", type=int, default=12)
    parser.add_argument("--disarm-z", type=float, default=-187.0,
                        help="and its z must be within 30 of this (the start straight; the circuit "
                        "passes the same x elsewhere on a long race)")
    parser.add_argument("--attach", action="store_true",
                        help="the race is under way and the autopilot is armed: just log")
    parser.add_argument("--after", type=float, default=35.0, help="seconds to log past the finish")
    parser.add_argument("--lead", type=int, default=60, help="frames to log before the finish")
    parser.add_argument("--shots", default="0,1,2,3,5,8,12,20,30,45,60,90,120")
    parser.add_argument("--shot-every", type=int, default=60,
                        help="then one photograph every N frames up to --after")
    parser.add_argument("--full-every", type=int, default=15)
    parser.add_argument("--state-every", type=int, default=30,
                        help="read the front end's state name every N frames (1 pins the frame "
                        "`Race End Photo` is entered on)")
    parser.add_argument("--timeout", type=float, default=900.0)
    parser.add_argument("--place-window", action="store_true")
    args = parser.parse_args()

    args.out.mkdir(parents=True, exist_ok=True)
    if args.place_window and not pair.place_window(args.display):
        raise SystemExit("no emulator window on %s" % args.display)
    shots = sorted({int(v) for v in args.shots.split(",") if v})

    dbg = Debugger(args.port)
    log = {"args": {k: str(v) for k, v in vars(args).items()}, "frames": [], "events": [],
           "set_state": [], "ctl_writes": []}
    try:
        if args.attach:
            dbg.resume()
        elif not args.no_restart:
            pair.restart_to_countdown(dbg, hold=True)
        else:
            dbg.resume()
            dbg.hold(cross=True)

        if not args.attach:
            go = None
            armed = False
            frame = 0
            for _, _ in hits(dbg, WEAPONS_DISPATCH_FIRE, 3000, timeout=60.0):
                manager = dbg.read_u32(RACE_MANAGER)
                if not ram(manager):
                    continue
                player = dbg.read_u32(manager + PLAYER_OFFSET)
                if not ram(player):
                    continue
                craft = dbg.read_u32(player + ENTITY_CRAFT)
                record = dbg.read_u32(player + ENTITY_RECORD)
                if not (ram(craft) and ram(record)):
                    continue
                throttle = f32s(dbg, craft + 0x2B8, 1)[0]
                if go is None and throttle > 0.0:
                    go = frame
                    print("GO at stop frame %d" % go, file=sys.stderr)
                if go is not None and frame - go >= args.arm_after:
                    if args.laps_hack:
                        dbg.write_u32(G_RACE_LAPS, args.laps_hack)
                        log["events"].append({"frame": frame, "laps_hack": args.laps_hack})
                    dbg.write_u32(record + HELD_WORD, 0xFFFFFFFF)
                    dbg.write_u32(record + FIRE_WORD, dbg.read_u32(record + FIRE_WORD) | AUTOPILOT_BIT)
                    armed = True
                    print("autopilot bit written at stop frame %d" % frame, file=sys.stderr)
                    break
                frame += 1
                if frame > 1500:
                    raise SystemExit("no GO within 1500 frames")
            # One more dispatch so the handler runs, then raise the timer it armed.
            for _, _ in hits(dbg, WEAPONS_DISPATCH_FIRE, 3, timeout=30.0):
                pass
            dbg.brk()
            timer = f32s(dbg, record + AUTO_TIMER, 1)[0]
            flags_now = dbg.read_u32(record + FIRE_WORD)
            log["events"].append({"autopilot_timer_armed": timer, "fire_word": flags_now})
            print("autopilot timer armed at %.2f s, fire word %#x" % (timer, flags_now), file=sys.stderr)
            dbg.write(record + AUTO_TIMER, struct.pack("<f", 1.0e9))
            dbg.resume()

        if not poll_final_lap(dbg, args.timeout):
            raise SystemExit("the player never reached the final lap")

        finished_frame = None
        disarmed = False
        frame = 0
        shots_done = set()
        last_state = None
        interval = 0
        gen = hits(dbg, WEAPONS_DISPATCH_FIRE, 400000, timeout=120.0)
        probe_now = None
        for _, _ in gen:
            manager = dbg.read_u32(RACE_MANAGER)
            if not ram(manager):
                continue
            player = dbg.read_u32(manager + PLAYER_OFFSET)
            if not ram(player):
                continue
            if args.disarm_x is not None and not disarmed:
                rec = dbg.read_u32(player + ENTITY_RECORD)
                pcraft = dbg.read_u32(player + ENTITY_CRAFT)
                pbody = dbg.read_u32(pcraft + 0x1CC)
                px, _py, pz = f32s(dbg, pbody + 0x30, 3)
                if (args.disarm_x < px < args.disarm_x + 60 and abs(pz - args.disarm_z) < 30 and dbg.read_u32(player + 0xAC8) >= dbg.read_u32(G_RACE_LAPS)
                        and f32s(dbg, manager + 0x2B8, 1)[0] > 20.0):
                    dbg.write(rec + AUTO_TIMER, struct.pack("<f", 0.0))
                    dbg.hold(cross=False)
                    disarmed = True
                    log["events"].append({"frame": frame, "disarmed_at_x": px})
                    print("autopilot disarmed and thrust released at x=%.1f, stop frame %d"
                          % (px, frame), file=sys.stderr)
                    if args.probe == "setstate":
                        probe_now = ("setstate", player)
                        break
                    if args.probe == "scale":
                        probe_now = ("scale", pcraft + 0x1D4)
                        break
            full = frame % args.full_every == 0
            n = dbg.read_u32(G_RACER_COUNT)
            crafts = []
            for i in range(min(n, 8)):
                entity = dbg.read_u32(manager + SLOTS + 4 * i)
                crafts.append(craft_row(dbg, entity, full) if ram(entity) else None)
            ticks = dbg.call("cpu.status")["ticks"]
            row = {"frame": frame, "cycle_frame": int(ticks / CYCLES_PER_FRAME),
                   "player_entity": player, "mgr": manager_row(dbg, manager), "crafts": crafts}
            if frame % args.state_every == 0:
                state = dbg.state_name()
                row["ui_state"] = state
            if frame % 10 == 0:
                row["cam_raw"] = camera_dump(dbg)
            me = next((c for c in crafts if c and c["entity"] == player), None)
            if finished_frame is None and me and me["finished"]:
                finished_frame = frame
                print("player finished at stop frame %d (sampled crossings %d)"
                      % (frame, me["crossings"]), file=sys.stderr)
                if args.probe == "ctl":
                    probe_now = ("ctl", me["ctl_ptr"])
                    break
            if finished_frame is not None:
                k = frame - finished_frame
                row["since_finish"] = k
                if k in shots or (k > 0 and args.shot_every and k % args.shot_every == 0):
                    name = "f%04d.png" % k
                    row["shot"] = name if pair.shoot(args.display, args.out / name) else None
                if k / 59.94 >= args.after:
                    log["frames"].append(row)
                    break
            elif frame >= 40000:
                break
            log["frames"].append(row)
            frame += 1
            if len(log["frames"]) > args.lead and finished_frame is None:
                log["frames"] = log["frames"][-args.lead:]
        gen.close()
        if probe_now and probe_now[0] == "setstate":
            probe_set_state(dbg, probe_now[1], args.probe_count, log)
        elif probe_now and probe_now[0] == "ctl":
            probe_ctl_write(dbg, probe_now[1], args.probe_count, log)
        elif probe_now and probe_now[0] == "scale":
            probe_ctl_write(dbg, probe_now[1], args.probe_count, log, size=4)
        dbg.resume()
    finally:
        dbg.hold(cross=False)
        (args.out / "log.json").write_text(json.dumps(log))
        dbg.close()
    print("wrote %s" % (args.out / "log.json"))


if __name__ == "__main__":
    main()
