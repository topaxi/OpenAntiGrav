#!/usr/bin/env python3
"""Walk Custom Race with a chosen RACE TYPE and SPEED CLASS, and screenshot both
the settings screen and the in-race HUD.

`psp-drive.py menu` hard-codes TIME TRIAL / VENOM - the one reachable
configuration its own reference scenarios need. Checking a lap count across
race types and speed classes (see `docs/gameplay/race-modes.md`) needs every
other cell of that grid, and needs a screenshot of the RACEBOX screen itself
before confirming - the selector presses that reach FLASH or PHANTOM are
inferred from `SpeedClass::ALL`'s order, and the only thing that proves they
landed where intended is a frame of the screen they were sent to. Skipping
that frame is how a stuck selector reads as a class it never actually reached.

    uv run --with websocket-client scripts/psp-racebox.py \\
        --race-type time_trial --class phantom --confirm --shot-prefix /tmp/phantom

Requires a PPSSPP debugger already reachable (see `psp-drive.py preflight`)
and a compositor or Xvfb `DISPLAY` `import` (ImageMagick) can screenshot -
see `docs/reverse-engineering/ppsspp-debugger.md`'s Xvfb recipe. `--display`
defaults to `:97`, this project's own convention for a throwaway Xvfb.
"""

import argparse
import subprocess
import sys
import time

sys.path.insert(0, "scripts")
from ppsspp_debugger import Debugger  # noqa: E402

MAIN_MENU = "Main Menu"
RACEBOX = "Racebox"
CUSTOM_RACE = "Single Player"
TRACK_SELECT = "Track Creation"
IN_GAME = "InGame"
DEMO_PREFIX = "Demo"

# The Custom Race selectors clamp rather than wrap - see psp-drive.py's own
# SATURATE for the measurement (RACE TYPE sat on TOURNAMENT and read SINGLE
# RACE after eight lefts).
SATURATE = 8

# RACE TYPE's own list, re-measured in psp-drive.py: 0 SINGLE RACE,
# 1 HEAD TO HEAD, 2 TIME TRIAL, 3 SPEED LAP, 4 TOURNAMENT, 5 ZONE,
# 6 ELIMINATOR. `right` presses from the saturated (leftmost) entry.
RACE_TYPES = {
    "single_race": 0,
    "head2head": 1,
    "time_trial": 2,
    "speed_lap": 3,
    "tournament": 4,
    "zone": 5,
    "eliminator": 6,
}

# SPEED CLASS, slowest first - the same order `oag_tables::handling::SpeedClass::ALL`
# holds. `right` presses from the saturated (leftmost, Venom) entry.
CLASSES = {"venom": 0, "flash": 1, "rapier": 2, "phantom": 3}

# Real time to sit through the track description and the countdown before the
# in-race screenshot - see psp-drive.py's RESTART_TO_DESCRIPTION/
# DESCRIPTION_TO_GREEN for the measured figures this approximates. Rougher
# here because the target is "the HUD is up", not a stationary craft.
DESCRIPTION_DELAY = 20.0
COUNTDOWN_SAMPLE_DELAY = 10.0


def tap(dbg, button, times=1, wait=0.45):
    for _ in range(times):
        dbg.press(button, duration=6)
        time.sleep(wait)


def named(seen, wanted):
    return seen is not None and seen.lower() == wanted.lower()


def expect(dbg, wanted, what, timeout=15.0):
    end = time.time() + timeout
    seen = None
    while time.time() < end:
        seen = dbg.state_name()
        if named(seen, wanted):
            return
        time.sleep(0.4)
    print("expected %r (%s), saw %r" % (wanted, what, seen), file=sys.stderr)
    raise SystemExit(1)


def screenshot(path, display):
    subprocess.run(["import", "-window", "root", path], env={"DISPLAY": display}, check=True)
    print("screenshot: %s" % path)


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, default=47810)
    ap.add_argument("--display", default=":97", help="X display to screenshot, e.g. Xvfb's")
    ap.add_argument("--race-type", choices=sorted(RACE_TYPES), default="time_trial")
    ap.add_argument("--class", dest="cls", choices=sorted(CLASSES), default="venom")
    ap.add_argument("--shot-prefix", default="/tmp/racebox")
    ap.add_argument(
        "--confirm",
        action="store_true",
        help="launch the race and screenshot the in-race HUD too, not just the settings screen",
    )
    args = ap.parse_args()

    dbg = Debugger(args.port)
    dbg.resume()

    state = dbg.state_name()
    if state and IN_GAME in state and not state.startswith(DEMO_PREFIX):
        print("already in a race (%r) - back out first" % state, file=sys.stderr)
        raise SystemExit(1)

    for _ in range(12):
        if named(dbg.state_name(), MAIN_MENU):
            break
        tap(dbg, "circle", 1, wait=1.5)
    expect(dbg, MAIN_MENU, "backing out")
    print("at Main Menu", file=sys.stderr)

    tap(dbg, "down")  # RACE CAMPAIGN -> RACEBOX
    tap(dbg, "cross", 1, wait=2.5)
    expect(dbg, RACEBOX, "entering Racebox")

    tap(dbg, "cross", 1, wait=3.0)  # CUSTOM RACE, the first entry
    expect(dbg, CUSTOM_RACE, "entering Custom Race")

    tap(dbg, "left", SATURATE, wait=0.3)  # RACE TYPE -> SINGLE RACE
    presses = RACE_TYPES[args.race_type]
    if presses:
        tap(dbg, "right", presses, wait=0.4)
    tap(dbg, "down")  # SPEED CLASS
    tap(dbg, "left", SATURATE, wait=0.3)  # -> VENOM
    presses = CLASSES[args.cls]
    if presses:
        tap(dbg, "right", presses, wait=0.4)
    tap(dbg, "up")
    time.sleep(0.5)
    screenshot(args.shot_prefix + "-settings.png", args.display)
    print("set %s / %s" % (args.race_type.upper(), args.cls.upper()), file=sys.stderr)

    if not args.confirm:
        dbg.close()
        return

    tap(dbg, "cross", 1, wait=3.0)
    expect(dbg, TRACK_SELECT, "confirming settings")
    tap(dbg, "cross", 1, wait=3.0)  # the track, whichever is selected
    tap(dbg, "cross", 1, wait=3.0)  # the ship
    expect(dbg, IN_GAME, "confirming ship", timeout=30.0)
    print("loading the race", file=sys.stderr)
    time.sleep(DESCRIPTION_DELAY)
    tap(dbg, "cross", 1, wait=0.1)
    print("dismissed track description", file=sys.stderr)
    time.sleep(COUNTDOWN_SAMPLE_DELAY)
    screenshot(args.shot_prefix + "-ingame.png", args.display)
    dbg.close()


if __name__ == "__main__":
    main()
