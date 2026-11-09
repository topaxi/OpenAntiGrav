#!/usr/bin/env python3
"""A film of HD's Missile hitting a craft, frame by frame from the hit, on an attached RPCS3.

    source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/hd-racebox-talons-venom-weapons-grid.SAVESTAT.zst
    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/rpcs3-hd-missile-film.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --out data/reference/hd-capture/missile-hit/hit0 \\
        [--back 12] [--ages 0.05,0.1,...]

Restore the state before every hit. One run is one hit: it gives the player a Missile (pickup
state 1, `*(ship + 0x5edc) + 0x204`, mirrored at `+0x208`), (the state must have Weapons On:
`hd-racebox-talons-venom-weapons-grid`, START RACE up; the Autopilot is never armed) taps START RACE, lets the
race run to `--fire-after` s of lap clock, holds CROSS, holds the player `--back` units behind the nearest rival ahead along
the rival's velocity, re-placed before every game frame, holds TRIANGLE until the explosion pool
(`MissileManager + 0x10c`) rises, then lets both craft fly free and steps single game frames (the lap clock,
`ship + 0x7014`, advanced 1/60 s per step). At each age in `--ages` (read off the pool object's own age,
`pool[0] + 0x170`, the authoritative clock of the blast) it waits `--flush` seconds for the RSX to present the
frame it was handed, takes the screenshot and, in the same pause, the pushbuffer camera, the blast centre
(`+0xd0..+0xd8`), the player's and every craft's pose. `steps.json` has the pool age after every single step
(the update rate: whether one 1/60 s step moves the age by 1/60 s).

The explosion pool and its manager are found by scanning guest memory for the object vtable `0x00864b38` and
cached in `--cache`, because the heap does not move between restores of one save state. Raw captures are
derived game data: keep them under `data/`. Documented on
docs/ghidra/functions/ps3-hdfury-eu/missile-explosion.md.
"""
import argparse
import importlib.util
import json
import math
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


poll = load("rpcs3_mem_poll", "rpcs3-mem-poll.py")
drive = poll.drive
place = poll.place
import ps3_pose  # noqa: E402
import rpcs3_hd_field  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

AUTOPILOT_SLOT = 0x5EDC
POOL_VTABLE = 0x00864B38
POOL_COUNT = 0x10C
POOL_FIRST = 0xCC
MISSILE_COUNT = 0xC4
AGE = 0x170
CENTRE = 0xD0
LAP_CLOCK = 0x7014
DEFAULT_AGES = "0.02,0.05,0.083,0.1,0.15,0.2,0.25,0.33,0.4,0.5,0.55,0.58,0.6,0.67,0.75,0.83,0.92,1.0,1.08,1.17,1.33,1.5,1.7"


def find_pool(read, cache):
    """The pool's sixteen objects and their manager, from the cache when it still names the vtable."""
    if cache.exists():
        saved = json.loads(cache.read_text())
        objs = saved["objs"]
        if all(struct.unpack(">I", read(o, 4))[0] == POOL_VTABLE for o in objs) \
                and struct.unpack(">II", read(saved["manager"] + POOL_FIRST, 8)) == tuple(objs[:2]):
            return saved["manager"], objs
    objs = poll.scan(read, struct.pack(">I", POOL_VTABLE))
    managers = [m - POOL_FIRST for m in poll.scan(read, struct.pack(">II", objs[0], objs[1]))]
    if len(objs) != 16 or len(managers) != 1:
        sys.exit("pool or manager not found as expected: %d objects, %d managers" % (len(objs), len(managers)))
    cache.parent.mkdir(parents=True, exist_ok=True)
    cache.write_text(json.dumps({"manager": managers[0], "objs": objs}))
    return managers[0], objs


def nearest_ahead(gdb, ship, me):
    best = None
    for s in struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32)):
        if not s or s == ship:
            continue
        b = place.u32(gdb.read(s + place.OFF_BODY, 4))
        p = place.read_pose(gdb, b)
        d = [p["pos"][i] - me["pos"][i] for i in range(3)]
        ahead = sum(d[i] * me["rows"][2][i] for i in range(3))
        dist = math.sqrt(sum(x * x for x in d))
        if ahead > 0 and (best is None or dist < best[0]):
            best = (dist, s, b)
    return best


def place_behind(gdb, body, rival_body, back):
    """The player `back` units behind the rival along its velocity, sharing that velocity and its basis."""
    rp = place.read_pose(gdb, rival_body)
    speed = math.sqrt(sum(v * v for v in rp["vel"]))
    f = [v / speed for v in rp["vel"]]
    pos = tuple(rp["pos"][i] - f[i] * back for i in range(3))
    place.write_pose(gdb, body, pos, [tuple(x) for x in rp["rows"]], speed)
    gdb.write(body + place.B_VELOCITY, struct.pack(">3f", *rp["vel"]))
    return speed


def craft_poses(gdb):
    rows = []
    for ship in struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32)):
        if not ship:
            continue
        role = struct.unpack(">I", gdb.read(ship + place.OFF_ROLE, 4))[0]
        body = struct.unpack(">I", gdb.read(ship + place.OFF_BODY, 4))[0]
        pose = place.read_pose(gdb, body)
        rows.append({"ship": "%08x" % ship, "player": role == 0, "pos": pose["pos"], "rows": pose["rows"],
                     "vel": pose["vel"]})
    return rows


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--back", type=float, default=12.0, help="units behind the rival at the shot (m5: 12)")
    ap.add_argument("--settle", type=int, default=20, help="game frames held in the wake before the press")
    ap.add_argument("--ages", default=DEFAULT_AGES, help="pool ages, seconds, to photograph")
    ap.add_argument("--flush", type=float, default=2.0, help="seconds a paused frame is given to be presented")
    ap.add_argument("--cache", type=Path, help="pool object cache; default <out>/../pool-cache.json")
    ap.add_argument("--fire-after", type=float, default=5.0, help="lap clock seconds to let pass before the player is held in a wake")
    ap.add_argument("--hold-after-hit", action="store_true",
                    help="keep the player at the pose it had at the hit (zero velocity, re-placed before every frame), so "
                         "the eye stays near the blast the way ours' parked scenario's does, instead of flying through it")
    ap.add_argument("--max-frames", type=int, default=900, help="frames to wait for the hit after the press")
    ap.add_argument("--step-timeout", type=float, default=8.0)
    args = ap.parse_args()
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    cache = args.cache or out.parent / "pool-cache.json"
    ages = sorted(float(a) for a in args.ages.split(","))
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])

    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        gdb = Debugger(port=port)
        mem = rpcs3_hd_field.Mem(session.open_mem())

        def read(addr, n):
            return mem.rd(addr, n)

        manager, objs = find_pool(read, cache)
        print("manager %#x, pool %#x..%#x" % (manager, objs[0], objs[-1]), flush=True)

        # START RACE is tapped until the lap clock runs, then the race is let run to `--fire-after`: a press in
        # the first seconds after GO fired nothing (rpcs3-hd-weapon.py).
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
        slot = place.u32(gdb.read(ship + AUTOPILOT_SLOT, 4))
        gdb.write(slot + 0x204, struct.pack(">ii", 1, 1))
        session.pad.set("cross", True)
        best = None
        for _ in range(600):
            best = nearest_ahead(gdb, ship, place.read_pose(gdb, body))
            if best:
                break
            gdb.resume()
            time.sleep(0.5)
            gdb.pause()
        if best is None:
            sys.exit("no rival ahead of the player")
        _dist, rival, rival_body = best
        print("player ship %#x, rival %#x, clock %.2f" % (ship, rival, mem.f32(ship + LAP_CLOCK)), flush=True)

        steps = []

        def pool_count():
            return struct.unpack(">I", read(manager + POOL_COUNT, 4))[0]

        def missiles():
            return struct.unpack(">I", read(manager + MISSILE_COUNT, 4))[0]

        hit_clock = [None]

        def pool_age():
            """The entry's own age while it lives; after its retirement (the pool resets the field) the
            lap clock since the hit, which is the same second-for-second."""
            age = struct.unpack(">f", read(objs[0] + AGE, 4))[0]
            if pool_count() or hit_clock[0] is None:
                return age
            return mem.f32(ship + LAP_CLOCK) - hit_clock[0]

        def step(hold):
            """One game frame, off the lap clock; the player re-placed first when `hold`."""
            if hold:
                place_behind(gdb, body, rival_body, args.back)
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
            return mem.f32(ship + LAP_CLOCK) - clock0

        shots = []
        centre = [None]

        def photograph(stem, note):
            time.sleep(args.flush)
            shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
            pose = place.read_pose(gdb, body)
            pose["race_clock_s"] = mem.f32(ship + LAP_CLOCK)
            pose["pool_age"] = pool_age()
            pose["pool_count"] = pool_count()
            if pose["pool_count"]:
                centre[0] = struct.unpack(">3f", read(objs[0] + CENTRE, 12))
            pose["centre"] = centre[0]
            pose["craft"] = craft_poses(gdb)
            blobs = []
            for chain, size in drive.PUSHBUFFER_REGIONS:
                at = drive.resolve_chain(gdb, chain)
                if at:
                    blobs.append((at, gdb.read(at, size)))
            candidates = []
            for at, blob in blobs:
                candidates.extend(ps3_pose.packet_candidates(blob, base=at))
            print("%s %s: age %.4f pool %d, %d camera candidate(s)" % (stem, note, pose["pool_age"],
                                                                        pose["pool_count"], len(candidates)), flush=True)
            shots.append((stem, shot, candidates, pose, note))

        for n in range(args.settle):
            step(True)
            print("settle %d/%d" % (n + 1, args.settle), flush=True)
        photograph("pre", "before the shot")

        session.pad.set("triangle", True)
        hit = None
        for n in range(args.max_frames):
            dt = step(True)
            count, miss = pool_count(), missiles()
            steps.append({"n": n, "dt": dt, "missiles": miss, "pool": count, "age": pool_age() if count else None})
            if n % 5 == 0 or miss or count:
                print("frame %d: dt %.4f missiles %d pool %d" % (n, dt, miss, count), flush=True)
            if miss and not hit:
                session.pad.set("triangle", False)
            if count:
                hit = n
                break
        session.pad.set("triangle", False)
        session.pad.set("cross", not args.hold_after_hit)
        if hit is None:
            gdb.resume()
            sys.exit("no hit within %d frames (missile list %d)" % (args.max_frames, missiles()))
        held = place.read_pose(gdb, body)
        hit_clock[0] = mem.f32(ship + LAP_CLOCK) - pool_age()
        print("hit after %d frames, pool age %.4f" % (hit, pool_age()), flush=True)

        want = list(ages)
        last = None
        for _ in range(600):
            age = pool_age()
            if last is not None:
                steps.append({"dt_age": age - last, "age": age})
            last = age
            while want and age >= want[0] - 1e-6:
                photograph("a%04d" % round(want[0] * 1000), "target %.3f" % want.pop(0))
                age = pool_age()
            if not want:
                break
            if args.hold_after_hit:
                place.write_pose(gdb, body, tuple(held["pos"]), [tuple(x) for x in held["rows"]], 0.0)
                gdb.write(body + place.B_VELOCITY, struct.pack(">3f", 0.0, 0.0, 0.0))
            step(False)
            print("age %.4f" % pool_age(), flush=True)
        session.pad.set("cross", False)
        gdb.resume()
    picks = ps3_pose.pick_camera([c for _, _, c, _, _ in shots])
    for (stem, shot, _, pose, note), (camera, reason, count) in zip(shots, picks):
        record = drive.describe(camera, reason, count, drive.track_name(), shot, None, None, pose)
        record["note"] = note
        (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
    (out / "steps.json").write_text(json.dumps(steps, indent=1))
    print("done: %d frames in %s" % (len(shots), out), flush=True)


if __name__ == "__main__":
    main()
