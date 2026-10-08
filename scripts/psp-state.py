#!/usr/bin/env python3
"""Save and load PPSSPP save states on a running SDL instance, by its own pause menu.

    DISPLAY=127.0.0.1:93 uv run --with websocket-client python3 scripts/psp-state.py \\
        --port 45494 save 1
    ... load 1
    ... check 1        # load, then report the craft's pose against the saved one

The debugger has no save-state command, and PPSSPP's F2/F4 hotkeys do not reach
it from `xdotool` on a bare Xvfb (measured 2026-10-08: nothing was written). What
does work is the pause menu: `Escape` opens it, and a **click made of a
`mousemove`, a 0.3 s pause, a `mousedown`, 0.2 s and a `mouseup`** presses a slot's
`Save state` or `Load state` button. A bare `xdotool click` is too fast for the
UI and is ignored. `Escape` toggles, so the menu's state is read from the
emulator: while it is open emulation is paused and the CPU cycle counter stands
still.

The states land in the instance's own memstick, `<HOME>/.config/ppsspp/PSP/
PPSSPP_STATE/` - game-derived data, never committed; keep the ones worth keeping
under `data/saves/pulse-psp/<point>/` (see docs/reverse-engineering/emulator-recipes.md).

Pixel positions are for the 960x544 window the SDL build opens by default: the
slot buttons are at x=280 (Save) and x=406 (Load), y = 28 + 96 * (slot - 1).
"""

import argparse
import importlib.util
import math
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from ppsspp_debugger import Debugger  # noqa: E402

SAVE_X, LOAD_X = 280, 406
SLOT_Y0, SLOT_DY = 28, 96


def xdo(*args):
    subprocess.run(["xdotool", *args], check=True, env=os.environ)


def paused(dbg):
    first = dbg.call("cpu.status")["ticks"]
    time.sleep(0.4)
    return dbg.call("cpu.status")["ticks"] == first


def set_menu(dbg, want_open):
    for _ in range(3):
        if paused(dbg) == want_open:
            return
        xdo("key", "Escape")
        time.sleep(1.2)
    raise SystemExit("could not %s the pause menu" % ("open" if want_open else "close"))


def click(x, y):
    xdo("mousemove", str(x), str(y))
    time.sleep(0.3)
    xdo("mousedown", "1")
    time.sleep(0.2)
    xdo("mouseup", "1")


def state_dir(home):
    return Path(home) / ".config" / "ppsspp" / "PSP" / "PPSSPP_STATE"


def newest_state(home):
    files = sorted(state_dir(home).glob("*.ppst"), key=lambda p: p.stat().st_mtime)
    return files[-1] if files else None


def slot_file(home, slot):
    hits = sorted(state_dir(home).glob("*_%d.ppst" % (slot - 1)))
    return hits[-1] if hits else None


def do_save(dbg, home, slot):
    set_menu(dbg, True)
    before = slot_file(home, slot)
    stamp = before.stat().st_mtime if before else 0
    click(SAVE_X, SLOT_Y0 + SLOT_DY * (slot - 1))
    began = time.time()
    while time.time() - began < 20:
        path = slot_file(home, slot)
        if path and path.stat().st_mtime > stamp:
            time.sleep(0.5)
            set_menu(dbg, False)
            return path
        time.sleep(0.3)
    raise SystemExit("slot %d was not written within 20 s" % slot)


def do_load(dbg, slot):
    set_menu(dbg, True)
    began = time.time()
    click(LOAD_X, SLOT_Y0 + SLOT_DY * (slot - 1))
    time.sleep(1.0)
    set_menu(dbg, False)
    return time.time() - began


def pose(port):
    spec = importlib.util.spec_from_file_location("psp_drive", HERE / "psp-drive.py")
    drive = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(drive)
    with Debugger(port) as dbg:
        craft = drive.find_craft(dbg)
        return drive.read_progress(dbg, craft)


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    ap.add_argument("--home", default=os.environ.get("HOME"),
                    help="the instance's HOME (its memstick is under .config/ppsspp)")
    ap.add_argument("command", choices=["save", "load", "check", "pose"])
    ap.add_argument("slot", type=int, nargs="?", default=1)
    ap.add_argument("--keep", help="copy the saved state to this path (under data/saves/)")
    ap.add_argument("--file", help="for load: first copy this .ppst into the slot's file name")
    args = ap.parse_args()
    if "DISPLAY" not in os.environ:
        sys.exit("DISPLAY must name the instance's own Xvfb, e.g. 127.0.0.1:93")
    with Debugger(args.port) as dbg:
        if args.command == "pose":
            pass
        elif args.command == "save":
            began = time.time()
            path = do_save(dbg, args.home, args.slot)
            print("saved %s (%.1f MB) in %.1f s" % (path, path.stat().st_size / 1e6,
                                                    time.time() - began))
            if args.keep:
                Path(args.keep).parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(path, args.keep)
                print("kept", args.keep)
        else:
            if args.command == "check":
                dbg.close()
                saved = pose(args.port)
                dbg = Debugger(args.port)
            if args.file:
                target = state_dir(args.home) / ("UCUS98712_1.00_%d.ppst" % (args.slot - 1))
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(args.file, target)
                print("placed %s -> %s" % (args.file, target))
                # The menu lists the slot only after a reopen; do_load reopens it.
            took = do_load(dbg, args.slot)
            print("loaded slot %d in %.1f s" % (args.slot, took))
    if args.command in ("check", "pose"):
        time.sleep(1.0)
        now = pose(args.port)
        print("pos %s speed %.3f throttle %.2f" % (
            tuple(round(v, 2) for v in now["pos"]), now["speed"], now["throttle"]))
        if args.command == "check":
            print("saved pos %s; distance %.3f" % (
                tuple(round(v, 2) for v in saved["pos"]),
                math.dist(saved["pos"], now["pos"])))


if __name__ == "__main__":
    main()
