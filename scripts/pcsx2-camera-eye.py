#!/usr/bin/env python3
"""Reads Pulse PS2's chase eye in the craft's own frame, off a savestate or a live PCSX2.

The PS2 counterpart of `psp-camera-pair.py`: it answers "what is the eye's offset
from the craft, in the craft's right/up/forward axes", which is the number the
authored `<ExternalCamera*>` blocks times `g_craft_scale` should equal.

    scripts/pcsx2-camera-eye.py find  STATE.p2s              # locate the craft in a savestate's EE RAM
    scripts/pcsx2-camera-eye.py live  --craft 0x1b374c0 [--slot 28093] [-n 5]

`find` keeps the addresses where `*(c+0x824)+0x30` is a position and both of the
rig's published eyes (`c+0x850` far, `c+0x860` close) sit 8 to 20 units from it, and
prints only the ones whose far/close distances are the ratio of two authored blocks
times a common scale (`far/close` between 1.2 and 1.3). The craft is not at a fixed
address, so run `find` on the same state the emulator is loaded with, then `live`.
`live` reads through PINE (`pcsx2_pine.py`), so start `pcsx2-qt` with
`EnablePINE = true` and the slot you pass; use your own `-datapath` and display.

Layout read off `Ship_UpdateCameraRigs` (`0x00158568`): `craft+0x824` is the body,
whose position is at `+0x30`; `craft+0x404` and `+0x408` point at the forward and up
axes; `craft+0x850` is the far eye and `+0x860` the close eye, written **after**
`g_craft_scale` (`0x0027e8cc`) is applied.
"""

import argparse
import math
import os
import struct
import sys
import time
import zipfile

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

SCALE_ADDR = 0x0027E8CC
RAM_LO, RAM_HI = 0x100000, 0x1F00000


def find(path):
    ram = zipfile.ZipFile(path).read("eeMemory.bin")

    def u32(a):
        return struct.unpack_from("<I", ram, a)[0]

    def f32(a):
        return struct.unpack_from("<f", ram, a)[0]

    print("g_craft_scale in this state: %#010x" % u32(SCALE_ADDR))
    for craft in range(RAM_LO, RAM_HI, 4):
        body = u32(craft + 0x824)
        if not (RAM_LO <= body < RAM_HI - 0x40) or body & 15:
            continue
        pos = [f32(body + 0x30 + 4 * i) for i in range(3)]
        if not all(math.isfinite(x) and abs(x) < 1e5 for x in pos):
            continue
        dist = []
        for off in (0x850, 0x860):
            eye = [f32(craft + off + 4 * i) for i in range(3)]
            if not all(math.isfinite(x) and abs(x) < 1e6 for x in eye):
                break
            dist.append(math.dist(eye, pos))
        else:
            far, close = dist
            if 8 < close < 20 and 1.2 < far / close < 1.3:
                print("craft %#x pos %s far %.3f close %.3f"
                      % (craft, [round(x, 2) for x in pos], far, close))


def live(craft, slot, count):
    from pcsx2_pine import Pine

    pine = Pine(slot=slot)

    def f(addr):
        return struct.unpack("<f", struct.pack("<I", pine.read32(addr)))[0]

    def vec(addr):
        return [f(addr + 4 * i) for i in range(3)]

    for _ in range(count):
        pos = vec(pine.read32(craft + 0x824) + 0x30)
        fwd = vec(pine.read32(craft + 0x404))
        up = vec(pine.read32(craft + 0x408))
        print("scale %#010x  pos %s" % (pine.read32(SCALE_ADDR), [round(x, 2) for x in pos]))
        for name, off in (("close", 0x860), ("far", 0x850)):
            eye = vec(craft + off)
            d = [eye[i] - pos[i] for i in range(3)]
            along = sum(a * b for a, b in zip(d, fwd))
            above = sum(a * b for a, b in zip(d, up))
            print("  %-5s |d| %.3f  forward %.3f  up %.3f" % (name, math.dist(eye, pos), along, above))
        time.sleep(1.0)


def main():
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    one = sub.add_parser("find")
    one.add_argument("state")
    two = sub.add_parser("live")
    two.add_argument("--craft", type=lambda s: int(s, 0), required=True)
    two.add_argument("--slot", type=int, default=28011)
    two.add_argument("-n", type=int, default=5)
    args = parser.parse_args()
    if args.cmd == "find":
        find(args.state)
    else:
        live(args.craft, args.slot, args.n)


if __name__ == "__main__":
    main()
