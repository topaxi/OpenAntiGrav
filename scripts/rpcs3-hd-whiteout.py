#!/usr/bin/env python3
"""Dump the RSX frame of an HD Missile hit at chosen delays after the explosion pool fills.

Boots HD on RPCS3, walks to a race, gives the player a Missile (state 1), puts
the player behind the nearest rival (`rpcs3-mem-poll.py`'s recipe), and for each
`--delay` fires, waits for `MissileManager + 0x10c` (the explosion pool's count)
to rise, sleeps the delay, pauses and dumps one frame of draws with the
`scripts/rpcs3_draw_hook.py` stages (`HOOK_LIGHT=1`). A baseline frame is dumped
before the first shot. Each frame lands in `<out>/<stem>.png` plus
`<stem>-<addr>.bin` and `<stem>.json` (pool entry ages read at the pause).

    OAG_RPCS3_DISPLAY=96 OAG_RPCS3_GDB=127.0.0.1:23496 ... HOOK_LIGHT=1 \\
        uv run --with evdev python3 scripts/rpcs3-hd-whiteout.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --out <dir> --delay 0.1 --delay 0.4

`--leach` (2026-10-12) fires state 10 instead: the player goes `--behind-rival`
units behind the nearest rival at its speed, waits `--settle` seconds (a shot
fired at once after the teleport ends in state 6), holds TRIANGLE (the beam
holds only while fire is held), pauses `--delay` after state 4 and in the same
pause reads every craft's body and the beam (state, clock, the drawn strip's
samples, wobble words) into `<stem>.json`. The state, beam clock and phase are
logged live at every state change. `HOOK_LEACH=1` dumps the strip draw's own
indices and vertices only. See leach-beam-strips.md, "hd-leach-2".
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


poll = load("rpcs3_mem_poll", "rpcs3-mem-poll.py")
drive = poll.drive
place = poll.place
from rpcs3_debugger import Debugger  # noqa: E402

POOL_COUNT = 0x10C
#: LeachBeam's vtable (`LeachBeam_Construct` 0x00153fb0 stores it), its state word
#: (2 ball, 3 reveal, 4 held, 6 ended), its clock, the drawn strip and that strip's
#: sample count, shooter and target craft (leach-beam-strips.md).
LEACH_VTABLE = 0x00864AF8
LEACH_STATE = 0x4C
LEACH_CLOCK = 0xC934
LEACH_STRIP = 0x6420
STRIP_COUNT = 0x144
STRIP_SAMPLES = 0x150
STRIP_SAMPLE = 0x50
STRIP_TARGET = 0x138
STRIP_SHOOTER = 0x13C
WOBBLE = 0x63C4
SLOT_TARGET = 0x1AC
AGE = 0x170
RING = (0x40000000, 0x20000)
TARGETS = [(0x02300000, 0x1400 * 180), (0x00CC0000, 0x1400 * 720), (0x00F50000, 0x2800 * 720)]


def dump_frame(gdb, out, stem, hook, attempts):
    """Read the paused frame; `None` when no attempt came out complete."""
    for attempt in range(attempts):
        for old in out.glob("%s-*.bin" % stem):
            old.unlink()
        blob = gdb.read(*RING)
        (out / ("%s-%08x.bin" % (stem, RING[0]))).write_bytes(blob)
        retry = False
        for round_no in range(24):
            extra = hook.regions(str(out), stem, round_no)
            if extra is None:
                retry = True
                break
            if not extra:
                break
            for addr, size in extra:
                try:
                    (out / ("%s-%08x.bin" % (stem, addr))).write_bytes(gdb.read(addr, size))
                except Exception as error:
                    print("  dump %#x+%#x failed: %s" % (addr, size, error), flush=True)
        if not retry:
            return attempt
        print("  %s: incomplete frame, retry %d" % (stem, attempt), flush=True)
        hook.ST.update(stage=0, asked=set())
        gdb.resume()
        time.sleep(0.5)
        gdb.pause()
    return None


def read_span(gdb, addr, size):
    out = b""
    while len(out) < size:
        n = min(0x400, size - len(out))
        out += gdb.read(addr + len(out), n)
    return out


def craft_poses(gdb):
    """Every craft's ship, role word and body pose, read from the stopped target."""
    ships = struct.unpack(">8I", gdb.read(place.CRAFT_ARRAY, 32))
    rows = []
    for ship in ships:
        if not ship:
            continue
        role = struct.unpack(">I", gdb.read(ship + place.OFF_ROLE, 4))[0]
        body = struct.unpack(">I", gdb.read(ship + place.OFF_BODY, 4))[0]
        pose = place.read_pose(gdb, body)
        rows.append({"ship": "%08x" % ship, "player": role == 0, "pos": pose["pos"],
                     "rows": pose["rows"], "vel": pose["vel"]})
    return rows


def leach_record(gdb, beam):
    """The beam's state, clock, the drawn strip's samples, ends and wobble words."""
    head = read_span(gdb, beam, 0x100)
    strip = beam + LEACH_STRIP
    count = struct.unpack(">i", gdb.read(strip + STRIP_COUNT, 4))[0]
    samples = []
    if 0 < count <= 300:
        blob = read_span(gdb, strip + STRIP_SAMPLES, count * STRIP_SAMPLE)
        samples = [struct.unpack_from(">20f", blob, i * STRIP_SAMPLE) for i in range(count)]
    ends = struct.unpack(">II", gdb.read(strip + STRIP_TARGET, 8))
    return {"beam": "%08x" % beam,
            "state": struct.unpack_from(">i", head, LEACH_STATE)[0],
            "clock": struct.unpack(">f", gdb.read(beam + LEACH_CLOCK, 4))[0],
            "count": count, "target": "%08x" % ends[0], "shooter": "%08x" % ends[1],
            "wobble": struct.unpack(">12f", gdb.read(strip + WOBBLE, 48)),
            "head": head.hex(), "samples": samples}


def leach_shots(args, session, gdb, slot, beams, beam_state, capture, beam_live):
    """One shot per `--delay`: place, wait for a target, fire, pause in state 4."""
    session.pad.set("cross", True)
    try:
        for n, delay in enumerate(args.delay):
            gdb.pause()
            gdb.write(slot + poll.STATE, struct.pack(">ii", 10, 10))
            gdb.resume()
            poll.behind_rival(gdb, args.behind_rival)
            # A shot fired at once after the teleport ends in state 6 on the next
            # update (eight of eight); every held beam came after a few seconds
            # behind the rival, so the lock is given time to form.
            time.sleep(args.settle)
            # The beam holds only while fire is held: a tap ends it at once.
            session.pad.set("triangle", True)
            t0 = time.time()
            held = None
            seen = []
            while time.time() - t0 < 6.0:
                states = [beam_state(b) for b in beams]
                if not seen or [x[0] for x in seen[-1][1]] != states:
                    seen.append((round(time.time() - t0, 3), [beam_live(b) for b in beams]))
                if 4 in states:
                    held = time.time()
                    break
                time.sleep(0.003)
            print("shot %d: beam states %s" % (n, seen), flush=True)
            if held is None:
                session.pad.set("triangle", False)
                print("shot %d: no held beam within 6 s" % n, flush=True)
                continue
            time.sleep(delay)
            for k in range(args.frames):
                gdb.pause()
                capture("leach%d_%d" % (n, k), "delay %.3f + %d x %.2f after state 4"
                        % (delay, k, args.frame_gap))
                gdb.resume()
                time.sleep(args.frame_gap)
            # The rest of the hold, sampled live: the held duration and the end.
            t1 = time.time()
            tail = []
            while time.time() - t1 < 8.0:
                states = [beam_state(b) for b in beams]
                if not tail or [x[0] for x in tail[-1][1]] != states:
                    tail.append((round(time.time() - held, 3), [beam_live(b) for b in beams]))
                time.sleep(0.005)
            session.pad.set("triangle", False)
            print("shot %d: after the frames %s" % (n, tail), flush=True)
            time.sleep(2.0)
    finally:
        session.pad.set("cross", False)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--delay", type=float, action="append", default=[])
    ap.add_argument("--behind-rival", type=float, default=12.0)
    ap.add_argument("--hook", default=str(HERE / "rpcs3_draw_hook.py"))
    ap.add_argument("--attempts", type=int, default=10)
    ap.add_argument("--no-baseline", action="store_true")
    ap.add_argument("--no-draws", action="store_true", help="skip the pushbuffer dump, read targets only")
    ap.add_argument("--no-targets", action="store_true", help="skip the render-target reads (11 MB over the stub, minutes): the command stream and its vertex-program constants only")
    ap.add_argument("--bomb", action="store_true", help="fire state 9 (Bomb) and pause --delay s after the press")
    ap.add_argument("--leach", action="store_true",
                    help="fire state 10 (LeachBeam) from --behind-rival, pause --delay s after the beam reaches "
                         "state 4 (held), and read the beam, its strip and every craft in the same pause")
    ap.add_argument("--settle", type=float, default=3.0, help="--leach: seconds behind the rival before firing")
    ap.add_argument("--frames", type=int, default=1, help="--leach: pauses per shot, --frame-gap s apart")
    ap.add_argument("--frame-gap", type=float, default=0.3)
    ap.add_argument("--start-wait", type=float, default=0.0, help="seconds after the load before the START RACE tap (the prompt ignores an earlier one; 12 worked)")
    ap.add_argument("--quake", action="store_true", help="fire state 2 (Quake) and pause --delay s after the press")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])

    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        gdb = Debugger(port=port)
        if session.in_race() and not session.in_demo():
            # An attached emulator already racing (a previous capture's race):
            # the craft are placed per shot, so the race's own progress is moot.
            print("already in a race (%s)" % drive.current_screen(), flush=True)
        else:
            if drive.current_screen() != "Single Player" and not session.wait_for_screen_pressing("Main Menu", 240):
                sys.exit("never reached the Main Menu")
            session.settle_menu(20)
            if session.walk_to_race() not in drive.RACE_ARRIVED:
                sys.exit("no race")
            session.wait_for_load(70)
            time.sleep(args.start_wait)
            session.tap("cross", settle=22)
        gdb.pause()
        ship, _body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + poll.SLOT_OFFSET, 4))[0]
        gdb.resume()
        mem = session.open_mem()

        def read(addr, n):
            mem.seek(poll.GUEST_BASE + addr)
            return mem.read(n)

        if read(*poll.CODE_CHECK[:1], 4) != poll.CODE_CHECK[1]:
            sys.exit("guest base is not 0x300000000 on this build")
        if args.leach:
            objs, manager = [], None
            beams = poll.scan(read, struct.pack(">I", LEACH_VTABLE))
            print("leach beams", [hex(b) for b in beams], flush=True)
            if not beams:
                sys.exit("no LeachBeam object found")
        else:
            beams = []
            objs = poll.scan(read, struct.pack(">I", 0x00864B38))
            managers = [m - 0xCC for m in poll.scan(read, struct.pack(">II", objs[0], objs[1]))]
            print("pool objects", [hex(o) for o in objs], "managers", [hex(m) for m in managers], flush=True)
            if len(objs) != 16 or len(managers) != 1:
                sys.exit("pool or manager not found as expected")
            manager = managers[0]

        def pool_count():
            return struct.unpack(">I", read(manager + POOL_COUNT, 4))[0] if manager else 0

        def ages():
            return [struct.unpack(">f", read(o + AGE, 4))[0] for o in objs]

        def beam_state(beam):
            return struct.unpack(">i", read(beam + LEACH_STATE, 4))[0]

        def beam_live(beam):
            """State, beam clock and the first wobble phase, read live without a pause."""
            return (beam_state(beam),
                    round(struct.unpack(">f", read(beam + LEACH_CLOCK, 4))[0], 3),
                    round(struct.unpack(">f", read(beam + LEACH_STRIP + WOBBLE + 8, 4))[0], 3))

        def capture(stem, note):
            hook = load("whiteout_hook", args.hook.rsplit("/", 1)[-1])
            hook.ST.update(stage=0, asked=set())
            t = time.time()
            used = None if args.no_draws else dump_frame(gdb, out, stem, hook, args.attempts)
            shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
            for off, size in ([] if args.leach or args.no_targets else TARGETS):
                try:
                    (out / ("%s-vram-%08x.bin" % (stem, off))).write_bytes(gdb.read(0xC0000000 + off, size))
                except Exception as error:
                    print("  target %#x: %s" % (off, error), flush=True)
            info = {"note": note, "attempt": used, "ages": ages(), "pool_count": pool_count(),
                    "shot": str(shot), "seconds": round(time.time() - t, 1)}
            if args.leach:
                # Read after the dump, in the same pause, so the poses and the
                # strip belong to the frame the screenshot shows.
                info["craft"] = craft_poses(gdb)
                info["beams"] = [leach_record(gdb, b) for b in beams]
            (out / ("%s.json" % stem)).write_text(json.dumps(info, indent=1))
            print(stem, info["note"], "attempt", used, "pool", info["pool_count"], flush=True)

        if not args.no_baseline:
            gdb.pause()
            capture("base", "before any shot")
            gdb.resume()
            time.sleep(0.5)

        if args.leach:
            leach_shots(args, session, gdb, slot, beams, beam_state, capture, beam_live)
            print("done", flush=True)
            return

        for n, delay in enumerate(args.delay):
            gdb.pause()
            state = 9 if args.bomb else 2 if args.quake else 1
            by_press = args.bomb or args.quake
            gdb.write(slot + poll.STATE, struct.pack(">ii", state, state))
            gdb.resume()
            time.sleep(0.5)
            if not by_press:
                poll.behind_rival(gdb, args.behind_rival)
            session.pad.set("triangle", True)
            time.sleep(0.12)
            session.pad.set("triangle", False)
            t0 = time.time()
            rose = None
            while not by_press and time.time() - t0 < 12.0:
                if pool_count() > 0:
                    rose = time.time()
                    break
                time.sleep(0.003)
            if rose is None and not by_press:
                print("shot %d: no hit within 12 s" % n, flush=True)
                continue
            time.sleep(delay)
            gdb.pause()
            capture(("bomb%d" if args.bomb else "quake%d" if args.quake else "hit%d") % n,
                    "delay %.3f after %s" % (delay, "the press" if by_press else "pool rise"))
            gdb.resume()
            time.sleep(8.0 if args.quake else 4.0)
    print("done", flush=True)


if __name__ == "__main__":
    main()
