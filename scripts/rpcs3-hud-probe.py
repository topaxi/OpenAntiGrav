#!/usr/bin/env python3
"""Poke WipEout HD's shield in a live race and read what the HUD and the craft do.

The tool behind `docs/ghidra/functions/ps3-hdfury-eu/hud-readouts.md`'s
"What the running original does at low shield". It boots a race through
`scripts/rpcs3-drive.py` (Recompiler, so the stub answers only while the
target is stopped), finds the cells by scanning rather than by address - the
heap moves a few kilobytes from boot to boot - and then does one of:

    arming     the post-hit window's arming rule, read off the HUD's own
               timers (`hud+0x10c` last percent, `+0x110` the timer, `+0x1e8`
               the flash accumulator): a fall inside one whole percent, one
               across it, a rise, three times each.
    schedule   a list of shield values held for a few seconds each while
               RPCS3's recorder films the HUD (30 fps); the frames are then
               counted with ffmpeg and numpy by whoever reads the clip.
    sweep      every craft's absorb stamp (`ship+0x6a80`), absorb feedback
               timer (`+0x7a5c`), leach flag (`+0x6958`) and pickup slot state
               every ~1.2 s, to see which pickup events stamp.

Run end to end from this script: `arming` (three boots) and `schedule` (one). `sweep` is the
`arm2.py` sweep the absorb reading came from, folded in and not re-run from here.

A member runs it under a private emulator (see `rpcs3-debugger.md`):

    export XDG_CONFIG_HOME=... XDG_CACHE_HOME=... OAG_RPCS3_DISPLAY=91 \\
           OAG_RPCS3_GDB=127.0.0.1:2391 OAG_RPCS3_SCRATCH_CONFIG=.../cfg.yml
    uv run --with evdev python3 scripts/rpcs3-hud-probe.py --image <iso> arming

The cells, as measured (Feisar, Talon's Junction):

    rm     = [[0x008a6814]]                 the RaceManager
    ships  = rm+0xe8 .. rm+0x104, rm+0x13e8 the local craft is the slot whose
                                            ship+0x7a60 == 0
    shield = ship+0x5fa0   (f32, 140.0)     the HUD's record at hud+0x40
                                            (shield +0x48, max +0x50) mirrors it
    hud    = the object whose +0x40 is that record's address
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
_spec = importlib.util.spec_from_file_location("drive", HERE / "rpcs3-drive.py")
drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(drive)
from rpcs3_debugger import Debugger  # noqa: E402

MAXV = 140.0
PORT = 2391


def f32(v):
    return struct.pack(">f", v)


def u32(g, a):
    return struct.unpack(">I", g.read(a, 4))[0]


def f32r(g, a):
    return struct.unpack(">f", g.read(a, 4))[0]


def is_record(b):
    lap, laps, place = struct.unpack(">3I", b[0xC:0x18])
    sh, mx = struct.unpack(">ff", b[0x48:0x4C] + b[0x50:0x54])
    return laps == 3 and 1 <= lap <= 3 and 1 <= place <= 8 and mx == MAXV and 0 < sh <= 2 * mx


def scan(gdb, lo, hi, test, step=0x800, tail=0x60):
    found = []
    for base in range(lo, hi, step):
        blob = gdb.read(base, step + tail)
        for off in range(0, step, 4):
            if test(blob[off:off + tail]):
                found.append(base + off)
    return found


class Cells:
    """The addresses this boot put the craft, the record and the HUD at."""

    def __init__(self, gdb, record_window, hud_window):
        self.record = scan(gdb, *record_window, is_record)[0]
        holder = u32(gdb, 0x008A6814)
        rm = u32(gdb, holder)
        self.slots = [u32(gdb, rm + 0xE8 + 4 * i) for i in range(8)] + [u32(gdb, rm + 0x13E8)]
        idx = [(u32(gdb, x + 0x7A60) if x else None) for x in self.slots]
        self.ship = next((x for x, i in zip(self.slots, idx) if i == 0), self.slots[-1])
        self.shield = self.ship + 0x5FA0
        want = struct.pack(">I", self.record)
        self.hud = scan(gdb, *hud_window, lambda b: b[0x40:0x44] == want)[0]


def boot_race(session, settle=8.0):
    assert session.wait_for_screen_pressing("Main Menu", 300)
    time.sleep(12)
    assert session.walk_to_race() in drive.RACE_ARRIVED
    time.sleep(50)
    session.pad.press("cross", 0.3)
    time.sleep(settle)


def timers(gdb, cells):
    hb = gdb.read(cells.hud, 0x200)
    cell = f32r(gdb, cells.shield)
    return {
        "pct": round(cell * 100 / MAXV, 3),
        "last_pct_10c": round(struct.unpack(">f", hb[0x10C:0x110])[0], 3),
        "post_hit_110": round(struct.unpack(">f", hb[0x110:0x114])[0], 4),
        "flash_1e8": round(struct.unpack(">f", hb[0x1E8:0x1EC])[0], 4),
    }


def poke(gdb, cells, pct):
    gdb.pause()
    gdb.drain(0.2)
    gdb.write(cells.shield, f32(MAXV * pct / 100.0))
    gdb.resume()


def run_arming(gdb, cells):
    out = []
    poke(gdb, cells, 60.9)
    time.sleep(2.5)
    for rep in range(3):
        for label, pct, wait in (("60.9->60.2", 60.2, 1.5), ("60.2->59.9", 59.9, 1.8)):
            poke(gdb, cells, pct)
            time.sleep(0.25)
            gdb.pause()
            gdb.drain(0.2)
            out.append({"rep": rep, "step": label, **timers(gdb, cells)})
            gdb.resume()
            time.sleep(wait)
        poke(gdb, cells, 60.9)
        time.sleep(1.8)
    poke(gdb, cells, 30.0)
    time.sleep(2.0)
    poke(gdb, cells, 40.0)
    time.sleep(0.25)
    gdb.pause()
    gdb.drain(0.2)
    out.append({"step": "30->40", **timers(gdb, cells)})
    gdb.resume()
    return out


SCHEDULE = [(None, 4.0), (50.0, 5.0), (20.5, 5.0), (20.0, 5.0), (15.0, 4.0), (5.0, 4.0),
            (60.0, 5.0), (100.0, 3.0)]


def run_schedule(session, gdb, cells):
    session.toggle_recording()
    log = []
    for pct, hold in SCHEDULE:
        if pct is not None:
            poke(gdb, cells, pct)
        log.append({"wall": time.time(), "pct": pct})
        time.sleep(hold)
    session.toggle_recording()
    time.sleep(8)
    return log


def run_sweep(gdb, cells, seconds):
    rows = []
    end = time.time() + seconds
    while time.time() < end:
        time.sleep(0.6)
        gdb.pause()
        gdb.drain(0.2)
        row = {"wall": time.time()}
        for si, ship in enumerate(cells.slots[:8]):
            if not ship:
                continue
            obj = u32(gdb, ship + 0x5EDC)
            state = u32(gdb, obj + 0x204) if obj else 0xFFFFFFFF
            row[si] = {
                "t6a30": f32r(gdb, ship + 0x6A30),
                "stamp6a80": f32r(gdb, ship + 0x6A80),
                "leach6958": gdb.read(ship + 0x6958, 1)[0],
                "absorb7a5c": f32r(gdb, ship + 0x7A5C),
                "slot_state": -1 if state >= 0x80000000 else state,
            }
        rows.append(row)
        gdb.resume()
    return rows


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    ap.add_argument("--image", default="data/images/hdfury-ps3-eu-dec.iso")
    ap.add_argument("--out", default=str(Path("data") / "scratch" / "hud-probe"))
    ap.add_argument("--record-window", default="0x307c0000,0x30844000",
                    help="heap window scanned for the HUD's player record")
    ap.add_argument("--hud-window", default="0x32800000,0x32a40000")
    ap.add_argument("--seconds", type=float, default=150.0, help="sweep length")
    ap.add_argument("mode", choices=("arming", "schedule", "sweep"))
    args = ap.parse_args()
    out = Path(args.out)
    out.mkdir(parents=True, exist_ok=True)
    window = lambda t: tuple(int(x, 16) for x in t.split(","))  # noqa: E731
    before = set(drive.recordings())
    with drive.Session(args.image, out) as session:
        boot_race(session)
        with Debugger(port=PORT) as gdb:
            cells = Cells(gdb, window(args.record_window), window(args.hud_window))
            print(json.dumps({k: hex(v) for k, v in vars(cells).items() if k != "slots"}))
            gdb.resume()
            if args.mode == "arming":
                result = run_arming(gdb, cells)
            elif args.mode == "schedule":
                result = run_schedule(session, gdb, cells)
            else:
                session.pad.set("cross", True)
                result = run_sweep(gdb, cells, args.seconds)
                session.pad.set("cross", False)
    (out / (args.mode + ".json")).write_text(json.dumps(result, indent=1))
    fresh = [p for p in drive.recordings() if p not in before]
    print(json.dumps(result, indent=1)[:2000])
    if fresh:
        print("recording:", fresh[-1])


if __name__ == "__main__":
    main()
