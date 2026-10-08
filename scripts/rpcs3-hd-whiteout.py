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
    ap.add_argument("--bomb", action="store_true", help="fire state 9 (Bomb) and pause --delay s after the press")
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])

    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            sys.exit("no race")
        session.wait_for_load(70)
        session.tap("cross", settle=22)
        gdb = Debugger(port=port)
        gdb.pause()
        ship, _body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + poll.SLOT_OFFSET, 4))[0]
        gdb.resume()
        mem = open("/proc/%d/mem" % session.proc.pid, "rb", 0)

        def read(addr, n):
            mem.seek(poll.GUEST_BASE + addr)
            return mem.read(n)

        if read(*poll.CODE_CHECK[:1], 4) != poll.CODE_CHECK[1]:
            sys.exit("guest base is not 0x300000000 on this build")
        objs = poll.scan(read, struct.pack(">I", 0x00864B38))
        managers = [m - 0xCC for m in poll.scan(read, struct.pack(">II", objs[0], objs[1]))]
        print("pool objects", [hex(o) for o in objs], "managers", [hex(m) for m in managers], flush=True)
        if len(objs) != 16 or len(managers) != 1:
            sys.exit("pool or manager not found as expected")
        manager = managers[0]

        def pool_count():
            return struct.unpack(">I", read(manager + POOL_COUNT, 4))[0]

        def ages():
            return [struct.unpack(">f", read(o + AGE, 4))[0] for o in objs]

        def capture(stem, note):
            hook = load("whiteout_hook", args.hook.rsplit("/", 1)[-1])
            hook.ST.update(stage=0, asked=set())
            t = time.time()
            used = None if args.no_draws else dump_frame(gdb, out, stem, hook, args.attempts)
            shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
            for off, size in TARGETS:
                try:
                    (out / ("%s-vram-%08x.bin" % (stem, off))).write_bytes(gdb.read(0xC0000000 + off, size))
                except Exception as error:
                    print("  target %#x: %s" % (off, error), flush=True)
            info = {"note": note, "attempt": used, "ages": ages(), "pool_count": pool_count(),
                    "shot": str(shot), "seconds": round(time.time() - t, 1)}
            (out / ("%s.json" % stem)).write_text(json.dumps(info, indent=1))
            print(stem, info["note"], "attempt", used, "pool", info["pool_count"], flush=True)

        if not args.no_baseline:
            gdb.pause()
            capture("base", "before any shot")
            gdb.resume()
            time.sleep(0.5)

        for n, delay in enumerate(args.delay):
            gdb.pause()
            state = 9 if args.bomb else 1
            gdb.write(slot + poll.STATE, struct.pack(">ii", state, state))
            gdb.resume()
            time.sleep(0.5)
            if not args.bomb:
                poll.behind_rival(gdb, args.behind_rival)
            session.pad.set("triangle", True)
            time.sleep(0.12)
            session.pad.set("triangle", False)
            t0 = time.time()
            rose = None
            while not args.bomb and time.time() - t0 < 12.0:
                if pool_count() > 0:
                    rose = time.time()
                    break
                time.sleep(0.003)
            if rose is None and not args.bomb:
                print("shot %d: no hit within 12 s" % n, flush=True)
                continue
            time.sleep(delay)
            gdb.pause()
            capture(("bomb%d" if args.bomb else "hit%d") % n, "delay %.3f after %s" % (delay, "the press" if args.bomb else "pool rise"))
            gdb.resume()
            time.sleep(4.0)
    print("done", flush=True)


if __name__ == "__main__":
    main()
