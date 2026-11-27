#!/usr/bin/env python3
"""A film of an HD Missile or Bomb exploding on a craft, on an attached RPCS3, with no GDB stub.

    source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/<weapons-on grid state> --no-gdb
    OAG_RPCS3_ATTACH=1 OAG_RPCS3_NO_GDB=1 uv run --with evdev python3 scripts/rpcs3-hd-blast-film.py \\
        --weapon missile --out data/reference/hd-capture/missile-hit/film1 [--ages 0.05,0.1,...]

The state must be a Racebox race with Weapons On, `START RACE` up. `--weapon missile` holds the player
`--back` units behind the nearest rival, fires, and reads the hit off the first craft whose shield pool
falls. `--weapon bomb` (and `--weapon mine`) stops the player, drops the weapon, moves the player `--back`
units back along its own heading so the weapon lies ahead of the camera, lets it arm (`--arm-frames`), then
writes a rival onto the drop point until its shield falls. Both then step single game frames and at each age
since the hit keep what `rpcs3-hd-rocket-film.py` keeps (frame with its world and flame clock sidecar, camera
candidates, craft poses and teams, command ring with the vertex spans of its `QUADS` draws) plus mip level 0
of every texture a particle draw binds (`tex/`, read by `scripts/hd-particle-uv.py`) and, for the Bomb, the
blast object's bytes (`NormalBombBlast`, vtable `0x00864ab8`: its matrix at `+0x290`) or for the Missile the
pool entry's age and centre (vtable `0x00864b38`, `+0x170`, `+0xd0`). The pickup words and what each one
fires are on `docs/reverse-engineering/rpcs3-capture.md`.

The target is frozen with `SIGSTOP`, as the Rocket film does; the picture can trail guest memory by a few
ticks, and `compare_env.py film-pair-field` moves a pair's field to the frame's instant.
Raw captures are derived game data: keep them under `data/`.
"""
import argparse
import collections
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


ring = load("rpcs3_hd_particle_ring", "rpcs3-hd-particle-ring.py")
rocket = ring.rocket
film, drive, place, pair = rocket.film, rocket.drive, rocket.place, rocket.pair
import compare_env  # noqa: E402
import ps3_pose  # noqa: E402
import rpcs3_hd_field  # noqa: E402

#: Pickup-slot states (`docs/reverse-engineering/rpcs3-capture.md`, "Giving the player a weapon").
#: The word written into the pickup slot. The Mine's 3 did not fire a Mine (the slot still read it after the
#: press); pass `--state` to try another.
STATES = {"missile": 1, "mine": 3, "bomb": 9}
BOMB_BLAST_VTABLE = 0x00864AB8
BOMB_BLAST_BYTES = 0x4C0
SCAN = (0x30000000, 0x40000000)
CHUNK = 0x100000


def find_objects(read, vtable):
    """Every 4-byte aligned guest address holding `vtable`, in chunks; an unmapped chunk is skipped."""
    key = struct.pack(">I", vtable)
    found = []
    for at in range(SCAN[0], SCAN[1], CHUNK):
        try:
            blob = read(at, CHUNK)
        except Exception:  # noqa: BLE001 - an unmapped chunk
            continue
        i = blob.find(key)
        while i >= 0:
            if i % 4 == 0:
                found.append(at + i)
            i = blob.find(key, i + 1)
    return found


#: How far the chase camera's eye can stand from the player's craft, world units.
EYE_REACH = 25.0


def held_camera(candidates, player):
    """The camera of a frame whose camera does not move between shots, which `ps3_pose.pick_camera`
    cannot tell from a constant: the matrix loaded into both camera registers whose eye is within
    `EYE_REACH` of the player, the one loaded most often. `None` when no matrix qualifies."""
    loads = collections.defaultdict(list)
    for candidate in candidates:
        loads[tuple(candidate[4])].append(candidate)
    near = [group for group in loads.values()
            if ps3_pose.CAMERA_REGISTERS <= {c[5] for c in group} and math.dist(group[0][3], player) < EYE_REACH]
    if not near:
        return None
    address, order, unit_error, eye, matrix, _register = max(near, key=len)[0]
    return {"address": address, "order": order, "unit_error": unit_error, "eye": eye, "view_proj": matrix,
            "registers": sorted(ps3_pose.CAMERA_REGISTERS), "picked": "held camera, %d loads" % len(max(near, key=len))}


def ring_candidates(out, stem):
    """The camera candidates in the frame's dumped command ring, which a held camera is loaded
    into where the push buffer regions hold no usable load of it."""
    candidates = []
    for path in sorted(out.glob("ring/%s-4*.bin" % stem)):
        base = int(path.stem.rsplit("-", 1)[1], 16)
        candidates.extend(ps3_pose.packet_candidates(path.read_bytes(), base=base))
    return candidates


def film_blast(session, gdb, mem, args, out):
    read = gdb.read
    ages = sorted(float(a) for a in args.ages.split(","))
    gdb.pause()
    for _ in range(60):
        gdb.pause()
        try:
            ship, body = place.find_player(gdb)
            clock = mem.f32(ship + film.LAP_CLOCK)
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
    slot = place.u32(read(ship + film.AUTOPILOT_SLOT, 4))
    state = args.state if args.state is not None else STATES[args.weapon]
    teams = {r["ship"]: r["team"] for r in rpcs3_hd_field.field_rows(mem)}

    def shields():
        return {s: struct.unpack(">f", read(s + film.SHIELD, 4))[0]
                for s in struct.unpack(">8I", read(place.CRAFT_ARRAY, 32)) if s and s != ship}

    def step(before_resume=None):
        if before_resume:
            before_resume()
        clock0 = mem.f32(ship + film.LAP_CLOCK)
        gdb.resume()
        deadline = time.time() + 8.0
        while time.time() < deadline:
            now = mem.f32(ship + film.LAP_CLOCK)
            if now - clock0 >= 1.0 / 60 - 1e-4 or now < clock0:
                break
            time.sleep(0.002)
        else:
            gdb.pause()
            sys.exit("the lap clock stopped at %.4f" % clock0)
        gdb.pause()
        return mem.f32(ship + film.LAP_CLOCK) - clock0

    def park(which, pose, rows):
        place.write_pose(gdb, which, tuple(pose), [tuple(x) for x in rows], 0.0)
        gdb.write(which + place.B_VELOCITY, struct.pack(">3f", 0.0, 0.0, 0.0))

    (out / "tex").mkdir(exist_ok=True)
    tex_index = {}
    shots = []
    pool = None
    if args.weapon == "missile":
        pool = find_objects(read, film.POOL_VTABLE)
        print("missile explosion objects: %d" % len(pool), flush=True)
    blasts = find_objects(read, BOMB_BLAST_VTABLE) if args.weapon == "bomb" else []
    if args.weapon == "bomb":
        print("bomb blast objects: %d" % len(blasts), flush=True)

    def blast_state():
        if args.weapon == "missile":
            return [{"object": "%08x" % o, "age": struct.unpack(">f", read(o + film.AGE, 4))[0],
                     "alive": read(o + 0xE0, 1)[0], "centre": list(struct.unpack(">3f", read(o + film.CENTRE, 12)))}
                    for o in pool if read(o + 0xE0, 1)[0]]
        rows = []
        for o in blasts:
            if not read(o + 0x490, 1)[0]:
                continue
            blob = read(o, BOMB_BLAST_BYTES)
            (out / ("blast-%08x-%d.bin" % (o, len(shots)))).write_bytes(blob)
            rows.append({"object": "%08x" % o, "centre": list(struct.unpack_from(">3f", blob, 0x2C0))})
        return rows

    def photograph(stem, note, age):
        time.sleep(args.flush)
        shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
        pose = place.read_pose(gdb, body)
        pose["race_clock_s"] = mem.f32(ship + film.LAP_CLOCK)
        pose["age_since_hit_s"] = age
        pose["craft"] = film.craft_poses(gdb)
        for craft in pose["craft"]:
            craft["team"] = teams.get(int(craft["ship"], 16))
        pose["blasts"] = blast_state()
        pose["pilot_assist"] = read(rpcs3_hd_field.pilot_assist_at(mem), 2).hex()
        candidates = []
        for chain, size in drive.PUSHBUFFER_REGIONS:
            at = drive.resolve_chain(gdb, chain)
            if at:
                candidates.extend(ps3_pose.packet_candidates(gdb.read(at, size), base=at))
        quads = rocket.ring_dump(read, out, stem)
        ring.dump_textures(read, out, stem, tex_index)
        print("%s %s: age %s, %d camera candidate(s), %d QUADS draw(s), %d texture(s), blasts %s"
              % (stem, note, age, len(candidates), quads, len(tex_index), pose["blasts"]), flush=True)
        shots.append((stem, shot, candidates, pose, note))

    info = {"weapon": args.weapon, "state": state, "back": args.back, "player": "%08x" % ship}
    before = shields()

    def dropped_craft():
        nonlocal before
        now = shields()
        hit = [(s, before[s], now[s]) for s in now if now[s] < before.get(s, now[s]) - 0.5]
        before = now
        return hit[0] if hit else None

    if args.weapon == "missile":
        gdb.write(slot + rocket.SLOT_STATE, struct.pack(">ii", state, state))
        session.pad.set("cross", True)
        best = None
        for _ in range(600):
            best = film.nearest_ahead(gdb, ship, place.read_pose(gdb, body))
            if best:
                break
            gdb.resume()
            time.sleep(0.5)
            gdb.pause()
        if best is None:
            sys.exit("no rival ahead of the player")
        _dist, rival, rival_body = best
        print("player %#x, rival %#x (%s)" % (ship, rival, teams.get(rival)), flush=True)

        def hold():
            film.place_behind(gdb, body, rival_body, args.back)

        for _ in range(args.settle):
            step(hold)
        before = shields()
        photograph("pre", "before the shot", None)
        session.pad.set("triangle", True)
        hit = None
        for n in range(args.max_frames):
            step(hold)
            if n == 4:
                session.pad.set("triangle", False)
            got = dropped_craft()
            if got:
                hit = (n,) + got
                break
        after_hit_hold = None
    else:
        # Weapons that lie on the road: the player stops, drops, then stands `back` units behind the drop.
        here = place.read_pose(gdb, body)
        drop, rows = here["pos"], here["rows"]
        info["drop_pose"] = here
        for _ in range(args.settle):
            step(lambda: park(body, drop, rows))
        gdb.write(slot + rocket.SLOT_STATE, struct.pack(">ii", state, state))
        session.pad.set("triangle", True)
        for n in range(args.press_frames):
            step(lambda: park(body, drop, rows))
        session.pad.set("triangle", False)
        info["slot_after_press"] = list(struct.unpack(">ii", read(slot + rocket.SLOT_STATE, 8)))
        print("slot after the press: %s" % info["slot_after_press"], flush=True)
        stand = [drop[i] - args.back * rows[2][i] for i in range(3)]
        info["stand_pos"] = stand

        def hold():
            park(body, stand, rows)

        before = shields()
        for _ in range(args.arm_frames):
            step(hold)
            if dropped_craft():
                sys.exit("a craft's shield fell before the rival was brought in")
        photograph("pre", "weapon lying ahead", None)
        others = [(math.dist(place.read_pose(gdb, struct.unpack(">I", read(s + place.OFF_BODY, 4))[0])["pos"], drop), s)
                  for s in before]
        _dist, rival = min(others)
        rival_body = struct.unpack(">I", read(rival + place.OFF_BODY, 4))[0]
        rival_rows = place.read_pose(gdb, rival_body)["rows"]
        print("rival %#x (%s) brought onto the drop" % (rival, teams.get(rival)), flush=True)

        def trip():
            hold()
            park(rival_body, drop, rows)

        before = shields()
        hit = None
        for n in range(args.max_frames):
            step(trip)
            got = dropped_craft()
            if got:
                hit = (n,) + got
                break
        info["rival_rows_before"] = rival_rows
        after_hit_hold = hold

    session.pad.set("triangle", False)
    if hit is None:
        sys.exit("no craft hit within %d frames" % args.max_frames)
    frames, struck, shield_before, shield_after = hit
    hit_clock = mem.f32(ship + film.LAP_CLOCK)
    struck_body = struct.unpack(">I", read(struck + place.OFF_BODY, 4))[0]
    info.update({"frames_to_hit": frames, "struck": "%08x" % struck, "struck_team": teams.get(struck),
                 "shield_before": shield_before, "shield_after": shield_after, "hit_clock": hit_clock,
                 "struck_pose": place.read_pose(gdb, struck_body), "player_pose": place.read_pose(gdb, body),
                 "blasts_at_hit": blast_state()})
    print("hit after %d frames on %#x: shield %.1f -> %.1f, blasts %s"
          % (frames, struck, shield_before, shield_after, info["blasts_at_hit"]), flush=True)
    photograph("a0000", "hit frame", 0.0)
    session.pad.set("cross", args.weapon == "missile")
    want = list(ages)
    for _ in range(400):
        age = mem.f32(ship + film.LAP_CLOCK) - hit_clock
        while want and age >= want[0] - 1e-6:
            photograph("a%04d" % round(want[0] * 1000), "target %.3f" % want.pop(0), age)
        if not want:
            break
        step(after_hit_hold)
    session.pad.set("cross", False)
    return shots, info


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--weapon", choices=sorted(STATES), required=True)
    ap.add_argument("--state", type=int, help="override the pickup-slot state written (the Mine's is not confirmed)")
    ap.add_argument("--back", type=float, default=12.0, help="units behind the rival, or behind the drop")
    ap.add_argument("--settle", type=int, default=20, help="game frames held before the shot or drop")
    ap.add_argument("--press-frames", type=int, default=4, help="game frames TRIANGLE stays down for a drop")
    ap.add_argument("--arm-frames", type=int, default=75, help="frames a laid weapon arms before the rival comes")
    ap.add_argument("--ages", default="0.05,0.1,0.167,0.233,0.3,0.4,0.55", help="seconds since the hit")
    ap.add_argument("--flush", type=float, default=0.5, help="seconds before a stopped frame is grabbed")
    ap.add_argument("--fire-after", type=float, default=5.0, help="lap clock seconds before the shot is set up")
    ap.add_argument("--max-frames", type=int, default=600, help="frames to wait for the hit")
    args = ap.parse_args()
    out = args.out
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session("attached", str(out / "logs")) as session:
        if not session.attach:
            sys.exit("attach to a `serve` (OAG_RPCS3_ATTACH=1): this script does not boot")
        pair.set_window(*compare_env.platform("hd-fury")["output"])
        handle = session.open_mem()
        gdb = rocket.Frozen(session.proc.pid, handle)
        try:
            shots, info = film_blast(session, gdb, rpcs3_hd_field.Mem(handle), args, out)
        finally:
            if gdb.stopped:
                gdb.resume()
    picks = ps3_pose.pick_camera([c for _, _, c, _, _ in shots])
    frame_candidates = {stem: c for stem, _, c, _, _ in shots}
    for (stem, shot, _, pose, note), (camera, reason, count) in zip(shots, picks):
        if camera is None:
            camera = (held_camera(frame_candidates[stem], pose["pos"])
                      or held_camera(ring_candidates(out, stem), pose["pos"]))
            reason = reason if camera is None else None
        record = drive.describe(camera, reason, count, drive.track_name(), shot, None, None, pose)
        record["note"] = note
        (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
    (out / "hit.json").write_text(json.dumps(info, indent=1) + "\n")
    print("done: %d frames in %s" % (len(shots), out), flush=True)


if __name__ == "__main__":
    main()
