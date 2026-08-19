#!/usr/bin/env python3
"""Boot WipEout HD in RPCS3, walk it into a race, and drive - with no window.

This is the PS3 counterpart of `scripts/psp-drive.py`, and it is shaped
differently because RPCS3 gives and withholds different things than PPSSPP does:

- **There is no input API**, so a button press is a real kernel device -
  `scripts/rpcs3_pad.py`, and its preflight is why a missing permission reads as
  a diagnosis instead of a `PermissionError`.
- **There is no state API worth pacing on** - the GDB stub answers nothing at
  all while the target runs (`scripts/rpcs3_debugger.py`). What replaces it is
  the game's own `printf`: RPCS3 writes it to `~/.cache/rpcs3/TTY.log`, and HD
  logs `Switching Screen "A" to "B"` on every front-end transition. That single
  line is the whole state machine, free, and it is what every wait here keys on.

**`--headless` cannot reach the front end, and the way it fails is a trap.** The
null renderer boots, prints as far as `TROPHY: Checking free disk space`, and
then stops for good with the main thread parked inside `cellGameDataCheck` -
looking exactly like a game waiting for a button, which is what cost a session
here. It is not: presses land and change nothing, because the game never got to
a screen. A *real* renderer on a virtual display reaches `Main Menu` in about 45
seconds, and from there six taps of cross start a race on Talon's Junction.

So the display is not optional and is also not the user's desktop:

    python3 scripts/rpcs3-drive.py display          # start Xvfb :77
    uv run --with evdev python3 scripts/rpcs3-drive.py race --drive 20 --shots

Xvfb is started with `-listen tcp -nolisten unix` and addressed as
`127.0.0.1:77` deliberately: a sandboxed session may not be able to write
`/tmp/.X11-unix`, and then the unix socket never appears while the TCP port
does. `QT_QPA_PLATFORM=xcb` and dropping `WAYLAND_DISPLAY` are needed for the
same reason - RPCS3 is Qt, and on a Wayland session it will not look at an X
display unless told to.

Subcommands:

    display   start (or check) the virtual display everything else needs.
    preflight the pad, the input profile and the display, with each fix.
    boot      launch and wait for the Main Menu, then hold it there.
    race      boot, walk the menus into a race, optionally drive and screenshot.
    record    the same, capturing the driven part as video.

**Video is the observable a race actually has.** `TTY.log` goes quiet the moment
the front end hands over, and the GDB stub answers nothing while the target
runs, so a driven lap has no per-tick channel at all - but RPCS3's own overlay
records one, and HD's HUD puts speed, position, lap number and lap time in the
frame. `record` drives that: start, drive, stop, and the file lands in
`~/.config/rpcs3/recordings/<TITLE_ID>/`.

Needs `evdev` for anything that presses a button; run those through
`uv run --with evdev`. See docs/reverse-engineering/rpcs3-debugger.md.
"""

import argparse
import glob
import os
import re
import subprocess
import sys
import time
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import rpcs3_pad

TTY = os.path.expanduser("~/.cache/rpcs3/TTY.log")
DISPLAY_NUMBER = 77
DISPLAY = "127.0.0.1:%d" % DISPLAY_NUMBER
# Taller than 720p on purpose: RPCS3's own home-menu overlay is nine rows and
# does not scroll, so at 1280x720 the last three - `SaveState` among them - are
# simply not on screen, and the highlight walking off the bottom reads exactly
# like a dead d-pad.
DISPLAY_GEOMETRY = "1600x1200x24"
SCREEN_LINE = re.compile(r'Switching Screen "(.*?)" to "(.*?)"')

# RPCS3's *own* home menu, opened by the PS button - not the game's. Measured
# 2026-08-19 at 1600x1200; the list wraps, and at 720p only the first six rows
# are visible with no scrolling, which reads exactly like a dead d-pad. Reached
# from the top by `up`, since counting backwards from `Exit Game` is both fewer
# presses and further from selecting it by accident.
HOME_MENU = [
    "Resume Game", "Settings", "Trophies", "Take Screenshot",
    "Start/Stop Recording", "Toggle Fullscreen", "SaveState",
    "Restart Game", "Exit Game",
]

# Measured: every step is the default highlighted row, so the walk into a race
# needs no d-pad at all. It is written as the *screens* rather than as six
# presses because a press is genuinely dropped now and then - HD runs at about
# 9 fps here, and a `cross` that lands mid-transition does nothing. Keying on
# `TTY.log` and re-pressing is the difference between a walk that works and one
# that ends on `Team Selection` about a third of the time.
RACE_WALK = [
    "Campaign Selection",
    "Grid Selection Fury",
    "Cell Selection",
    "Team Selection",
    "Launch Game",
    "InGame",
]

# Where the walk is finished. `Launch Game` is *transient* - it can come and go
# between two polls - so nothing here waits for a named screen: the walk presses
# until the screen changes at all and stops when it recognises one of these.
# Waiting for `Launch Game` by name overshoots straight past `InGame` into
# `HUD`, which is what the first version of this did.
RACE_ARRIVED = {"InGame", "HUD"}


def tty_text():
    try:
        with open(TTY, errors="replace") as handle:
            return handle.read()
    except FileNotFoundError:
        return ""


def current_screen():
    """The last screen HD said it switched to, or `?` before the first one."""
    hits = SCREEN_LINE.findall(tty_text())
    return hits[-1][1] if hits else "?"


def display_running():
    probe = subprocess.run(
        ["python3", "-c",
         "import socket,sys;"
         "s=socket.socket();s.settimeout(2);"
         "sys.exit(0 if s.connect_ex(('127.0.0.1', %d)) == 0 else 1)"
         % (6000 + DISPLAY_NUMBER)],
        capture_output=True)
    return probe.returncode == 0


def start_display():
    if display_running():
        return False
    subprocess.Popen(
        ["Xvfb", ":%d" % DISPLAY_NUMBER, "-screen", "0", DISPLAY_GEOMETRY,
         "-listen", "tcp", "-nolisten", "unix"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        start_new_session=True)
    for _ in range(20):
        time.sleep(0.5)
        if display_running():
            return True
    raise RuntimeError("Xvfb :%d did not come up" % DISPLAY_NUMBER)


def recordings(title_id="BCES00664"):
    """Every recording RPCS3 has written for a title, oldest first.

    The path is `recordings/<TITLE_ID>/*.mp4` - a *subdirectory* per title,
    which is worth stating because globbing `recordings/*` finds nothing and
    reads as "recording silently did not work".
    """
    root = os.environ.get("XDG_CONFIG_HOME") or os.path.expanduser("~/.config")
    pattern = os.path.join(root, "rpcs3", "recordings", title_id, "*.mp4")
    return sorted(glob.glob(pattern), key=os.path.getmtime)


def emulator_env():
    env = dict(os.environ)
    env["DISPLAY"] = DISPLAY
    env["QT_QPA_PLATFORM"] = "xcb"
    env.pop("WAYLAND_DISPLAY", None)
    return env


def screenshot(path):
    """Grab the virtual root window. Returns the path, or None if it failed."""
    result = subprocess.run(
        ["import", "-display", DISPLAY, "-window", "root", str(path)],
        capture_output=True)
    if result.returncode != 0:
        print("screenshot failed: %s" % result.stderr.decode()[:160],
              file=sys.stderr)
        return None
    return path


class Session:
    """One emulator run, with the pad it must not outlive."""

    def __init__(self, image, log_dir):
        # Resolved here rather than passed through: `data/` is gitignored and
        # does not travel into a worktree, so a relative default silently
        # becomes a path RPCS3 answers `Invalid file or folder` for.
        self.image = Path(image).resolve()
        if not self.image.exists():
            raise SystemExit(
                "no such image: %s\n"
                "  `data/` is gitignored and absent from a worktree - run this "
                "from the main checkout, or pass --image with an absolute path"
                % self.image)
        self.log_dir = Path(log_dir)
        self.log_dir.mkdir(parents=True, exist_ok=True)
        self.pad = None
        self.proc = None

    def __enter__(self):
        start_display()
        # The pad has to exist before RPCS3 does: it binds pads when it
        # enumerates devices and does not rescan.
        self.pad = rpcs3_pad.Pad()
        open(TTY, "w").close()
        self.proc = subprocess.Popen(
            ["rpcs3", "--no-gui", "--input-config", rpcs3_pad.INPUT_CONFIG_NAME,
             str(self.image)],
            stdout=open(self.log_dir / "rpcs3.log", "w"),
            stderr=subprocess.STDOUT, start_new_session=True,
            env=emulator_env())
        return self

    def __exit__(self, *_):
        if self.proc is not None:
            self.proc.terminate()
        if self.pad is not None:
            self.pad.close()

    def wait_for_screen(self, name, timeout=180.0):
        deadline = time.time() + timeout
        while time.time() < deadline:
            if current_screen() == name:
                return True
            if self.proc.poll() is not None:
                raise RuntimeError("RPCS3 exited before reaching %r" % name)
            time.sleep(2)
        return False

    def tap(self, button, settle=4.0):
        was = current_screen()
        self.pad.press(button, 0.15)
        time.sleep(settle)
        return was, current_screen()

    def home_menu_select(self, item, settle=1.0):
        """Open RPCS3's overlay and highlight `item`, without confirming it.

        Only the `SaveState` path has been walked end to end; the rest is the
        same arithmetic on a list that was read off a screenshot, so treat a
        first use of another row as unverified.
        """
        if item not in HOME_MENU:
            raise KeyError("no such home-menu item: %r" % item)
        self.pad.press("ps", 0.30)
        time.sleep(4.0)
        for _ in range(len(HOME_MENU) - HOME_MENU.index(item)):
            self.pad.press("up", 0.12)
            time.sleep(settle)

    def toggle_recording(self):
        """Start or stop RPCS3's own capture. The overlay item is a toggle."""
        self.home_menu_select("Start/Stop Recording")
        self.pad.press("cross", 0.15)
        time.sleep(3.0)

    def press_once(self, button, settle=5.0):
        """Press, then wait for the screen to change. Returns (was, now)."""
        was = current_screen()
        self.pad.press(button, 0.15)
        deadline = time.time() + settle
        while time.time() < deadline and current_screen() == was:
            time.sleep(0.3)
        return was, current_screen()

    def walk_to_race(self, max_presses=12):
        """Press cross until HD is in a race, or give up and say where it got to.

        Deliberately not a fixed count: HD runs at about 9 fps here and a press
        landing mid-transition does nothing, so `RACE_WALK` is what the path was
        *measured* to be and this loop is what survives a dropped press.
        """
        for index in range(1, max_presses + 1):
            if current_screen() in RACE_ARRIVED:
                break
            was, now = self.press_once("cross")
            print("  press %2d  %-22s -> %-22s%s"
                  % (index, was, now, "" if now != was else "   (dropped)"),
                  flush=True)
        return current_screen()


def cmd_display(args):
    started = start_display()
    print("Xvfb :%d %s; address it as DISPLAY=%s"
          % (DISPLAY_NUMBER, "started" if started else "already running", DISPLAY))
    return 0


def cmd_preflight(args):
    ok = rpcs3_pad.report(stream=sys.stdout)
    if display_running():
        print("display OK: Xvfb is listening on %s" % DISPLAY)
    else:
        print("\n  no virtual display on %s - RPCS3 needs a real renderer to "
              "get past its trophy check, and --headless never will" % DISPLAY)
        print("\n      python3 scripts/rpcs3-drive.py display")
        ok = False
    return 0 if ok else 1


def cmd_boot(args):
    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d, waiting for the Main Menu" % session.proc.pid,
              flush=True)
        if not session.wait_for_screen("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        print("Main Menu. Holding for %g s." % args.hold, flush=True)
        time.sleep(args.hold)
    return 0


def cmd_race(args):
    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        print("Main Menu; settling", flush=True)
        time.sleep(args.settle)
        if args.shots:
            screenshot(session.log_dir / "01-menu.png")
        print("walking the menus:", flush=True)
        screen = session.walk_to_race()
        if screen not in RACE_ARRIVED:
            print("ended on %r rather than in a race" % screen, file=sys.stderr)
            return 1
        print("%s; waiting %g s for the track to load"
              % (current_screen(), args.load), flush=True)
        time.sleep(args.load)
        if args.shots:
            screenshot(session.log_dir / "02-grid.png")
        if args.drive:
            print("holding thrust for %g s" % args.drive, flush=True)
            session.pad.set("cross", True)
            time.sleep(args.drive)
            session.pad.set("cross", False)
            if args.shots:
                screenshot(session.log_dir / "03-driven.png")
        print("done; artefacts in %s" % session.log_dir, flush=True)
    return 0


def cmd_record(args):
    before = set(recordings())
    with Session(args.image, args.log_dir) as session:
        print("rpcs3 pid %d" % session.proc.pid, flush=True)
        if not session.wait_for_screen("Main Menu", args.timeout):
            print("never reached the Main Menu (last screen: %s)"
                  % current_screen(), file=sys.stderr)
            return 1
        time.sleep(args.settle)
        print("walking the menus:", flush=True)
        if session.walk_to_race() not in RACE_ARRIVED:
            print("ended on %r rather than in a race" % current_screen(),
                  file=sys.stderr)
            return 1
        print("%s; waiting %g s for the track to load"
              % (current_screen(), args.load), flush=True)
        time.sleep(args.load)
        print("starting the recording", flush=True)
        session.toggle_recording()
        print("driving %g s" % args.drive, flush=True)
        session.pad.set("cross", True)
        time.sleep(args.drive)
        session.pad.set("cross", False)
        print("stopping the recording", flush=True)
        session.toggle_recording()
        time.sleep(8.0)
    fresh = [path for path in recordings() if path not in before]
    if not fresh:
        print("no new recording appeared - check RPCS3.log for `video_encoder`",
              file=sys.stderr)
        return 1
    for path in fresh:
        print("%s  %.1f MiB" % (path, os.path.getsize(path) / 1048576.0))
    return 0


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--image",
                        default="data/images/hdfury-ps3-eu-dec.iso",
                        help="the layer-1 DECRYPTED disc image")
    parser.add_argument("--log-dir", default="/tmp/rpcs3-drive",
                        help="where the emulator log and screenshots go")
    sub = parser.add_subparsers(dest="command", required=True)

    sub.add_parser("display").set_defaults(run=cmd_display)
    sub.add_parser("preflight").set_defaults(run=cmd_preflight)

    boot = sub.add_parser("boot")
    boot.add_argument("--timeout", type=float, default=180.0)
    boot.add_argument("--hold", type=float, default=30.0)
    boot.set_defaults(run=cmd_boot)

    race = sub.add_parser("race")
    race.add_argument("--timeout", type=float, default=180.0)
    race.add_argument("--settle", type=float, default=12.0,
                      help="pause after the Main Menu before the first press")
    race.add_argument("--load", type=float, default=50.0,
                      help="pause after 'InGame' for the track to load")
    race.add_argument("--drive", type=float, default=0.0,
                      help="seconds to hold thrust once the race is up")
    race.add_argument("--shots", action="store_true",
                      help="capture the menu, the grid and the driven frame")
    race.set_defaults(run=cmd_race)

    rec = sub.add_parser("record")
    rec.add_argument("--timeout", type=float, default=180.0)
    rec.add_argument("--settle", type=float, default=12.0)
    rec.add_argument("--load", type=float, default=50.0)
    rec.add_argument("--drive", type=float, default=30.0,
                     help="seconds of held thrust to capture")
    rec.set_defaults(run=cmd_record)

    args = parser.parse_args(argv)
    return args.run(args)


if __name__ == "__main__":
    sys.exit(main())
