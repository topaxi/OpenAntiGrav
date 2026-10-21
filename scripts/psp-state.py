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
    """Whether emulation stands still: the debugger's own `paused` flag (true with the pause menu open, 2026-10-21, v1.20.4),
    or the CPU cycle counter not moving over 0.4 s. The `rerender-cold` lane saw the counter keep advancing with the menu
    open and could not load a state at all; the flag does not depend on it."""
    first = dbg.call("cpu.status")
    if first.get("paused"):
        return True
    time.sleep(0.4)
    return dbg.call("cpu.status")["ticks"] == first["ticks"]


def set_menu(dbg, want_open):
    for _ in range(3):
        if paused(dbg) == want_open:
            return
        xdo("key", "Escape")
        time.sleep(1.2)
    raise SystemExit("could not %s the pause menu" % ("open" if want_open else "close"))


def window_origin():
    """Root coordinates of the emulator window's top-left corner.

    The button positions are relative to the 960x544 window, and `xdotool`
    moves the pointer in root coordinates. On an Xvfb exactly as large as the
    window the two agree; on the 1280x800 screen `ppsspp-start.sh` declares
    there is no window manager, so the window is centred at (160, 128) and every
    click landed in the black margin, outside it (found 2026-10-20: `load`
    reported success on a fresh instance and nothing loaded).
    """
    out = subprocess.run(["xdotool", "search", "--onlyvisible", "--name", ""],
                         capture_output=True, text=True, env=os.environ).stdout.split()
    best = None
    for wid in out:
        geo = subprocess.run(["xdotool", "getwindowgeometry", "--shell", wid],
                             capture_output=True, text=True, env=os.environ).stdout
        vals = dict(line.split("=") for line in geo.split() if "=" in line)
        try:
            x, y, w, h = (int(vals[k]) for k in ("X", "Y", "WIDTH", "HEIGHT"))
        except (KeyError, ValueError):
            continue
        if (w, h) == (960, 544):
            return x, y
        if best is None and w >= 400 and h >= 300:
            best = (x, y)
    return best or (0, 0)


def click(x, y):
    ox, oy = window_origin()
    x, y = x + ox, y + oy
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


def do_load_stopped(dbg, slot, stop_at):
    """Press Load on `slot` with an execution breakpoint armed at `stop_at`, run on and stop at its first hit.

    The stop is then the state's own first frame: emulation from a loaded state is bit-identical run to run, so the
    first hit of any per-frame address is the same instant every time (`scripts/psp-ribbon-ab.py` measured it with
    `Trail_DrawRibbon`: two loads, 0 differing bytes). `do_load` followed by `cpu.stepping` stops wherever the
    emulation has got to after the load, the toast wait and the poll, which differed by 0.6 s of race clock between two
    runs of one state. The breakpoint is removed again and the CPU is left stopped there.
    """
    set_menu(dbg, True)
    dbg.brk()
    dbg.add_breakpoint(stop_at)
    click(LOAD_X, SLOT_Y0 + SLOT_DY * (slot - 1))
    time.sleep(2.5)
    dbg.call("cpu.status")
    dbg.pending = []
    dbg.call("cpu.resume")
    try:
        dbg.wait_for_break(stop_at, timeout=60)
    finally:
        if not dbg.is_stepping():
            dbg.brk()
        dbg.remove_breakpoint(stop_at)


def do_load(dbg, slot):
    """Press Load on `slot`, then leave the menu only if it is still open.

    PPSSPP may close the pause menu itself once a state has loaded, so an
    unconditional `Escape` could reopen it (seen 2026-10-20: "could not close
    the pause menu" after a load that had worked). The CPU is also stood still
    while the state is read, so one reading of `paused()` straight after the
    click cannot tell the two apart: wait for it to run, and press `Escape`
    only when it has not within a few seconds.
    """
    set_menu(dbg, True)
    began = time.time()
    click(LOAD_X, SLOT_Y0 + SLOT_DY * (slot - 1))
    time.sleep(1.5)
    for attempt in range(2):
        for _ in range(8):
            if not paused(dbg):
                return time.time() - began
            time.sleep(0.5)
        if attempt == 0:
            xdo("key", "Escape")
            time.sleep(1.2)
    raise SystemExit("the game did not resume after the load")


def pose(port):
    spec = importlib.util.spec_from_file_location("psp_drive", HERE / "psp-drive.py")
    drive = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(drive)
    with Debugger(port) as dbg:
        craft = drive.find_craft(dbg)
        try:
            return drive.read_progress(dbg, craft)
        finally:
            # find_craft stops the CPU at a breakpoint. Left stopped, every
            # later `paused()` reading is true, so the pause menu's state can no
            # longer be told from the debugger's own stop.
            dbg.resume()


def main():
    ap = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    ap.add_argument("--port", type=int, default=int(os.environ.get("OAG_PPSSPP_PORT", 47810)))
    # `ppsspp-start.sh`'s `env.sh` exports the private instance's HOME as `PP_HOME`; falling
    # back to the shell's own HOME put a `load --file` copy into the maintainer's memstick
    # (2026-10-21).
    ap.add_argument("--home", default=os.environ.get("PP_HOME") or os.environ.get("HOME"),
                    help="the instance's HOME (its memstick is under .config/ppsspp); "
                    "defaults to $PP_HOME, then $HOME")
    ap.add_argument("command", choices=["save", "load", "check", "pose"])
    ap.add_argument("slot", type=int, nargs="?", default=1)
    ap.add_argument("--keep", help="copy the saved state to this path (under data/saves/)")
    ap.add_argument("--serial", default="UCUS98712_1.00",
                    help="disc serial and version in the state file name (Pulse USA by default)")
    ap.add_argument("--stop-at", type=lambda v: int(v, 0), default=None, metavar="ADDR",
                    help="for load: stop at the first hit of this address (the state's own first frame, "
                    "deterministic run to run) and leave the CPU stopped; 0x08849618 is Ship_UpdateCraft")
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
                target = state_dir(args.home) / ("%s_%d.ppst" % (args.serial, args.slot - 1))
                target.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(args.file, target)
                print("placed %s -> %s" % (args.file, target))
                # The menu lists the slot only after a reopen; do_load reopens it.
            if args.stop_at is not None:
                began = time.time()
                do_load_stopped(dbg, args.slot, args.stop_at)
                took = time.time() - began
                print("loaded slot %d and stopped at %#x (race clock %.4f) in %.1f s" % (
                    args.slot, args.stop_at, dbg.read_f32(0x08B344B0), took))
            else:
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
