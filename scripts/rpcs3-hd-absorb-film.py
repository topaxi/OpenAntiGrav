#!/usr/bin/env python3
"""A film of the player's craft absorbing a held pickup on HD, frame by frame from the press, on an attached RPCS3.

    source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid.SAVESTAT.zst
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-absorb-film.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --out data/scratch/<lane>/hd-absorb/a1 [--control]

The recipe of `docs/rendering/weapon-absorb-pairs.md` ("HD (PS3)"): the state is Racebox, Weapons On, START RACE up. START RACE
is tapped until the lap clock runs, the race is let run to `--fire-after` s, then the craft is held at rest (re-placed
before every frame) and given a pickup (state 1 at `*(ship + 0x5edc) + 0x204`, mirrored at `+0x208`). Circle is held until the shell
timer `craft + 0x7a5c` rises to 1.0 (`Ship_PlayAbsorbFeedback`'s store), then single game frames are stepped off the lap clock
(`ship + 0x7014`) and each age in `--ages` is photographed, `age = 1.0 - timer`. `--control` grants and presses nothing and
photographs the same lap-clock offsets from the moment the press would have been. Each shot writes `NN.png` and `NN.json` in
the record shape `scripts/hd-weapon-pair.py` reads (`camera`, `ship.pos/rows/race_clock_s/fire_clock_after_s`).
"""
import argparse
import importlib.util
import json
import os
import struct
import sys
import time
import types
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))


def load(name, file):
    spec = importlib.util.spec_from_file_location(name, HERE / file)
    mod = importlib.util.module_from_spec(spec)
    sys.modules[name] = mod
    spec.loader.exec_module(mod)
    return mod


poll = load("rpcs3_mem_poll", "rpcs3-mem-poll.py")
drive = poll.drive
place = poll.place
import ps3_pose  # noqa: E402
import rpcs3_hd_field  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

AUTOPILOT_SLOT = 0x5EDC
SHELL_TIMER = 0x7A5C
SHELL_FADE = 0x7A50
LAP_CLOCK = 0x7014
DEFAULT_AGES = "0.05,0.1,0.2,0.3,0.5,0.75,1.0,1.2"


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--ages", default=DEFAULT_AGES)
    ap.add_argument("--fire-after", type=float, default=5.0)
    ap.add_argument("--flush", type=float, default=2.0)
    ap.add_argument("--step-timeout", type=float, default=8.0)
    ap.add_argument("--control", action="store_true")
    ap.add_argument("--dump-at", default="",
                    help="comma list of ages (each also in --ages): while the target is paused at that age, dump the frame's RSX "
                         "draws with `rpcs3_draw_hook.py` to OUT/dNN-*.bin, read back with `scripts/rsx-draw-list.py OUT dNN --samplers`")
    ap.add_argument("--dump-attempts", type=int, default=3,
                    help="how often a dump that came back incomplete is retried (each retry runs the game 0.3 s on)")
    args = ap.parse_args()
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    ages = sorted(float(a) for a in args.ages.split(","))
    dump_ages = {float(a) for a in args.dump_at.split(",") if a}
    if dump_ages - set(ages):
        sys.exit("--dump-at ages must also be in --ages")
    os.environ.setdefault("HOOK_LIGHT", "1")
    os.environ.setdefault("HOOK_NO_HEADS", "1")
    dump_args = types.SimpleNamespace(dump=["0x40000000:0x20000"], dump_attempts=args.dump_attempts, dump_gap=0.3,
                                      hook=str(HERE / "rpcs3_draw_hook.py"))
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])

    with drive.Session(args.image, str(out / "logs")) as session:
        gdb = Debugger(port=port)
        mem = rpcs3_hd_field.Mem(session.open_mem())
        for _ in range(60):
            gdb.pause()
            try:
                ship, body = place.find_player(gdb)
                clock = mem.f32(ship + LAP_CLOCK)
            except RuntimeError:
                clock = 0.0
            if clock >= args.fire_after:
                break
            gdb.resume()
            if clock < 0.5:
                session.tap("cross", settle=6)
            else:
                time.sleep(2)
        else:
            sys.exit("the race never reached %.1f s on the lap clock" % args.fire_after)
        held = place.read_pose(gdb, body)

        nudges = [0]

        def step(hold=True):
            if hold:
                # A camera identical in every frame is excluded by `ps3_pose.pick_camera`'s cross-frame rule, so the
                # craft's held pose moves 2 mm per frame along world x: no picture can tell, and every frame's own
                # camera is then the one value that differs between frames.
                nudges[0] += 1
                pos = (held["pos"][0] + 0.002 * nudges[0], held["pos"][1], held["pos"][2])
                place.write_pose(gdb, body, pos, [tuple(x) for x in held["rows"]], 0.0)
                gdb.write(body + place.B_VELOCITY, struct.pack(">3f", 0.0, 0.0, 0.0))
            clock0 = mem.f32(ship + LAP_CLOCK)
            gdb.resume()
            deadline = time.time() + args.step_timeout
            while time.time() < deadline:
                now = mem.f32(ship + LAP_CLOCK)
                if now - clock0 >= 1.0 / 60 - 1e-4 or now < clock0:
                    break
                time.sleep(0.002)
            else:
                gdb.pause()
                sys.exit("the lap clock stopped at %.4f" % clock0)
            gdb.pause()

        timer = lambda: mem.f32(ship + SHELL_TIMER)  # noqa: E731
        press_clock = mem.f32(ship + LAP_CLOCK)
        if not args.control:
            slot = place.u32(gdb.read(ship + AUTOPILOT_SLOT, 4))
            gdb.write(slot + 0x204, struct.pack(">ii", 1, 1))
            session.pad.set("circle", True)
            for n in range(120):
                step()
                if timer() > 0.5:
                    break
            else:
                sys.exit("the shell timer never rose: no absorb")
            session.pad.set("circle", False)
            press_clock = mem.f32(ship + LAP_CLOCK) - (1.0 - timer())
        print("press clock %.4f" % press_clock, flush=True)

        def age():
            return mem.f32(ship + LAP_CLOCK) - press_clock

        shots = []
        want = list(ages)
        while want:
            while want and age() >= want[0] - 1e-6:
                time.sleep(args.flush)
                stem = "%02d" % len(shots)
                shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
                pose = place.read_pose(gdb, body)
                pose["race_clock_s"] = mem.f32(ship + LAP_CLOCK)
                pose["fire_clock_after_s"] = press_clock
                pose["age"] = age()
                pose["timer"] = timer()
                pose["fade"] = mem.f32(ship + SHELL_FADE)
                blobs = []
                for chain, size in drive.PUSHBUFFER_REGIONS:
                    at = drive.resolve_chain(gdb, chain)
                    if at:
                        blobs.append((at, gdb.read(at, size)))
                candidates = []
                for at, blob in blobs:
                    candidates.extend(ps3_pose.packet_candidates(blob, base=at))
                print("%s: want %.3f age %.4f timer %.3f fade %.3f, %d camera candidate(s)"
                      % (stem, want[0], pose["age"], pose["timer"], pose["fade"], len(candidates)), flush=True)
                if want[0] in dump_ages:
                    drive.dump_frame(gdb, out, "d" + stem, dump_args)
                shots.append((stem, shot, candidates, pose, want.pop(0)))
            if want:
                step()
        gdb.resume()
    picks = ps3_pose.pick_camera([c for _, _, c, _, _ in shots])
    for (stem, shot, _, pose, wanted), (camera, reason, count) in zip(shots, picks):
        record = drive.describe(camera, reason, count, drive.track_name(), shot, None, None, pose)
        record["ship"] = pose
        record["wanted_age_s"] = wanted
        (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
    print("done: %d frames in %s" % (len(shots), out), flush=True)


if __name__ == "__main__":
    main()
