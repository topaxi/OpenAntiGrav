#!/usr/bin/env python3
"""Screenshot every front-end screen from boot to the race box's last page.

`psp-drive.py menu` walks straight through the front end into a race and
`psp-racebox.py` stops only at the settings page and the HUD. This one exists
to *look* at the screens in between: it walks from wherever the emulator is
to `Team Selection`, and on every screen it reaches it writes a screenshot
named after the state the front end announces (`0x08b31784+0x18c`, see
`ppsspp-debugger.md`), then backs out without ever launching a race.

    uv run --with websocket-client scripts/psp-frontend-capture.py \\
        --out data/cache/fe-capture

The output directory is under `data/`, which is gitignored: these are frames
of the original and are never committed. Describe what they show in `docs/`
with measurements, the way `docs/ui/menus-original.md` does.

Needs a PPSSPP SDL build with its debugger up and an X display `import`
(ImageMagick) can grab - `docs/reverse-engineering/ppsspp-debugger.md` has
the Xvfb recipe. The default `--display :97` is this project's throwaway
Xvfb convention.

What it captures, in order:

- every boot state it passes through (`Show Logo`, `LogoFMV`, `Language
  Selection`, the first-boot profile screens if the memory stick is fresh);
- `Main Menu`, once per row so every row's own help text is seen;
- `Racebox`, once per row;
- `Single Player` (Custom Race), once per row, plus every value of the
  RACE TYPE selector;
- `Track Creation`, twice per circuit a second apart (the preview animates)
  and the `Track Help` overlay;
- `Team Selection`, twice per team a second apart, the `Team Help` overlay
  and the `Pre Race Music Select` overlay;
- the Race Campaign leg, from `Main Menu` back to itself: `Grid Selection`
  (page 1, then paged to the last page of locked "Phantom Grid" tiers),
  `Cell Selection` on the default cursor, and `Cell Help`.

**`TournamentLoad`'s autosave dialog never fires on this walk, fresh profile
or not.** A confirm on `Main Menu`'s own default cursor (`RACE CAMPAIGN`, row
1 - no `down` press needed) goes straight to `Grid Selection`, measured with
a 50 ms poll on a memory stick that had just answered every first-boot
dialog for the first time (`RemoveMemoryStickWarning` through
`Language Selection`, confirmed fresh). `docs/formats/race-setup.md`'s
original reading of the dialog (behind `MSC_SQ_MSG7`) is corroborated
elsewhere in this project's own docs as tied to `Race_RecordResult`'s dirty
flag, i.e. it fires on *returning* from a race that changed the profile, not
on first entry - consistent with never seeing it here, since this walk never
races. The campaign leg below does not wait for or answer it.

`--skip-racebox` runs only the campaign leg, for faster iteration once the
racebox/track/team screens above are already captured.
"""

import argparse
import importlib.util
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
from ppsspp_debugger import Debugger  # noqa: E402

# `psp-drive` is not a Python identifier, so it is loaded off its file path.
_spec = importlib.util.spec_from_file_location("psp_drive", Path(__file__).with_name("psp-drive.py"))
psp_drive = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(psp_drive)

FIRST_BOOT = psp_drive.FIRST_BOOT
MAIN_MENU = psp_drive.MAIN_MENU
RACEBOX = psp_drive.RACEBOX
CUSTOM_RACE = psp_drive.CUSTOM_RACE
TRACK_SELECT = psp_drive.TRACK_SELECT
TEAM_SELECT = "Team Selection"
GRID_SELECT = "Grid Selection"
CELL_SELECT = "Cell Selection"
CELL_HELP = "Cell Help"
IN_GAME = psp_drive.IN_GAME
DEMO_PREFIX = psp_drive.DEMO_PREFIX
SATURATE = psp_drive.SATURATE
tap = psp_drive.tap
named = psp_drive.named

# Row counts read off the disc's own definitions (fe-menu-definitions.md,
# race-setup.md): seven rows on Main Menu, four on Racebox, five selectors
# plus the launch row on Single Player. The walk moves `down` one fewer time
# than there are rows and returns with `up`.
MAIN_MENU_ROWS = 7
RACEBOX_ROWS = 4
CUSTOM_RACE_ROWS = 6
RACE_TYPES = 7
TRACKS = 3
TEAMS = 8

# Sixteen grids, four tiers per page (`docs/ui/campaign-screens.md`'s honey
# counter reading) - the last page holds `grid12`..`grid15`, the four whose
# idstring reads "Phantom Grid N" rather than "Grid N".
GRID_PAGES = 4


class Shots:
    def __init__(self, out, display):
        self.out = Path(out)
        self.out.mkdir(parents=True, exist_ok=True)
        self.display = display
        self.index = 0

    def take(self, label):
        self.index += 1
        safe = "".join(c if c.isalnum() else "-" for c in label).strip("-").lower()
        path = self.out / ("%02d-%s.png" % (self.index, safe))
        subprocess.run(["import", "-window", "root", str(path)], env={"DISPLAY": self.display}, check=True)
        print("shot %s" % path)
        return path


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


def boot(dbg, shots, timeout):
    """Screenshot every state from now until `Main Menu`, answering each one."""
    end = time.time() + timeout
    last = None
    while time.time() < end:
        state = dbg.state_name()
        if state != last and state:
            time.sleep(1.0)
            shots.take(state)
            last = state
        if named(state, MAIN_MENU):
            return
        if state and state.startswith(DEMO_PREFIX):
            tap(dbg, "cross", 1, wait=1.2)
            continue
        if state and IN_GAME in state:
            print("in a race (%r) - back out first" % state, file=sys.stderr)
            raise SystemExit(1)
        keys = next((k for name, k in FIRST_BOOT if state == name), None)
        if keys is not None:
            for button, times in keys:
                tap(dbg, button, times)
            time.sleep(1.5)
        elif state and not named(state, MAIN_MENU):
            tap(dbg, "circle", 1, wait=1.5)
        else:
            time.sleep(0.5)
    print("never reached %r, last saw %r" % (MAIN_MENU, last), file=sys.stderr)
    raise SystemExit(1)


def each_row(dbg, shots, label, rows, settle=0.6):
    for row in range(rows):
        time.sleep(settle)
        shots.take("%s-row%d" % (label, row + 1))
        if row + 1 < rows:
            tap(dbg, "down")
    tap(dbg, "up", rows - 1, wait=0.25)


def campaign_leg(dbg, shots):
    """The Race Campaign, from wherever the emulator is back to itself.

    Confirms `Main Menu`'s own default cursor (`RACE CAMPAIGN`, row 1 - see
    this module's own doc for why no dialog needs answering on the way in),
    captures `Grid Selection` on its first page and its last (the locked
    "Phantom Grid" tiers), opens `Cell Selection` on the default cursor,
    opens `Cell Help`, then backs out to `Main Menu` the same way the
    walk above does.
    """
    state = dbg.state_name()
    if not named(state, MAIN_MENU):
        print("campaign leg expects Main Menu, saw %r" % state, file=sys.stderr)
        raise SystemExit(1)
    tap(dbg, "cross", 1, wait=1.5)
    expect(dbg, GRID_SELECT, "confirming RACE CAMPAIGN")
    shots.take("grid-selection-page1")

    for page in range(2, GRID_PAGES + 1):
        tap(dbg, "down", 1, wait=1.0)
        shots.take("grid-selection-page%d" % page)

    tap(dbg, "up", GRID_PAGES - 1, wait=0.5)  # back to page 1 / grid0
    expect(dbg, GRID_SELECT, "paging back to grid0")

    tap(dbg, "cross", 1, wait=1.5)
    expect(dbg, CELL_SELECT, "confirming grid0")
    shots.take("cell-selection-default")

    tap(dbg, "triangle", 1, wait=1.2)
    expect(dbg, CELL_HELP, "opening Cell Help")
    shots.take("cell-help")

    tap(dbg, "circle", 1, wait=1.2)
    expect(dbg, CELL_SELECT, "closing Cell Help")
    tap(dbg, "circle", 1, wait=1.2)
    expect(dbg, GRID_SELECT, "backing out of Cell Selection")
    tap(dbg, "circle", 1, wait=1.2)
    for _ in range(5):
        if named(dbg.state_name(), MAIN_MENU):
            break
        tap(dbg, "circle", 1, wait=1.2)
    shots.take("campaign-back-at-main-menu")


def main():
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("--port", type=int, default=47810)
    ap.add_argument("--display", default=":97")
    ap.add_argument("--out", default="data/cache/fe-capture")
    ap.add_argument("--boot-timeout", type=float, default=240.0)
    ap.add_argument(
        "--skip-racebox",
        action="store_true",
        help="only the campaign leg, for iterating on it without replaying racebox/track/team",
    )
    args = ap.parse_args()

    shots = Shots(args.out, args.display)
    dbg = Debugger(args.port)
    dbg.resume()

    boot(dbg, shots, args.boot_timeout)
    print("at %r" % MAIN_MENU, file=sys.stderr)

    if args.skip_racebox:
        campaign_leg(dbg, shots)
        dbg.close()
        return

    each_row(dbg, shots, "main-menu", MAIN_MENU_ROWS)

    tap(dbg, "down")  # RACE CAMPAIGN -> RACEBOX
    tap(dbg, "cross", 1, wait=2.5)
    expect(dbg, RACEBOX, "entering Racebox")
    each_row(dbg, shots, "racebox", RACEBOX_ROWS)

    tap(dbg, "cross", 1, wait=3.0)  # CUSTOM RACE
    expect(dbg, CUSTOM_RACE, "entering Custom Race")
    tap(dbg, "left", SATURATE, wait=0.3)  # RACE TYPE -> SINGLE RACE
    for i in range(RACE_TYPES):
        time.sleep(0.5)
        shots.take("custom-race-type%d" % i)
        if i + 1 < RACE_TYPES:
            tap(dbg, "right")
    tap(dbg, "left", SATURATE, wait=0.3)  # back to SINGLE RACE
    each_row(dbg, shots, "custom-race", CUSTOM_RACE_ROWS)

    tap(dbg, "cross", 1, wait=3.0)
    expect(dbg, TRACK_SELECT, "confirming the race settings")
    for i in range(TRACKS):
        time.sleep(1.0)
        shots.take("track-creation-%d-a" % (i + 1))
        time.sleep(1.0)
        shots.take("track-creation-%d-b" % (i + 1))
        tap(dbg, "down", 1, wait=1.5)
    tap(dbg, "triangle", 1, wait=1.5)
    shots.take("track-help")
    tap(dbg, "circle", 1, wait=1.5)
    expect(dbg, TRACK_SELECT, "closing Track Help")

    tap(dbg, "cross", 1, wait=3.0)
    expect(dbg, TEAM_SELECT, "confirming the track")
    for i in range(TEAMS):
        time.sleep(1.0)
        shots.take("team-selection-%d-a" % (i + 1))
        time.sleep(1.0)
        shots.take("team-selection-%d-b" % (i + 1))
        tap(dbg, "down", 1, wait=1.5)
    tap(dbg, "right", 1, wait=1.0)
    shots.take("team-selection-livery-right")
    tap(dbg, "left", 1, wait=1.0)
    tap(dbg, "triangle", 1, wait=1.5)
    shots.take("team-help")
    tap(dbg, "circle", 1, wait=1.5)
    expect(dbg, TEAM_SELECT, "closing Team Help")
    tap(dbg, "square", 1, wait=1.5)
    shots.take("pre-race-music-select")
    tap(dbg, "circle", 1, wait=1.5)
    expect(dbg, TEAM_SELECT, "closing the music select")

    for _ in range(6):
        if named(dbg.state_name(), MAIN_MENU):
            break
        tap(dbg, "circle", 1, wait=1.5)
    shots.take("back-at-main-menu")

    campaign_leg(dbg, shots)
    dbg.close()


if __name__ == "__main__":
    main()
