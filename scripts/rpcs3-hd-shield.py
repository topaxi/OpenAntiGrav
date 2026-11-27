#!/usr/bin/env python3
"""A raised HD Shield, and an absorbed hit on it, filmed on a live RPCS3 with the shield object read beside each frame.

    source data/scratch/<lane>/env.sh      # a `rpcs3-drive.py serve` on the weapons-on Talon's grid state
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-shield.py steady --out DIR --ages 3.0,4.0
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-shield.py flash --out DIR --after 2,4,6,8,10,14
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-shield.py plain --out DIR --laps 8.6

The player's pickup slot is written with the Shield state (5) and TRIANGLE pressed. Every pause keeps what
`rpcs3-hd-blast-film.py` keeps (frame with its clock sidecar, camera candidates, craft poses and teams) plus the
shield object `*(craft+0x6f6c)`: live colour `+0xc0`, target `+0xd0`, swell `+0xe4`, active `+0xf4`, clock
`+0xf8` (`docs/ghidra/functions/ps3-hdfury-eu/shield.md`). `flash` waits for the shield to settle, writes pending
damage into the slot (`+0x120`, the word `Ship_ApplyPendingDamage_q` consumes and, with the slot's shield bit set,
answers with `ShipShield_Hit`), steps single game frames and reads the object after each. `plain` raises no
Shield and photographs the same craft at the named lap clocks, the control that separates the shell from the
hull under it. The picture trails
guest memory by a few ticks, so each frame records the ticks since the hit at its pause.
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


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


drive = load("rpcs3_drive", "rpcs3-drive.py")
pair = load("rpcs3_hd_pair", "rpcs3-hd-pair.py")
blast = load("rpcs3_hd_blast_film", "rpcs3-hd-blast-film.py")
import compare_env  # noqa: E402
import ps3_pose  # noqa: E402
import rpcs3_hd_field  # noqa: E402
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

SLOT_OFFSET = 0x5EDC
SHIELD_OBJECT = 0x6F6C
STATE = 0x204
FLAGS = 0x200
PENDING = 0x120
LAP_CLOCK = 0x7014
SHIELD_STATE = 5


def u32(blob, at=0):
    return struct.unpack_from(">I", blob, at)[0]


def f32(blob, at=0):
    return struct.unpack_from(">f", blob, at)[0]


class Live:
    """The player's craft, its pickup slot and its shield object, read through `serve`."""

    def __init__(self, gdb, mem, ship, body):
        self.gdb, self.mem, self.ship, self.body = gdb, mem, ship, body
        self.slot = mem.u32(ship + SLOT_OFFSET)
        self.shield = mem.u32(ship + SHIELD_OBJECT)

    def clock(self):
        return self.mem.f32(self.ship + LAP_CLOCK)

    def read(self):
        blob = self.mem.rd(self.shield, 0x120)
        return {
            "live_rgba": [f32(blob, 0xC0 + 4 * i) for i in range(4)],
            "target_rgba": [f32(blob, 0xD0 + 4 * i) for i in range(4)],
            "colour_rate": f32(blob, 0xE0),
            "swell": f32(blob, 0xE4),
            "swell_target": f32(blob, 0xE8),
            "active": blob[0xF4],
            "fading": blob[0xF5],
            "time_s": f32(blob, 0xF8),
            "authored_rgba": [f32(blob, 0x104 + 4 * i) for i in range(4)],
            "slot_flags": self.mem.u32(self.slot + FLAGS),
            "pending_damage": self.mem.f32(self.slot + PENDING),
            "raw": blob.hex(),
        }

    def step(self, ticks=1):
        """Run until the lap clock has moved `ticks` game frames, then pause. The target is paused on entry."""
        start = self.clock()
        self.gdb.resume()
        deadline = time.time() + 20.0
        while time.time() < deadline:
            if self.clock() - start >= ticks / 60.0 - 1e-4:
                break
            time.sleep(0.001)
        else:
            sys.exit("the lap clock never advanced")
        self.gdb.pause()


def picture(session, live, out, stem, extra):
    """The paused frame: grab, camera candidates, craft poses, the shield object."""
    gdb = live.gdb
    time.sleep(3.0)
    state = live.read()
    shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
    pose = place.read_pose(gdb, live.body)
    pose["race_clock_s"] = live.clock()
    mem = rpcs3_hd_field.Mem(session.open_mem())
    pose["craft"] = rpcs3_hd_field.field_rows(mem)
    pose["pilot_assist"] = mem.rd(rpcs3_hd_field.pilot_assist_at(mem), 2).hex()
    pose["shield"] = state
    pose.update(extra)
    candidates = []
    for chain, size in drive.PUSHBUFFER_REGIONS:
        at = drive.resolve_chain(gdb, chain)
        if at:
            candidates.extend(ps3_pose.packet_candidates(gdb.read(at, size), base=at))
    return stem, shot, candidates, pose


def write_records(out, shots):
    picks = ps3_pose.pick_camera([c for _, _, c, _ in shots])
    for (stem, shot, candidates, pose), (camera, reason, count) in zip(shots, picks):
        if camera is None:
            # a resting craft's camera does not move between frames, which the picker cannot tell from a constant
            camera = blast.held_camera(candidates, pose["pos"])
            reason = None if camera else reason
        record = drive.describe(camera, reason, count, drive.track_name(), shot, None, None, pose)
        (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
        shield = pose["shield"]
        print("  %s: lap %.3f live %s swell %.4f t %.3f camera %s"
              % (stem, pose["race_clock_s"], ["%.3f" % v for v in shield["live_rgba"]], shield["swell"],
                 shield["time_s"], "yes" if camera else "NONE (%s)" % reason), flush=True)


def start_race(session, gdb, fire_after):
    """Tap START RACE until the lap clock runs, then wait for `fire_after`; returns the player's craft."""
    for _ in range(60):
        gdb.pause()
        try:
            ship, body = place.find_player(gdb)
            clock = struct.unpack(">f", gdb.read(ship + LAP_CLOCK, 4))[0]
        except RuntimeError:
            clock = 0.0
        gdb.resume()
        if clock >= fire_after:
            return ship, body
        if clock < 0.5:
            session.tap("cross", settle=6)
        else:
            time.sleep(1.0)
    sys.exit("the race never reached %.1f s on the lap clock" % fire_after)


def raise_shield(session, live, hold):
    """Write the Shield state and press TRIANGLE; returns the lap clock at the press."""
    gdb = live.gdb
    gdb.pause()
    gdb.write(live.slot + STATE, struct.pack(">ii", SHIELD_STATE, SHIELD_STATE))
    before = live.clock()
    gdb.resume()
    time.sleep(1.0)
    session.pad.set("triangle", True)
    time.sleep(hold)
    session.pad.set("triangle", False)
    for _ in range(40):
        if live.read()["active"]:
            return before, live.clock()
        time.sleep(0.25)
    sys.exit("the shield never went active")


def wait_shield_age(live, age):
    for _ in range(400):
        live.gdb.pause()
        t = live.read()["time_s"]
        if t >= age:
            return
        live.gdb.resume()
        time.sleep(max(0.02, min(0.5, (age - t) / 0.8 - 0.05)))
    sys.exit("the shield clock stopped short of %.2f s" % age)


def run_steady(session, live, args, out):
    shots = []
    for n, age in enumerate(float(a) for a in args.ages.split(",")):
        wait_shield_age(live, age)
        shots.append(picture(session, live, out, "s%02d" % n, {}))
    write_records(out, shots)


def run_plain(session, live, args, out):
    """No Shield: the same resting craft at the named lap clocks."""
    shots = []
    for n, lap in enumerate(float(v) for v in args.laps.split(",")):
        for _ in range(400):
            live.gdb.pause()
            if live.clock() >= lap:
                break
            live.gdb.resume()
            time.sleep(max(0.02, min(0.5, (lap - live.clock()) / 0.8 - 0.05)))
        else:
            sys.exit("the lap clock stopped short of %.2f s" % lap)
        shots.append(picture(session, live, out, "p%02d" % n, {}))
    write_records(out, shots)


def run_flash(session, live, args, out):
    gdb = live.gdb
    wait_shield_age(live, args.hit_age)
    before = live.read()
    gdb.write(live.slot + PENDING, struct.pack(">f", args.damage))
    clock0 = live.clock()
    # step single game frames until the damage word is consumed, reading the object after each
    series = []
    for k in range(1, 6):
        live.step(1)
        state = live.read()
        series.append({"step": k, "lap_clock": live.clock(), **state})
        if state["pending_damage"] == 0.0:
            break
    else:
        sys.exit("the pending damage was never consumed")
    hit_clock = live.clock()
    print("hit: before live %s, after %s swell %.4f (consumed in %d step(s))"
          % (["%.3f" % v for v in before["live_rgba"]], ["%.4f" % v for v in series[-1]["live_rgba"]],
             series[-1]["swell"], len(series)), flush=True)
    shots = []
    for n, after in enumerate(sorted(int(v) for v in args.after.split(","))):
        done = round((live.clock() - hit_clock) * 60)
        if after > done:
            live.step(after - done)
        ticks = round((live.clock() - hit_clock) * 60)
        series.append({"step": "frame%02d" % n, "ticks_since_consumed": ticks, "lap_clock": live.clock(), **live.read()})
        shots.append(picture(session, live, out, "f%02d" % n, {"ticks_since_consumed": ticks}))
    write_records(out, shots)
    (out / "hit.json").write_text(json.dumps(
        {"damage": args.damage, "write": "slot+0x120", "clock_before_write": clock0, "clock_consumed": hit_clock,
         "before": before, "series": series}, indent=2) + "\n")


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("mode", choices=("steady", "flash", "plain"))
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--image", default="/home/topaxi/projects/OpenAntiGrav/data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--fire-after", type=float, default=3.0, help="lap-clock seconds before the shield is raised")
    ap.add_argument("--hold", type=float, default=0.5)
    ap.add_argument("--ages", default="3.0,4.0", help="steady: shield-clock seconds to photograph")
    ap.add_argument("--hit-age", type=float, default=3.0, help="flash: shield-clock seconds at the hit")
    ap.add_argument("--damage", type=float, default=10.0)
    ap.add_argument("--after", default="2,4,6,8,10,14", help="flash: game ticks after the damage is consumed")
    ap.add_argument("--laps", default="8.6", help="plain: lap-clock seconds to photograph")
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    pair.set_window(*compare_env.platform("hd-fury")["output"])
    with drive.Session(args.image, str(out / "logs"), attach=True) as session:
        gdb = Debugger(port=port)
        mem = rpcs3_hd_field.Mem(session.open_mem())
        ship, body = start_race(session, gdb, args.fire_after)
        gdb.pause()
        at = rpcs3_hd_field.pilot_assist_at(mem)
        mem.wr(at, b"\x00\x00")
        live = Live(gdb, mem, ship, body)
        print("craft %#x slot %#x shield %#x" % (ship, live.slot, live.shield), flush=True)
        gdb.resume()
        if args.mode == "plain":
            (out / "run.json").write_text(json.dumps({"mode": "plain"}, indent=2) + "\n")
            run_plain(session, live, args, out)
            gdb.pause()
            gdb.resume()
            return
        before, after = raise_shield(session, live, args.hold)
        print("shield raised between lap %.3f and %.3f" % (before, after), flush=True)
        (out / "run.json").write_text(json.dumps(
            {"mode": args.mode, "press_clock_before": before, "press_clock_after": after,
             "args": {k: str(v) for k, v in vars(args).items()}}, indent=2) + "\n")
        if args.mode == "steady":
            run_steady(session, live, args, out)
        else:
            run_flash(session, live, args, out)
        gdb.pause()
        gdb.resume()


if __name__ == "__main__":
    main()
