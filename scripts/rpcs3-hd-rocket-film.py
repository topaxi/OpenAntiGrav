#!/usr/bin/env python3
"""A film of an HD Rocket striking a craft just ahead, on an attached RPCS3, with no GDB stub.

    source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/<weapons-on grid state> --no-gdb
    OAG_RPCS3_ATTACH=1 OAG_RPCS3_NO_GDB=1 uv run --with evdev python3 scripts/rpcs3-hd-rocket-film.py \\
        --out data/reference/hd-capture/rocket-craft-hit/hit1 [--back 12] [--mode fly|hold] [--ages 0.05,...]

The state must be a Racebox race with Weapons On, `START RACE` up. The script starts the race, writes the
Rocket (pickup state 0) to the player's slot, holds the player `--back` units behind the nearest rival and
fires, and reads the hit off the first rival whose shield pool (`ship + 0x5fa0`) falls. Then it steps single
game frames (the lap clock `ship + 0x7014` advancing 1/60 s) and at each age since the hit keeps the frame,
the camera candidates in the pushbuffer, every craft's pose and team, and the RSX command ring with the
vertex spans of its `QUADS` draws (`ring/`), which `scripts/hd-particle-quads.py` reads. `--mode fly` keeps
thrust held after the hit, `--mode hold` parks the player at its hit pose.

The target is frozen with `SIGSTOP` on the emulator's own pid and read through the `serve` socket: restored
with the stub on, this save state did not answer in time. The picture on screen can therefore trail
guest memory by a few ticks; the ring, the camera and the poses are from the same stop.
Raw captures are derived game data: keep them under `data/`.
"""
import argparse
import glob
import importlib.util
import json
import os
import signal
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


film = load("rpcs3_hd_missile_film", "rpcs3-hd-missile-film.py")
pair = load("rpcs3_hd_pair", "rpcs3-hd-pair.py")
drive = film.drive
place = film.place
import compare_env  # noqa: E402
import ps3_pose  # noqa: E402
import rpcs3_hd_field  # noqa: E402
import rsx_fifo  # noqa: E402

GUEST = 0x300000000
ROCKET = 0
SLOT_STATE = 0x204
FIFO = 0x40000000
LOCAL = 0xC0000000


class Frozen:
    """The slice of the GDB client the film uses, without the stub."""

    def __init__(self, pid, handle):
        comm = Path("/proc/%d/comm" % pid).read_text().strip()
        if not comm.startswith(("AppRun", "rpcs3")):
            sys.exit("pid %d is %r, not an emulator" % (pid, comm))
        self.pid, self.handle, self.stopped = pid, handle, False

    def pause(self):
        os.kill(self.pid, signal.SIGSTOP)
        self.stopped = True

    def resume(self):
        os.kill(self.pid, signal.SIGCONT)
        self.stopped = False

    def read(self, addr, n):
        self.handle.seek(GUEST + addr)
        return self.handle.read(n)

    def write(self, addr, blob):
        self.handle.seek(GUEST + addr)
        self.handle.write(blob)


def ring_dump(read, out, stem):
    """The command ring, the stream's missing pages and the vertex spans of its QUADS draws."""
    ring = out / "ring"
    ring.mkdir(exist_ok=True)

    def save(addr, size):
        try:
            blob = read(addr, size)
        except Exception as err:  # noqa: BLE001 - an unmapped page is reported, not fatal
            print("   ring read %#x+%#x failed: %s" % (addr, size, err), flush=True)
            return
        (ring / ("%s-%08x.bin" % (stem, addr))).write_bytes(blob)

    save(FIFO, 0x20000)
    asked = set()
    quads = []
    for _ in range(24):
        walk = rsx_fifo.Walk(rsx_fifo.Mem.load(sorted(glob.glob("%s/%s-4*.bin" % (ring, stem)))))
        regs, prim, batches, quads = {}, 0, [], []
        for _pos, method, noninc, args in walk.run(0x1000):
            for i, v in enumerate(args):
                regs[method if noninc else method + 4 * i] = v
            if method == 0x1814:
                batches += [((a & 0xFFFFFF), (a >> 24) + 1) for a in args]
            if method != 0x1808:
                continue
            if args[0]:
                prim, batches = args[0], []
            elif prim == 8 and batches:
                quads.append((dict(regs), list(batches)))
        need = [a for a in sorted({p & ~0xFFF for p in walk.missing}) if a not in asked]
        if not need:
            break
        for a in need:
            asked.add(a)
            save(FIFO + a, 0x40000)
    spans = set()
    for regs, batches in quads:
        fmt, off = regs.get(0x1740, 0), regs.get(0x1680, 0)
        stride = (fmt >> 8) & 0xFF
        if not stride:
            continue
        base = (FIFO if off >> 31 else LOCAL) + (off & 0x7FFFFFFF)
        lo = min(f for f, _ in batches)
        hi = max(f + c for f, c in batches)
        spans.add((base + lo * stride, (hi - lo) * stride))
    for addr, size in sorted(spans):
        save(addr, min(size, 0x80000))
    return len(quads)


def film_hit(session, gdb, mem, args, out):
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
    slot = place.u32(gdb.read(ship + film.AUTOPILOT_SLOT, 4))
    gdb.write(slot + SLOT_STATE, struct.pack(">ii", ROCKET, ROCKET))
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
    teams = {r["ship"]: r["team"] for r in rpcs3_hd_field.field_rows(mem)}
    print("player %#x, rival %#x (%s)" % (ship, rival, teams.get(rival)), flush=True)

    def shields():
        return {s: struct.unpack(">f", read(s + film.SHIELD, 4))[0]
                for s in struct.unpack(">8I", read(place.CRAFT_ARRAY, 32)) if s and s != ship}

    def step(hold):
        if hold:
            film.place_behind(gdb, body, rival_body, args.back)
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

    shots = []

    def photograph(stem, note, age):
        time.sleep(args.flush)
        shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
        pose = place.read_pose(gdb, body)
        pose["race_clock_s"] = mem.f32(ship + film.LAP_CLOCK)
        pose["age_since_hit_s"] = age
        pose["craft"] = film.craft_poses(gdb)
        for craft in pose["craft"]:
            craft["team"] = teams.get(int(craft["ship"], 16))
        candidates = []
        for chain, size in drive.PUSHBUFFER_REGIONS:
            at = drive.resolve_chain(gdb, chain)
            if at:
                candidates.extend(ps3_pose.packet_candidates(gdb.read(at, size), base=at))
        quads = ring_dump(read, out, stem)
        print("%s %s: age %s, %d camera candidate(s), %d QUADS draw(s)" % (stem, note, age, len(candidates), quads),
              flush=True)
        shots.append((stem, shot, candidates, pose, note))

    for _ in range(args.settle):
        step(True)
    photograph("pre", "before the shot", None)

    before = shields()
    session.pad.set("triangle", True)
    hit = None
    for n in range(args.max_frames):
        step(True)
        if n == 4:
            session.pad.set("triangle", False)
        now = shields()
        dropped = [s for s in now if now[s] < before.get(s, now[s]) - 0.5]
        if dropped:
            hit = (n, dropped[0], before[dropped[0]], now[dropped[0]])
            break
        before = now
    session.pad.set("triangle", False)
    if hit is None:
        sys.exit("no craft hit within %d frames" % args.max_frames)
    frames, struck, shield_before, shield_after = hit
    hit_clock = mem.f32(ship + film.LAP_CLOCK)
    struck_body = struct.unpack(">I", read(struck + place.OFF_BODY, 4))[0]
    held = place.read_pose(gdb, body)
    info = {"frames_to_hit": frames, "struck": "%08x" % struck, "struck_team": teams.get(struck),
            "shield_before": shield_before, "shield_after": shield_after, "hit_clock": hit_clock,
            "struck_pose": place.read_pose(gdb, struck_body), "player_pose": held, "mode": args.mode,
            "back": args.back}
    print("hit after %d frames on %#x: shield %.1f -> %.1f" % (frames, struck, shield_before, shield_after),
          flush=True)
    photograph("a0000", "hit frame", 0.0)
    session.pad.set("cross", args.mode == "fly")

    want = list(ages)
    for _ in range(400):
        age = mem.f32(ship + film.LAP_CLOCK) - hit_clock
        while want and age >= want[0] - 1e-6:
            photograph("a%04d" % round(want[0] * 1000), "target %.3f" % want.pop(0), age)
        if not want:
            break
        if args.mode == "hold":
            place.write_pose(gdb, body, tuple(held["pos"]), [tuple(x) for x in held["rows"]], 0.0)
            gdb.write(body + place.B_VELOCITY, struct.pack(">3f", 0.0, 0.0, 0.0))
        step(False)
    session.pad.set("cross", False)
    return shots, info


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--back", type=float, default=12.0, help="units behind the rival at the shot")
    ap.add_argument("--mode", choices=["fly", "hold"], default="fly")
    ap.add_argument("--settle", type=int, default=20, help="game frames held in the wake before the shot")
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
        gdb = Frozen(session.proc.pid, handle)
        try:
            shots, info = film_hit(session, gdb, rpcs3_hd_field.Mem(handle), args, out)
        finally:
            if gdb.stopped:
                gdb.resume()
    picks = ps3_pose.pick_camera([c for _, _, c, _, _ in shots])
    for (stem, shot, _, pose, note), (camera, reason, count) in zip(shots, picks):
        record = drive.describe(camera, reason, count, drive.track_name(), shot, None, None, pose)
        record["note"] = note
        (out / ("%s.json" % stem)).write_text(json.dumps(record, indent=2) + "\n")
    (out / "hit.json").write_text(json.dumps(info, indent=1))
    print("done: %d frames in %s" % (len(shots), out), flush=True)


if __name__ == "__main__":
    main()
