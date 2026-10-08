#!/usr/bin/env python3
"""Dump HD's RSX frames at rest, at speed and during a Turbo boost, to see which post-chain programs run.

Boots HD on RPCS3, walks to a race, taps through the flyby and countdown, then for
each `--plan` entry holds throttle / fires a Turbo and dumps one frame of draws with the
`scripts/rpcs3_draw_hook.py` stages (`HOOK_LIGHT=1`). Plan entries, run in order:

    rest              dump at the grid, nothing held
    speed:SECS        hold cross (throttle) for SECS, then dump
    boost:DELAY       with throttle held, write state 4 (Turbo) to the pickup slot, press
                      triangle, wait DELAY seconds, dump
    bomb:DELAY        standing still, write state 9 (Bomb), press triangle, wait DELAY seconds, dump
                      (the blast reaches the player: a damage hit with no speed)

Each frame lands in `<out>/<stem>-<addr>.bin` (see `scripts/rsx-draw-list.py`), `<stem>.png`
and `<stem>.json`. Name the programs with `scripts/rsx-fp-names.py`.

    OAG_RPCS3_DISPLAY=96 OAG_RPCS3_GDB=127.0.0.1:23496 ... HOOK_LIGHT=1 \\
        uv run --with evdev python3 scripts/rpcs3-hd-postchain.py \\
        --image data/images/hdfury-ps3-eu-dec.iso --out <dir> --plan rest --plan speed:6 --plan boost:0.3
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


wh = load("rpcs3_hd_whiteout", "rpcs3-hd-whiteout.py")
poll, drive, place = wh.poll, wh.drive, wh.place
Debugger = wh.Debugger
#: The whiteout lane's three targets, plus the bloom chain's buffers: `0x02150000` is the half-resolution scene
#: (640x360, pitch 0xa00, written once a frame), and the quarter-resolution (320x180, pitch 0x500) `0x02300000` and
#: `0x022c0000` are the ping-pong pair the chain accumulates into, one the other's history.
wh.TARGETS.extend([(0x02280000, 0x500 * 180), (0x022C0000, 0x500 * 180), (0x02240000, 0x500 * 180),
                   (0x02150000, 0xA00 * 360)])


#: The `FunkLayer` global the post-chain runner reads (`0x8b73bc` holds this pointer). `+0x58`/`+0x5c`
#: and `+0x64`/`+0x68` are its two per-viewport inputs to the `FunkLayerZoom` pass; `+0x130 + 4 * viewport`
#: points at the vertex buffer object whose `+0x10` holds the pass's 24-quad CPU mesh (24 records of 0xa0 bytes).
FUNK_LAYER = 0x00C50EE0
#: The tuning struct the Zoom pass reads its per-viewport alpha scale (`+0x00`, `+0x04`) and warp factors
#: (`+0x64` 0.15, `+0x68` 0.95 in the executable's initial data) from.
FUNK_TUNING = 0x008C2B70


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--plan", action="append", default=[])
    ap.add_argument("--hook", default=str(HERE / "rpcs3_draw_hook.py"))
    ap.add_argument("--attempts", type=int, default=10)
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])

    with drive.Session(args.image, str(out / "logs")) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        (out / "rpcs3.pid").write_text("%d\n" % session.proc.pid)
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        time.sleep(20)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            sys.exit("no race")
        print("track: %s" % drive.track_name(), flush=True)
        time.sleep(70)
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

        def capture(stem, note):
            hook = wh.load("postchain_hook", args.hook.rsplit("/", 1)[-1])
            hook.ST.update(stage=0, asked=set())
            t = time.time()
            used = wh.dump_frame(gdb, out, stem, hook, args.attempts)
            shot = drive.screenshot(out / ("%s.png" % stem), trim=True)
            for off, size in wh.TARGETS:
                try:
                    (out / ("%s-vram-%08x.bin" % (stem, off))).write_bytes(gdb.read(0xC0000000 + off, size))
                except Exception as error:
                    print("  target %#x: %s" % (off, error), flush=True)
            funk = gdb.read(FUNK_LAYER, 0x100)
            (out / ("%s-funklayer.bin" % stem)).write_bytes(funk)
            f = struct.unpack(">64f", funk)
            tuning = struct.unpack(">32f", gdb.read(FUNK_TUNING, 0x80))
            template = {}
            for v in range(2):
                ptr = struct.unpack(">I", gdb.read(FUNK_LAYER + 0x130 + 4 * v, 4))[0]
                if not ptr:
                    continue
                try:
                    obj = gdb.read(ptr, 0x40)
                    data = struct.unpack(">I", obj[0x10:0x14])[0]
                    (out / ("%s-zoommesh%d.bin" % (stem, v))).write_bytes(gdb.read(data, 24 * 0xA0))
                    template[v] = [hex(ptr), hex(data)]
                except Exception as error:
                    template[v] = "read failed: %s" % error
            info = {"note": note, "attempt": used, "shot": str(shot), "seconds": round(time.time() - t, 1),
                    "screen": drive.current_screen(),
                    "funk_0x50_0x6c": [round(x, 6) for x in f[0x50 // 4:0x70 // 4]],
                    "zoom_mesh": template,
                    "tuning_0x00_0x7c": [round(x, 6) for x in tuning]}
            (out / ("%s.json" % stem)).write_text(json.dumps(info, indent=1))
            print(stem, note, "attempt", used, flush=True)

        held = False
        for n, item in enumerate(args.plan):
            kind, _, value = item.partition(":")
            if kind == "rest":
                gdb.pause()
                capture("rest%d" % n, "grid, nothing held")
                gdb.resume()
                time.sleep(0.5)
            elif kind == "speed":
                if not held:
                    session.pad.set("cross", True)
                    held = True
                time.sleep(float(value))
                drive.screenshot(out / ("speed%d-pre.png" % n), trim=True)
                gdb.pause()
                capture("speed%d" % n, "throttle held, %s s into this hold" % value)
                gdb.resume()
                time.sleep(0.5)
            elif kind == "boost":
                if not held:
                    session.pad.set("cross", True)
                    held = True
                    time.sleep(6.0)
                gdb.pause()
                gdb.write(slot + poll.STATE, struct.pack(">ii", 4, 4))
                gdb.resume()
                time.sleep(0.5)
                session.pad.set("triangle", True)
                time.sleep(0.12)
                session.pad.set("triangle", False)
                time.sleep(float(value))
                gdb.pause()
                capture("boost%d" % n, "Turbo fired, %s s after the press" % value)
                gdb.resume()
                time.sleep(3.0)
            elif kind == "bomb":
                if held:
                    session.pad.set("cross", False)
                    held = False
                    time.sleep(8.0)
                gdb.pause()
                gdb.write(slot + poll.STATE, struct.pack(">ii", 9, 9))
                gdb.resume()
                time.sleep(0.5)
                session.pad.set("triangle", True)
                time.sleep(0.12)
                session.pad.set("triangle", False)
                time.sleep(float(value))
                gdb.pause()
                capture("bomb%d" % n, "Bomb fired standing, %s s after the press" % value)
                gdb.resume()
                time.sleep(3.0)
            else:
                sys.exit("bad plan entry %r" % item)
        session.pad.set("cross", False)
    print("done", flush=True)


if __name__ == "__main__":
    main()
