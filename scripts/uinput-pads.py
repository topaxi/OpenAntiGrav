#!/usr/bin/env python3
"""Virtual Xbox-360-shaped gamepads on uinput (two, or `OAG_PADS=N` up to eight), driven by commands on stdin.

For walking the split-screen front end (join screen, Track and Ship Select, a race)
with two real pad devices and no hardware. gilrs sees them like any pad, so
`oag-game` seats them in the order they press a button.

    mkfifo cmd; (sleep 7200 > cmd &)          # keep the fifo open
    env DISPLAY=:96 UV_CACHE_DIR=~/.cache/uv UV_OFFLINE=1 \\
        uv run --with evdev python3 scripts/uinput-pads.py < cmd &
    echo 'a press cross' > cmd                # then drive it from the shell

Commands, one per line: `a|b|c|... press BUTTON`, `a|b|c|... hold BUTTON SECONDS`, `sleep SECONDS`,
`shot PATH` (an ImageMagick `import` of the Xvfb root named by `$DISPLAY`) and `quit`.
BUTTON is one of cross circle square triangle start select up down left right.

Needs write access to /dev/uinput (the `input` group). The devices exist system-wide
while the script runs, so run it only against your own Xvfb session, and stop it by
its PID when done: both pads vanish with it.
"""
import os
import subprocess
import sys
import time

from evdev import AbsInfo, UInput
from evdev import ecodes as e

BTN = {
    "cross": e.BTN_SOUTH,
    "circle": e.BTN_EAST,
    "square": e.BTN_WEST,
    "triangle": e.BTN_NORTH,
    "start": e.BTN_START,
    "select": e.BTN_SELECT,
}
HAT = {
    "up": (e.ABS_HAT0Y, -1),
    "down": (e.ABS_HAT0Y, 1),
    "left": (e.ABS_HAT0X, -1),
    "right": (e.ABS_HAT0X, 1),
}


def make(name):
    stick = AbsInfo(0, -32768, 32767, 16, 128, 0)
    trigger = AbsInfo(0, 0, 255, 0, 0, 0)
    hat = AbsInfo(0, -1, 1, 0, 0, 0)
    cap = {
        e.EV_KEY: [
            e.BTN_SOUTH, e.BTN_EAST, e.BTN_NORTH, e.BTN_WEST, e.BTN_TL, e.BTN_TR,
            e.BTN_SELECT, e.BTN_START, e.BTN_MODE, e.BTN_THUMBL, e.BTN_THUMBR,
        ],
        e.EV_ABS: [
            (e.ABS_X, stick), (e.ABS_Y, stick), (e.ABS_RX, stick), (e.ABS_RY, stick),
            (e.ABS_Z, trigger), (e.ABS_RZ, trigger), (e.ABS_HAT0X, hat), (e.ABS_HAT0Y, hat),
        ],
    }
    return UInput(cap, name=name, vendor=0x045E, product=0x028E, version=0x110, bustype=e.BUS_USB)


def main():
    count = min(8, max(1, int(os.environ.get("OAG_PADS", "2"))))
    pads = {
        letter: make(f"OAG test pad {letter.upper()} (Microsoft X-Box 360 pad)")
        for letter in "abcdefgh"[:count]
    }
    time.sleep(1.5)
    for line in sys.stdin:
        parts = line.split()
        if not parts:
            continue
        if parts[0] == "quit":
            break
        if parts[0] == "sleep":
            time.sleep(float(parts[1]))
            continue
        if parts[0] == "shot":
            subprocess.run(["import", "-display", os.environ["DISPLAY"], "-window", "root", parts[1]])
            continue
        pad, verb, button = pads[parts[0]], parts[1], parts[2]
        hold = float(parts[3]) if verb == "hold" else 0.12
        if button in HAT:
            axis, value = HAT[button]
            pad.write(e.EV_ABS, axis, value)
            pad.syn()
            time.sleep(hold)
            pad.write(e.EV_ABS, axis, 0)
        else:
            pad.write(e.EV_KEY, BTN[button], 1)
            pad.syn()
            time.sleep(hold)
            pad.write(e.EV_KEY, BTN[button], 0)
        pad.syn()
        time.sleep(0.25)
    for pad in pads.values():
        pad.close()


if __name__ == "__main__":
    main()
