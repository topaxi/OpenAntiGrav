#!/usr/bin/env python3
"""A held weapon fired on HD with the particle draws' command ring and bound textures, on an attached RPCS3.

    source data/scratch/<lane>/env.sh
    scripts/emu-restore-state.sh <lane> data/saves/hd-fury/<weapons-on grid state> --no-gdb
    OAG_RPCS3_ATTACH=1 OAG_RPCS3_NO_GDB=1 uv run --with evdev python3 scripts/rpcs3-hd-particle-ring.py \\
        --out data/scratch/<lane>/films/plasma1 --state 7 --window 0.1,0.9 [--back 30]

The state is a Racebox race with Weapons On, `START RACE` up (or any running race), or a Detonator grid. The
script starts the race, writes the pickup `--state` (0 Rocket, 1 Missile, 2 Quake, 7 Plasma, 9 Bomb,
10 LeachBeam; none in Detonator) to the player's slot, optionally holds the player `--back` units behind the
nearest rival, writes a full EMP charge (`--emp-charge`), holds `--button` for `--press-frames` game frames and then steps single game frames. At every frame of each `--window` (seconds
since the press) it keeps the frame, the command ring with the vertex spans of its `QUADS` draws (`ring/`) and mip level 0 of every texture
a particle draw binds (`tex/<offset>.bin` with `tex/textures.json`). `scripts/hd-particle-uv.py` reads them.
A stop often catches the ring with no particle draw in it, so a frame whose ring holds none is dropped.

The emulator is frozen with `SIGSTOP`, as `rpcs3-hd-rocket-film.py` does.
"""
import argparse
import glob
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


rocket = load("rpcs3_hd_rocket_film", "rpcs3-hd-rocket-film.py")
film, drive, place, pair = rocket.film, rocket.drive, rocket.place, rocket.pair
import compare_env  # noqa: E402
import rpcs3_hd_field  # noqa: E402
import rsx_fifo  # noqa: E402

#: `detonator-emp.md`: `**RACE_MANAGER` is the race manager, `+EMP_CHARGE` its EMP charge, `craft+EMP_GAUGE` the gauge.
RACE_MANAGER = 0x008A6814
EMP_CHARGE = 0x2DD8
EMP_GAUGE = 0x7DA0

#: Bytes per 4x4 block (DXT) or per texel (linear 32 bit) by the format byte with the LN/UN bits off.
BLOCK_BYTES = {0x86: 8, 0x87: 16, 0x88: 16}
TEXEL_BYTES = {0x85: 4, 0x9E: 4}


def texture_bytes(fmt, rect):
    """Mip level 0's size in bytes, or `None` for a format this does not size."""
    code = (fmt >> 8) & 0x9F
    width, height = rect >> 16, rect & 0xFFFF
    if code in BLOCK_BYTES:
        return max(width // 4, 1) * max(height // 4, 1) * BLOCK_BYTES[code]
    if code in TEXEL_BYTES:
        return width * height * TEXEL_BYTES[code]
    return None


def bound_textures(ring, stem):
    """`{(offset, format, rect)}` of unit 0 at every particle `QUADS` draw of one dumped ring."""
    walk = rsx_fifo.Walk(rsx_fifo.Mem.load(sorted(glob.glob("%s/%s-4*.bin" % (ring, stem)))))
    regs, prim, found = {}, 0, set()
    for _pos, method, noninc, args in walk.run(0x1000):
        for i, v in enumerate(args):
            regs[method if noninc else method + 4 * i] = v
        if method != 0x1808:
            continue
        if args[0]:
            prim = args[0]
            continue
        fmt = regs.get(0x1740, 0)
        if prim == 8 and (fmt >> 8 & 0xFF, fmt >> 4 & 0xF, fmt & 0xF) == (16, 3, 3) and 0x1A00 in regs:
            found.add((regs[0x1A00], regs.get(0x1A04, 0), regs.get(0x1A18, 0)))
    return found


def dump_textures(read, out, stem, index):
    for offset, fmt, rect in sorted(bound_textures(out / "ring", stem)):
        key = "%08x" % offset
        size = texture_bytes(fmt, rect)
        if key in index or size is None:
            continue
        base = rocket.LOCAL if fmt & 3 == 1 else rocket.FIFO
        (out / "tex" / (key + ".bin")).write_bytes(read(base + offset, size))
        index[key] = {"format": "%08x" % fmt, "width": rect >> 16, "height": rect & 0xFFFF}
    (out / "tex" / "textures.json").write_text(json.dumps(index, indent=1) + "\n")


def run(session, gdb, mem, args, out):
    read = gdb.read
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
    if args.state is not None:
        slot = place.u32(read(ship + film.AUTOPILOT_SLOT, 4))
        gdb.write(slot + rocket.SLOT_STATE, struct.pack(">ii", args.state, args.state))
        session.pad.set("cross", True)
    rival_body = None
    if args.back > 0:
        for _ in range(600):
            best = film.nearest_ahead(gdb, ship, place.read_pose(gdb, body))
            if best:
                rival_body = best[2]
                break
            gdb.resume()
            time.sleep(0.5)
            gdb.pause()

    def step():
        if rival_body is not None:
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

    for _ in range(args.settle):
        step()
    (out / "tex").mkdir(exist_ok=True)
    index = {}
    shots = {}

    def sample(stem, age):
        quads = rocket.ring_dump(read, out, stem)
        if not quads and stem != "pre":
            for stale in (out / "ring").glob("%s-*.bin" % stem):
                stale.unlink()
            return
        time.sleep(args.flush)
        drive.screenshot(out / ("%s.png" % stem), trim=True)
        dump_textures(read, out, stem, index)
        pose = place.read_pose(gdb, body)
        shots[stem] = {"age_since_press_s": age, "race_clock_s": mem.f32(ship + film.LAP_CLOCK),
                       "player": pose, "craft": film.craft_poses(gdb), "quads_draws": quads}
        (out / "shots.json").write_text(json.dumps(shots, indent=1) + "\n")
        print("%s: age %.3f, %d QUADS draw(s), %d texture(s)" % (stem, age, quads, len(index)), flush=True)

    sample("pre", -1.0)
    if args.emp_charge is not None:
        manager = place.u32(read(place.u32(read(RACE_MANAGER, 4)), 4))
        gdb.write(manager + EMP_CHARGE, struct.pack(">I", args.emp_charge))
        gdb.write(ship + EMP_GAUGE, struct.pack(">f", 1.0))
    session.pad.set(args.button, True)
    pressed = mem.f32(ship + film.LAP_CLOCK)
    frames = 0
    windows = [tuple(float(a) for a in w.split(",")) for w in args.window]
    end = max(hi for _, hi in windows)
    for _ in range(1200):
        age = mem.f32(ship + film.LAP_CLOCK) - pressed
        if any(lo - 1e-6 <= age <= hi + 1e-6 for lo, hi in windows):
            sample("a%05d" % round(age * 1000), age)
        if age > end:
            break
        step()
        frames += 1
        if frames == args.press_frames:
            session.pad.set(args.button, False)
    session.pad.set(args.button, False)
    session.pad.set("cross", False)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--state", type=int, help="held-weapon state to fire")
    ap.add_argument("--button", default="triangle", help="the fire button (l1 fires Detonator's EMP)")
    ap.add_argument("--emp-charge", type=int, help="write this Detonator EMP charge and a full gauge before the press")
    ap.add_argument("--window", required=True, action="append",
                    help="FROM,TO seconds since the press, sampled every game frame; repeatable")
    ap.add_argument("--back", type=float, default=0.0, help="hold the player this far behind a rival; 0 flies free")
    ap.add_argument("--press-frames", type=int, default=4, help="game frames TRIANGLE stays down")
    ap.add_argument("--settle", type=int, default=20)
    ap.add_argument("--flush", type=float, default=0.2)
    ap.add_argument("--fire-after", type=float, default=5.0)
    args = ap.parse_args()
    out = args.out.resolve()
    out.mkdir(parents=True, exist_ok=True)
    with drive.Session("attached", str(out / "logs")) as session:
        if not session.attach:
            sys.exit("attach to a `serve` (OAG_RPCS3_ATTACH=1): this script does not boot")
        pair.set_window(*compare_env.platform("hd-fury")["output"])
        handle = session.open_mem()
        gdb = rocket.Frozen(session.proc.pid, handle)
        try:
            run(session, gdb, rpcs3_hd_field.Mem(handle), args, out)
        finally:
            if gdb.stopped:
                gdb.resume()
    print("done: %s" % out, flush=True)


if __name__ == "__main__":
    main()
