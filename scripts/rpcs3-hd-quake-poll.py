#!/usr/bin/env python3
"""Fire a Quake (held-weapon state 2) on an HD race on RPCS3 and poll the wave's state live.

The Quake singleton is the `0x00ae98c0` block (`g_quake`, the global
`Quake_Update`'s module reads through TOC cell `0x008b3c38`):

    +0x70   lifetime (5.0 s)             +0x140  age
    +0x148  the wave's t on its path     +0x150  signed speed
    +0x78   span count (0..2)            +0x88   live-span count
    +0x380  amplitude(age), written by the span sampler

Sampled with `/proc/<pid>/mem` at about 100 Hz without pausing the emulator
(`rpcs3-mem-poll.py`'s trick), together with the player's rigid body position,
so the wave's travel and amplitude read against the game's own clock (`+0x140`),
not the host's. Writes `<out>/series.json`.

    OAG_RPCS3_ATTACH=1 uv run --with evdev python3 scripts/emu-run.py -- \\
        python3 -u scripts/rpcs3-hd-quake-poll.py --image <iso> --out <dir> [--seconds 7]
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
_spec = importlib.util.spec_from_file_location("rpcs3_drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
import rpcs3_place as place  # noqa: E402
from rpcs3_debugger import Debugger  # noqa: E402

GUEST_BASE = 0x300000000
G_QUAKE = 0x00AE98C0
SPAN = 0x400
SLOT_OFFSET = 0x5EDC
STATE = 0x204


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--image", required=True)
    ap.add_argument("--out", required=True)
    ap.add_argument("--seconds", type=float, default=7.0)
    ap.add_argument("--start-wait", type=float, default=12.0)
    ap.add_argument("--countdown", type=float, default=12.0)
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    port = int(drive.GDB_SERVER.rsplit(":", 1)[1])
    with drive.Session(args.image, str(out / "logs")) as session:
        if not session.wait_for_screen_pressing("Main Menu", 240):
            sys.exit("never reached the Main Menu")
        session.settle_menu(20)
        if session.walk_to_race() not in drive.RACE_ARRIVED:
            sys.exit("no race")
        session.wait_for_load(70)
        time.sleep(args.start_wait)
        session.pad.press("cross", 0.15)
        time.sleep(args.countdown)
        gdb = Debugger(port=port)
        gdb.pause()
        ship, body = place.find_player(gdb)
        slot = struct.unpack(">I", gdb.read(ship + SLOT_OFFSET, 4))[0]
        gdb.write(slot + STATE, struct.pack(">ii", 2, 2))
        gdb.resume()
        mem = session.open_mem()

        def read(addr, n):
            mem.seek(GUEST_BASE + addr)
            return mem.read(n)

        base = read(G_QUAKE, SPAN)
        time.sleep(0.5)
        samples = []
        session.pad.set("triangle", True)
        t0 = time.time()
        pressed = True
        while time.time() - t0 < args.seconds:
            now = time.time() - t0
            if pressed and now > 0.12:
                session.pad.set("triangle", False)
                pressed = False
            samples.append({
                "t": now,
                "q": read(G_QUAKE, SPAN).hex(),
                "pos": read(body + place.B_POS, 16).hex(),
            })
            time.sleep(0.004)
        (out / "series.json").write_text(json.dumps({"base": base.hex(), "samples": samples}))
        print("%d samples over %.1f s" % (len(samples), args.seconds), flush=True)


if __name__ == "__main__":
    main()
