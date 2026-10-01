#!/usr/bin/env python3
"""Absorb a held pickup on a live PPSSPP race and read the frames out of EDRAM.

`psp-fire-weapon.py` sets a bit in the fire-request word, which has no absorb
bit: the absorb is not a weapon. It is `Ship_AbsorbHeldPickup` (`0x08844ec4`),
which reads the controller's absorb byte (circle) while the craft holds a
pickup, adds the pickup's shield value, clears the slot, stamps the craft's
`+0x878` with the race clock and calls `Ship_PlayAbsorbFeedback`. So the whole
mechanism is two steps, both measured 2026-09-23
(`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`, "a real
absorb draws the overlay in play"):

1. grant the pickup: write `0` into the player's weapon record at `+0x1bc`
   (`craft+0x1bc` is the held weapon, `-1` for none), **while the CPU is
   stopped** - a write to a free-running emulator does not reach the RAM the
   game reads (see `psp-fire-weapon.py`);
2. hold circle through `input.buttons.send`.

The player is `*(*(0x08b317b4) + 0x2c0)` and its weapon record is `*(player +
0x4c)`; neither is `Ship_UpdateCraft`'s `a0`. The one-second window is the
race clock minus `player+0x878` (the stamp reads `-10.0` until the first
absorb).

Frames: every `--every`-th hit of `Weapons_DispatchFire` (once a frame) after
the stamp appears, until `--frames` are written. With `--edram` both
framebuffers (`0x04000000` and `0x04088000`, 480x272, stride 512, alpha = the bloom's glow
mask; which one is on screen alternates)
and the bloom's final blurred layer (`0x04110000`, 240x136, stride 256) are
written as raw dumps beside a `frames.jsonl` of per-frame clock and window
fraction. EDRAM is only real on PPSSPP's **software renderer**
(`[Graphics] SoftwareRenderer = True`); the OpenGL backend leaves it zero.

    uv run --with websocket-client scripts/psp-absorb-frames.py --port 45682 \\
        --out data/scratch/<lane>/absorb --frames 14 --edram

Stationary poses only: a moving craft's frame lags the CSV row.
"""

import argparse
import json
import struct
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger

WEAPONS_DISPATCH_FIRE = 0x08861814
RACE_MANAGER = 0x08B317B4
PLAYER_OFFSET = 0x2C0
WEAPON_RECORD_OFFSET = 0x4C
HELD_OFFSET = 0x1BC
ABSORB_STAMP = 0x878
CLOCK = 0x830
WINDOW_SECONDS = 1.0

FB_0 = (0x04000000, 512, 480, 272)
FB_1 = (0x04088000, 512, 480, 272)
FB_BLOOM = (0x04110000, 256, 240, 136)


def read_big(dbg, address, size):
    out = b""
    while size > 0:
        n = min(size, 32768)
        out += dbg.read(address, n)
        address += n
        size -= n
    return out


def write_edram(dbg, directory, name, spec):
    address, stride, width, height = spec
    raw = read_big(dbg, address, stride * height * 4)
    path = Path(directory) / name
    path.write_bytes(raw)
    return path


def screenshot(display, path):
    env = {"DISPLAY": display, "PATH": "/usr/bin:/bin"}
    for argv in (["magick", "import"], ["import"]):
        if subprocess.run([*argv, "-window", "root", str(path)], env=env, check=False).returncode == 0:
            return
    raise RuntimeError("no working ImageMagick import")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--frames", type=int, default=14)
    ap.add_argument("--every", type=int, default=1, help="capture every Nth frame after the stamp")
    ap.add_argument("--held", type=int, default=0, help="held-weapon id to grant (0 is what the 2026-09-23 probe used)")
    ap.add_argument("--hold-frames", type=int, default=3, help="frames circle is held")
    ap.add_argument("--control", action="store_true", help="grant nothing and press nothing: the no-absorb control")
    ap.add_argument("--edram", action="store_true")
    ap.add_argument("--ge-dump", type=Path, default=None,
                    help="after the frame --ge-at, resume and record the GE list of the next frame "
                    "(psp-ge-dump.py's `gpu.record.dump`) into this .ppdmp, then stop")
    ap.add_argument("--ge-at", type=int, default=8, help="captured-frame index to dump after")
    ap.add_argument("--display", default=None, help="Xvfb display for a root screenshot of each frame")
    ap.add_argument("--timeout", type=float, default=60.0)
    args = ap.parse_args()
    args.out.mkdir(parents=True, exist_ok=True)

    dbg = Debugger(args.port)
    dbg.resume()
    log = open(args.out / "frames.jsonl", "w")
    written = 0
    hits = 0
    pressed_at = None
    stamp_before = None
    stamp_seen = None
    try:
        for hit, _ in dbg.each_hit(WEAPONS_DISPATCH_FIRE, 100000, timeout=args.timeout):
            hits += 1
            craft = dbg.read_u32(dbg.read_u32(RACE_MANAGER) + PLAYER_OFFSET)
            record = dbg.read_u32(craft + WEAPON_RECORD_OFFSET)
            clock = dbg.read_f32(craft + CLOCK)
            stamp = dbg.read_f32(craft + ABSORB_STAMP)
            if pressed_at is None and not args.control:
                dbg.write_u32(record + HELD_OFFSET, args.held)
                dbg.hold(circle=True)
                pressed_at = hits
                stamp_before = stamp
                print("granted held=%d, circle down at hit %d clock %.3f" % (args.held, hits, clock), file=sys.stderr)
                continue
            if pressed_at is not None and hits - pressed_at == args.hold_frames:
                dbg.hold(circle=False)
            if args.control:
                if hits < 3:
                    continue
                age = 0.0
            else:
                if stamp == stamp_before or stamp > clock:
                    if hits - pressed_at > 60:
                        print("no absorb stamp after 60 frames; held now 0x%08x" % dbg.read_u32(record + HELD_OFFSET), file=sys.stderr)
                        raise SystemExit(1)
                    continue
                if stamp_seen is None:
                    stamp_seen = hits
                age = clock - stamp
                if (hits - stamp_seen) % args.every:
                    continue
            name = "f%02d" % written
            row = {"frame": written, "hit": hits, "clock": clock, "stamp": stamp, "age": age,
                   "window_fraction": age / WINDOW_SECONDS,
                   "held": dbg.read_u32(record + HELD_OFFSET)}
            if args.edram:
                write_edram(dbg, args.out, name + "_fb0.bin", FB_0)
                write_edram(dbg, args.out, name + "_fb1.bin", FB_1)
                write_edram(dbg, args.out, name + "_bloom.bin", FB_BLOOM)
            if args.display:
                screenshot(args.display, args.out / (name + "_root.png"))
            log.write(json.dumps(row) + "\n")
            log.flush()
            print("%s age %.3f held 0x%x" % (name, age, row["held"]), file=sys.stderr)
            written += 1
            if args.ge_dump is not None and written - 1 == args.ge_at:
                import base64
                dbg.remove_breakpoint(WEAPONS_DISPATCH_FIRE)
                dbg.resume()
                reply = dbg.call("gpu.record.dump", timeout=120)
                args.ge_dump.parent.mkdir(parents=True, exist_ok=True)
                args.ge_dump.write_bytes(base64.b64decode(reply["uri"].split(",", 1)[1]))
                dbg.brk()
                clock_after = dbg.read_f32(craft + CLOCK)
                print("GE dump written to %s, window age after %.3f" % (args.ge_dump, clock_after - stamp), file=sys.stderr)
                break
            if written >= args.frames:
                break
    finally:
        dbg.resume()
        dbg.hold(circle=False)
        dbg.close()
        log.close()


if __name__ == "__main__":
    main()
